mod common;

use axum::body::Body;
use axum::http::{Method, StatusCode};
use capsnap_server::util::token_hash;
use chrono::{TimeDelta, Utc};
use common::*;
use serde_json::json;

#[tokio::test]
async fn health_and_security_headers() {
    let w = world().await;
    let app = app(&w);
    let r = get(&app, "/api/v1/health", None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json(), json!({ "status": "ok" }));
    for (h, v) in [
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "no-referrer"),
        ("x-frame-options", "DENY"),
        ("cache-control", "no-store"),
        (
            "content-security-policy",
            "default-src 'none'; frame-ancestors 'none'",
        ),
    ] {
        assert_eq!(r.headers.get(h).unwrap(), v, "{h}");
    }
    let page = get(&app, "/g/", None).await;
    let csp = page
        .headers
        .get("content-security-policy")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        csp.contains("script-src 'self'")
            && csp.contains("connect-src 'self'")
            && !csp.contains("unsafe")
    );
}

#[tokio::test]
async fn sign_in_sessions_and_rate_limit() {
    let w = world().await;
    let app = app(&w);

    let r = json(
        &app,
        Method::POST,
        "/api/v1/auth/sessions",
        None,
        json!({
            "login": "  ADA@atelier ", "password": PASSWORD, "deviceLabel": "Pixel 9"
        }),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED);
    let body = r.json();
    let token = body["token"].as_str().unwrap().to_owned();
    assert_eq!(token.len(), 43);
    assert_eq!(body["staff"]["displayName"], "Ada");
    assert_eq!(body["staff"]["role"], "admin");
    assert_eq!(body["organization"]["name"], "Atelier No. 8");
    assert_eq!(
        body["locations"].as_array().unwrap().len(),
        2,
        "admins see every location"
    );

    // Stored: the token's hash and an Argon2id password hash, never the secrets.
    let (stored,): (String,) = sqlx::query_as("SELECT token_hash FROM device_sessions")
        .fetch_one(&w.state.pool)
        .await
        .unwrap();
    assert_eq!(stored, token_hash(&token));
    assert_ne!(stored, token);
    let (pw,): (String,) =
        sqlx::query_as("SELECT password_hash FROM staff WHERE login = 'ada@atelier'")
            .fetch_one(&w.state.pool)
            .await
            .unwrap();
    assert!(pw.starts_with("$argon2id$") && !pw.contains(PASSWORD));

    let chef = sign_in(&app, "chen@atelier").await;
    let me = get(&app, "/api/v1/me", Some(&chef)).await.json();
    assert_eq!(
        me["locations"].as_array().unwrap().len(),
        1,
        "a chef sees only assigned locations"
    );

    // Wrong password and unknown name look the same.
    let bad = |login: &'static str, password: &'static str| {
        let app = app.clone();
        async move {
            json(
                &app,
                Method::POST,
                "/api/v1/auth/sessions",
                None,
                json!({ "login": login, "password": password, "deviceLabel": "x" }),
            )
            .await
        }
    };
    let a = bad("ada@atelier", "wrong password!").await;
    let b = bad("nobody@nowhere", "wrong password!").await;
    assert_eq!((a.status, a.json()), (b.status, b.json()));
    assert_eq!(a.status, StatusCode::UNAUTHORIZED);

    // Five failures lock that sign-in name, even with the right password.
    for _ in 0..4 {
        bad("ada@atelier", "wrong password!").await;
    }
    let locked = bad("ada@atelier", PASSWORD).await;
    assert_eq!(locked.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(locked.error(), "rate_limited");
    assert_eq!(
        bad("chen@atelier", PASSWORD).await.status,
        StatusCode::CREATED,
        "other names unaffected"
    );

    // Sign-out revokes; bad or missing bearer tokens are refused.
    let r = call(
        &app,
        Method::DELETE,
        "/api/v1/auth/sessions/current",
        &[("authorization", &format!("Bearer {token}"))],
        Body::empty(),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(
        get(&app, "/api/v1/me", Some(&token)).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        get(&app, "/api/v1/me", Some("not-a-token")).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        get(&app, "/api/v1/me", None).await.status,
        StatusCode::UNAUTHORIZED
    );

    // Expired sessions stop working.
    let t = sign_in(&app, "bo@bistro").await;
    sqlx::query(
        "UPDATE device_sessions SET expires_at = '2020-01-01T00:00:00.000Z' WHERE token_hash = ?",
    )
    .bind(token_hash(&t))
    .execute(&w.state.pool)
    .await
    .unwrap();
    assert_eq!(
        get(&app, "/api/v1/me", Some(&t)).await.status,
        StatusCode::UNAUTHORIZED
    );

    // Device lost: revoking from the host ends every session of that person.
    let c2 = sign_in(&app, "chen@atelier").await;
    // Chen has three live sessions by now (two sign-ins above plus this one).
    assert_eq!(
        capsnap_server::admin::revoke_sessions(&w.state.pool, "chen@atelier")
            .await
            .unwrap(),
        3
    );
    assert_eq!(
        get(&app, "/api/v1/me", Some(&c2)).await.status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn tenants_and_locations_are_isolated() {
    let w = world().await;
    let app = app(&w);
    let chef = sign_in(&app, "chen@atelier").await;
    let bo = sign_in(&app, "bo@bistro").await;

    let menu = get(
        &app,
        &format!("/api/v1/locations/{}/menu-items", w.loc_a1),
        Some(&chef),
    )
    .await;
    assert_eq!(menu.status, StatusCode::OK);
    let names: Vec<_> = menu
        .json()
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(names, ["Saffron butter cod", "Pork belly bao"]);

    // Another restaurant's location, and a location the chef isn't assigned to.
    assert_eq!(
        get(
            &app,
            &format!("/api/v1/locations/{}/menu-items", w.loc_a1),
            Some(&bo)
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get(
            &app,
            &format!("/api/v1/locations/{}/menu-items", w.loc_a2),
            Some(&chef)
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );

    // Bo can't attach Atelier's photo, post to Atelier's location, or use Atelier's dish.
    let atelier_media = upload_ok(&app, &chef, phone_jpeg(64, 48, 1)).await;
    let bistro_media = upload_ok(&app, &bo, phone_jpeg(64, 48, 2)).await;
    let cod = menu_item(&app, &chef, &w.loc_a1, "Saffron butter cod").await;
    assert_eq!(
        capture(&app, &bo, "bo-capture-1", &atelier_media, &w.loc_b, None)
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        capture(&app, &bo, "bo-capture-2", &bistro_media, &w.loc_a1, None)
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        capture(
            &app,
            &bo,
            "bo-capture-3",
            &bistro_media,
            &w.loc_b,
            Some(&cod)
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        capture(
            &app,
            &chef,
            "chef-capture-1",
            &atelier_media,
            &w.loc_a2,
            None
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );

    // The same client id in two restaurants is two different captures.
    assert_eq!(
        capture(
            &app,
            &chef,
            "same-client-id",
            &atelier_media,
            &w.loc_a1,
            None
        )
        .await
        .status,
        StatusCode::CREATED
    );
    assert_eq!(
        capture(&app, &bo, "same-client-id", &bistro_media, &w.loc_b, None)
            .await
            .status,
        StatusCode::CREATED
    );
}

#[tokio::test]
async fn media_is_validated_before_it_is_kept() {
    let w = world().await;
    let app = app(&w);
    let t = sign_in(&app, "chen@atelier").await;

    let photo = phone_jpeg(400, 300, 7);
    let sha = sha256(&photo);
    let first = upload(&app, &t, photo.clone(), &sha, "image/jpeg").await;
    assert_eq!(first.status, StatusCode::CREATED);
    let again = upload(&app, &t, photo.clone(), &sha, "image/jpeg").await;
    assert_eq!(
        again.json()["mediaId"],
        first.json()["mediaId"],
        "same photo, same id"
    );

    let wrong = upload(&app, &t, photo.clone(), &"0".repeat(64), "image/jpeg").await;
    assert_eq!(
        (wrong.status, wrong.error().as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, "digest_mismatch")
    );
    assert_eq!(
        upload(&app, &t, photo.clone(), &sha, "image/png")
            .await
            .status,
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
    let no_digest = call(
        &app,
        Method::POST,
        "/api/v1/media",
        &[
            ("authorization", &format!("Bearer {t}")),
            ("content-type", "image/jpeg"),
        ],
        Body::from(photo.clone()),
    )
    .await;
    assert_eq!(no_digest.status, StatusCode::BAD_REQUEST);

    // Metadata still inside: an EXIF APP1 segment right after SOI.
    let mut exif = vec![0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x10];
    exif.extend_from_slice(b"Exif\0\0MM\0\x2a\0\0\0\x08");
    exif.extend_from_slice(&photo[2..]);
    let r = upload(&app, &t, exif.clone(), &sha256(&exif), "image/jpeg").await;
    assert_eq!(
        (r.status, r.error().as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, "metadata_present")
    );

    // Not resized on the phone.
    let big = raw_jpeg(3000, 100);
    let r = upload(&app, &t, big.clone(), &sha256(&big), "image/jpeg").await;
    assert_eq!(
        (r.status, r.error().as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, "too_large_dimensions")
    );

    // A JPEG signature on something that isn't a picture; and not a JPEG at all.
    let fake = [
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0, 1, 1, 0, 0, 1, 0, 1, 0, 0,
        0xFF, 0xD9,
    ]
    .to_vec();
    let r = upload(&app, &t, fake.clone(), &sha256(&fake), "image/jpeg").await;
    assert_eq!(
        (r.status, r.error().as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, "undecodable")
    );
    let png = b"\x89PNG\r\n\x1a\n0000".to_vec();
    assert_eq!(
        upload(&app, &t, png.clone(), &sha256(&png), "image/jpeg")
            .await
            .status,
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );

    // Size cap, enforced while streaming.
    let mut small = (*w.state.cfg).clone();
    small.max_media_bytes = 1000;
    let tight = capsnap_server::router(capsnap_server::AppState {
        cfg: std::sync::Arc::new(small),
        ..w.state.clone()
    });
    assert_eq!(
        upload(&tight, &t, photo.clone(), &sha, "image/jpeg")
            .await
            .status,
        StatusCode::PAYLOAD_TOO_LARGE
    );

    // Without a session nothing is accepted; rejected uploads leave no files behind.
    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/v1/media",
            &[("content-type", "image/jpeg")],
            Body::from(photo)
        )
        .await
        .status,
        StatusCode::UNAUTHORIZED
    );
    let leftovers = std::fs::read_dir(w.state.cfg.media_dir.join(".incoming"))
        .unwrap()
        .count();
    assert_eq!(leftovers, 0);
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM media_assets")
        .fetch_one(&w.state.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn captures_are_idempotent_and_issue_one_time_guest_links() {
    let w = world().await;
    let app = app(&w);
    let t = sign_in(&app, "chen@atelier").await;
    let media = upload_ok(&app, &t, phone_jpeg(320, 240, 3)).await;
    let cod = menu_item(&app, &t, &w.loc_a1, "Saffron butter cod").await;

    let first = capture(&app, &t, "3f1c2a9e-0001", &media, &w.loc_a1, Some(&cod)).await;
    assert_eq!(first.status, StatusCode::CREATED);
    let ack = first.json();
    let url1 = ack["guestUrl"].as_str().unwrap().to_owned();
    assert!(url1.starts_with("https://guests.example/g/#"), "{url1}");
    assert_eq!(token_from(&url1).len(), 43);

    // The phone never saw that answer and sends again: same capture, fresh link.
    let retry = capture(&app, &t, "3f1c2a9e-0001", &media, &w.loc_a1, Some(&cod)).await;
    assert_eq!(retry.status, StatusCode::OK);
    assert_eq!(retry.json()["captureId"], ack["captureId"]);
    let url2 = retry.json()["guestUrl"].as_str().unwrap().to_owned();
    assert_ne!(url1, url2);
    assert_eq!(
        guest_session(&app, &token_from(&url1)).await.status,
        StatusCode::NOT_FOUND,
        "old token retired"
    );
    let session = guest_session(&app, &token_from(&url2)).await;
    assert_eq!(session.status, StatusCode::OK);
    assert_eq!(session.json()["dishName"], "Saffron butter cod");
    assert_eq!(session.json()["locationName"], "Atelier No. 8");

    // Only hashes of guest tokens are stored.
    let (stored,): (String,) = sqlx::query_as("SELECT token_hash FROM guest_links")
        .fetch_one(&w.state.pool)
        .await
        .unwrap();
    assert_eq!(stored, token_hash(&token_from(&url2)));
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM captures")
        .fetch_one(&w.state.pool)
        .await
        .unwrap();
    assert_eq!(n, 1);

    // Reusing a client id for a different photo is refused.
    let other = upload_ok(&app, &t, phone_jpeg(320, 240, 4)).await;
    assert_eq!(
        capture(&app, &t, "3f1c2a9e-0001", &other, &w.loc_a1, None)
            .await
            .status,
        StatusCode::CONFLICT
    );
    // Malformed ids and future times are refused.
    assert_eq!(
        capture(&app, &t, "bad id!", &media, &w.loc_a1, None)
            .await
            .status,
        StatusCode::BAD_REQUEST
    );
    let future = json(&app, Method::POST, "/api/v1/captures", Some(&t), json!({
        "clientId": "3f1c2a9e-0002", "mediaId": media, "locationId": w.loc_a1, "menuItemId": null,
        "captureTimeUtc": "2099-01-01T00:00:00.000Z"
    }))
    .await;
    assert_eq!(future.status, StatusCode::BAD_REQUEST);
    // Tenant ids in the body are not accepted as fields at all.
    let sneaky = json(&app, Method::POST, "/api/v1/captures", Some(&t), json!({
        "clientId": "3f1c2a9e-0003", "mediaId": media, "locationId": w.loc_a1, "menuItemId": null,
        "captureTimeUtc": "2026-09-30T19:24:00.000Z", "orgId": "someone-else"
    }))
    .await;
    assert!(sneaky.status.is_client_error());
}

#[tokio::test]
async fn guest_feedback_is_neutral_bounded_and_one_time() {
    let w = world().await;
    let app = app(&w);
    let t = sign_in(&app, "chen@atelier").await;
    let media = upload_ok(&app, &t, phone_jpeg(320, 240, 5)).await;
    let ack = capture(&app, &t, "guest-flow-0001", &media, &w.loc_a1, None)
        .await
        .json();
    let token = token_from(ack["guestUrl"].as_str().unwrap());

    let session = guest_session(&app, &token).await;
    let set = session
        .headers
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    for part in [
        "HttpOnly",
        "SameSite=Strict",
        "Path=/api/v1/guest",
        "Secure",
        "Max-Age=1800",
    ] {
        assert!(set.contains(part), "{set} lacks {part}");
    }
    assert_eq!(session.json()["hasPhoto"], true);
    let c = cookie(&session);

    let photo = call(
        &app,
        Method::GET,
        "/api/v1/guest/photo",
        &[("cookie", &c)],
        Body::empty(),
    )
    .await;
    assert_eq!(photo.status, StatusCode::OK);
    assert_eq!(photo.headers.get("content-type").unwrap(), "image/jpeg");
    assert_eq!(
        call(&app, Method::GET, "/api/v1/guest/photo", &[], Body::empty())
            .await
            .status,
        StatusCode::NOT_FOUND
    );

    for bad in [
        json!({ "rating": 0 }),
        json!({ "rating": 6 }),
        json!({ "rating": 3, "comment": "x".repeat(2001) }),
    ] {
        assert_eq!(
            feedback(&app, &c, bad).await.status,
            StatusCode::BAD_REQUEST
        );
    }
    assert!(
        feedback(&app, &c, json!({ "rating": 3, "name": "Pat" }))
            .await
            .status
            .is_client_error(),
        "no other fields, e.g. a name"
    );
    // 2,000 characters are allowed even when they take more bytes.
    let r = feedback(
        &app,
        &c,
        json!({ "rating": 2, "comment": "é".repeat(2000) }),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED);
    assert!(
        r.headers
            .get("set-cookie")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("Max-Age=0")
    );

    // Once only: the session, the link and a retry of the capture all know it.
    assert_eq!(
        feedback(&app, &c, json!({ "rating": 5 })).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        guest_session(&app, &token).await.status,
        StatusCode::NOT_FOUND
    );
    let retry = capture(&app, &t, "guest-flow-0001", &media, &w.loc_a1, None)
        .await
        .json();
    assert!(retry["guestUrl"].is_null());
    let (rating, comment): (i64, String) =
        sqlx::query_as("SELECT rating, comment FROM guest_feedback")
            .fetch_one(&w.state.pool)
            .await
            .unwrap();
    assert_eq!((rating, comment.chars().count()), (2, 2000));

    // Unknown, malformed and expired links all get the same answer.
    let unknown = guest_session(&app, &"A".repeat(43)).await;
    let malformed = guest_session(&app, "short").await;
    assert_eq!(
        (unknown.status, unknown.json()),
        (malformed.status, malformed.json())
    );
    let media2 = upload_ok(&app, &t, phone_jpeg(320, 240, 6)).await;
    let ack2 = capture(&app, &t, "guest-flow-0002", &media2, &w.loc_a1, None)
        .await
        .json();
    sqlx::query(
        "UPDATE guest_links SET expires_at = '2020-01-01T00:00:00.000Z' WHERE used_at IS NULL",
    )
    .execute(&w.state.pool)
    .await
    .unwrap();
    let expired = guest_session(&app, &token_from(ack2["guestUrl"].as_str().unwrap())).await;
    assert_eq!(
        (expired.status, expired.json()),
        (unknown.status, unknown.json())
    );
}

#[tokio::test]
async fn retention_deletes_old_photos_and_feedback() {
    let w = world().await;
    let app = app(&w);
    let t = sign_in(&app, "chen@atelier").await;
    let media = upload_ok(&app, &t, phone_jpeg(320, 240, 9)).await;
    let ack = capture(&app, &t, "retention-0001", &media, &w.loc_a1, None)
        .await
        .json();
    let orphan = upload_ok(&app, &t, phone_jpeg(320, 240, 10)).await;
    let files = || {
        std::fs::read_dir(w.state.cfg.media_dir.join(w_org(&w)))
            .unwrap()
            .count()
    };
    assert_eq!(files(), 2);

    let now = Utc::now();
    let report = capsnap_server::retention::run(&w.state.pool, &w.state.cfg.media_dir, now)
        .await
        .unwrap();
    assert_eq!(report.photos_deleted, 0, "nothing is due yet");

    // 31 days later the synced photo goes; a day later the orphan went too.
    let later = now + TimeDelta::days(31);
    let report = capsnap_server::retention::run(&w.state.pool, &w.state.cfg.media_dir, later)
        .await
        .unwrap();
    assert_eq!(report.photos_deleted, 2);
    assert_eq!(files(), 0);
    let session = guest_session(&app, &token_from(ack["guestUrl"].as_str().unwrap())).await;
    assert_eq!(
        session.json()["hasPhoto"],
        false,
        "the link still works, without the photo"
    );
    let _ = orphan;

    // Feedback older than 12 months is deleted.
    let c = cookie(&session);
    assert_eq!(
        feedback(&app, &c, json!({ "rating": 4 })).await.status,
        StatusCode::CREATED
    );
    let in_13_months = now.checked_add_months(chrono::Months::new(13)).unwrap();
    let report =
        capsnap_server::retention::run(&w.state.pool, &w.state.cfg.media_dir, in_13_months)
            .await
            .unwrap();
    assert_eq!(report.feedback_deleted, 1);
}

fn w_org(w: &World) -> String {
    // The media folder is per organization; find Atelier's id.
    std::fs::read_dir(&w.state.cfg.media_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .find(|n| !n.starts_with('.'))
        .unwrap()
}

#[tokio::test]
async fn onboarding_import_needs_a_manager_and_records_consent() {
    let mut w = world().await;
    // Pin the test site to 127.0.0.1: the production IP policy must refuse it.
    w.state.onboard = std::sync::Arc::new(capsnap_onboard::Config {
        resolve_overrides: vec![(
            "atelier8.test".into(),
            std::net::IpAddr::from([127, 0, 0, 1]),
        )],
        ..capsnap_onboard::Config::default()
    });
    let app = app(&w);
    let chef = sign_in(&app, "chen@atelier").await;
    let admin = sign_in(&app, "ada@atelier").await;
    let body = |statement: &str| json!({ "siteUrl": "https://atelier8.test/", "acceptedStatement": statement });

    assert_eq!(
        json(
            &app,
            Method::POST,
            "/api/v1/onboarding/import",
            Some(&chef),
            body(capsnap_onboard::CONSENT_STATEMENT)
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        json(
            &app,
            Method::POST,
            "/api/v1/onboarding/import",
            Some(&admin),
            body("sure")
        )
        .await
        .status,
        StatusCode::BAD_REQUEST
    );
    let r = json(
        &app,
        Method::POST,
        "/api/v1/onboarding/import",
        Some(&admin),
        body(capsnap_onboard::CONSENT_STATEMENT),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        r.json()["message"]
            .as_str()
            .unwrap()
            .contains("isn't a public website"),
        "{}",
        r.json()
    );

    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT site_url, statement FROM onboarding_consents")
            .fetch_all(&w.state.pool)
            .await
            .unwrap();
    assert_eq!(
        rows,
        [(
            "https://atelier8.test/".to_owned(),
            capsnap_onboard::CONSENT_STATEMENT.to_owned()
        )]
    );
}
