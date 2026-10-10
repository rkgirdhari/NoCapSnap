//! The phone's sync client against the real server, over HTTP on 127.0.0.1.

use std::io::Cursor;
use std::path::PathBuf;

use capsnap_store::{CaptureDetails, DEMO_LOCATION_ID, LocalStore, Setting};
use capsnap_sync::{Server, SyncError, check_server_url, feedback_page, sign_in, sync_pending};

const PASSWORD: &str = "correct horse battery";

struct Rig {
    _dir: tempfile::TempDir,
    server: capsnap_server::AppState,
    base: String,
    location: String,
    phone: LocalStore,
    media: PathBuf,
}

async fn rig() -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = capsnap_server::AppState::open(capsnap_server::Config::for_dir(
        &dir.path().join("server"),
        &base,
    ))
    .await
    .unwrap();
    let pool = &server.pool;
    use capsnap_server::admin::*;
    create_org(pool, "atelier", "Atelier No. 8").await.unwrap();
    let location = create_location(pool, "atelier", "Atelier No. 8", "America/Chicago")
        .await
        .unwrap();
    create_staff(
        pool,
        "atelier",
        "maya@atelier",
        "Maya",
        "server",
        PASSWORD,
        std::slice::from_ref(&location),
    )
    .await
    .unwrap();
    import_menu(
        pool,
        &location,
        &[
            MenuEntry {
                name: "Saffron butter cod".into(),
                category: "Mains".into(),
            },
            MenuEntry {
                name: "Pork belly bao".into(),
                category: "Starters".into(),
            },
        ],
    )
    .await
    .unwrap();
    create_staff(
        pool,
        "atelier",
        "ada@atelier",
        "Ada",
        "manager",
        PASSWORD,
        &[],
    )
    .await
    .unwrap();
    tokio::spawn(capsnap_server::serve_listener(listener, server.clone()));

    let phone = LocalStore::open(&dir.path().join("phone.db"))
        .await
        .unwrap();
    phone.seed_demo_if_empty().await.unwrap(); // as the app does on first start
    Rig {
        media: dir.path().join("phone-media"),
        _dir: dir,
        server,
        base,
        location,
        phone,
    }
}

fn photo(seed: u8) -> Vec<u8> {
    let img = image::RgbImage::from_fn(640, 480, |x, y| {
        image::Rgb([(x as u8) ^ seed, y as u8, seed])
    });
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Jpeg).unwrap();
    out.into_inner()
}

async fn snap(
    r: &Rig,
    dish: Option<&str>,
    table: Option<&str>,
    seed: u8,
) -> capsnap_store::Capture {
    let dish_id = match dish {
        Some(name) => {
            let loc = r.phone.setting(Setting::LocationId).await.unwrap().unwrap();
            Some(
                r.phone
                    .menu_items(&loc)
                    .await
                    .unwrap()
                    .into_iter()
                    .find(|m| m.name == name)
                    .unwrap()
                    .id,
            )
        }
        None => None,
    };
    r.phone
        .ingest(
            &r.media,
            "local-device",
            CaptureDetails {
                menu_item_id: dish_id.as_deref(),
                table_label: table,
            },
            photo(seed),
        )
        .await
        .unwrap()
}

async fn server_count(r: &Rig, table: &str) -> i64 {
    let sql = match table {
        "captures" => "SELECT COUNT(*) FROM captures",
        "media_assets" => "SELECT COUNT(*) FROM media_assets",
        other => panic!("no count query for {other}"),
    };
    let (n,): (i64,) = sqlx::query_as(sql).fetch_one(&r.server.pool).await.unwrap();
    n
}

#[tokio::test(flavor = "multi_thread")]
async fn signs_in_syncs_and_gets_a_working_guest_link() {
    let r = rig().await;
    let demo = snap(&r, Some("Saffron butter cod"), None, 1).await; // before sign-in: demo menu
    assert_eq!(demo.location_id.as_deref(), Some(DEMO_LOCATION_ID));

    let signed = sign_in(&r.phone, &r.base, "Maya@Atelier", PASSWORD, "Pixel 9")
        .await
        .unwrap();
    assert_eq!(signed.me.staff.display_name, "Maya");
    assert_eq!(
        r.phone.setting(Setting::LocationId).await.unwrap(),
        Some(r.location.clone())
    );
    assert_eq!(
        r.phone
            .setting(Setting::OrganizationName)
            .await
            .unwrap()
            .as_deref(),
        Some("Atelier No. 8")
    );
    assert_eq!(
        r.phone
            .setting(Setting::StaffDisplayName)
            .await
            .unwrap()
            .as_deref(),
        Some("Maya")
    );
    let menu = r.phone.menu_items(&r.location).await.unwrap();
    assert_eq!(menu.len(), 2);
    assert!(menu.iter().all(|m| m.source == "server"));

    let plate = snap(&r, Some("Saffron butter cod"), Some("12B"), 2).await;
    let report = sync_pending(&r.phone, &r.media).await.unwrap();
    assert_eq!(
        (
            report.synced,
            report.refused,
            report.remaining,
            report.stopped
        ),
        (1, 0, 0, None)
    );

    let synced = r.phone.get(&plate.client_id).await.unwrap().unwrap();
    assert_eq!(synced.sync_state, "synced");
    let url = synced.guest_url.clone().unwrap();
    assert!(url.starts_with(&format!("{}/g/#", r.base)), "{url}");
    assert!(
        synced.remote_media_id.is_some()
            && synced.guest_expires_at.is_some()
            && synced.last_sync_error.is_none()
    );
    // The demo plate stays on the phone.
    assert_eq!(
        r.phone
            .get(&demo.client_id)
            .await
            .unwrap()
            .unwrap()
            .sync_state,
        "pending"
    );

    // On the server: one capture with the dish; the device-only table label never left the phone.
    assert_eq!(server_count(&r, "captures").await, 1);
    let row: (String, String, Option<String>) =
        sqlx::query_as("SELECT client_id, capture_time_utc, dish_name FROM captures")
            .fetch_one(&r.server.pool)
            .await
            .unwrap();
    assert_eq!(
        row,
        (
            plate.client_id.clone(),
            plate.capture_time_utc.clone(),
            Some("Saffron butter cod".to_owned())
        )
    );
    let everything: String = sqlx::query_as::<_, (String,)>("SELECT group_concat(quote(c.id) || quote(c.dish_name) || quote(c.client_id)) FROM captures c")
        .fetch_one(&r.server.pool)
        .await
        .unwrap()
        .0;
    assert!(!everything.contains("12B"));

    // The guest link from the phone opens the guest session on the server.
    let token = url.split_once('#').unwrap().1.to_owned();
    let base = r.base.clone();
    let status = tokio::task::spawn_blocking(move || {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build()
            .into();
        agent
            .post(&format!("{base}/api/v1/guest/session"))
            .send_json(serde_json::json!({ "token": token }))
            .unwrap()
            .status()
            .as_u16()
    })
    .await
    .unwrap();
    assert_eq!(status, 200);
}

#[tokio::test(flavor = "multi_thread")]
async fn offline_leaves_captures_pending_then_they_sync() {
    let r = rig().await;
    sign_in(&r.phone, &r.base, "maya@atelier", PASSWORD, "Pixel 9")
        .await
        .unwrap();
    let plate = snap(&r, Some("Pork belly bao"), None, 3).await;

    // Nothing listens on port 1: as good as no network.
    r.phone
        .set_setting(Setting::ServerUrl, "http://127.0.0.1:1")
        .await
        .unwrap();
    let report = sync_pending(&r.phone, &r.media).await.unwrap();
    assert_eq!(report.synced, 0);
    assert_eq!(report.remaining, 1);
    assert!(report.stopped.as_deref().unwrap().contains("can't reach"));
    let waiting = r.phone.get(&plate.client_id).await.unwrap().unwrap();
    assert_eq!(
        (waiting.sync_state.as_str(), waiting.sync_attempts),
        ("pending", 1)
    );
    assert!(
        waiting.guest_url.is_none(),
        "no QR before the acknowledgement"
    );
    assert_eq!(server_count(&r, "captures").await, 0);

    r.phone
        .set_setting(Setting::ServerUrl, &r.base)
        .await
        .unwrap();
    let report = sync_pending(&r.phone, &r.media).await.unwrap();
    assert_eq!((report.synced, report.remaining), (1, 0));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_lost_acknowledgement_is_recovered_without_a_duplicate() {
    let r = rig().await;
    let signed = sign_in(&r.phone, &r.base, "maya@atelier", PASSWORD, "Pixel 9")
        .await
        .unwrap();
    let plate = snap(&r, None, None, 4).await;

    // The server accepted everything, but the phone never heard back.
    let bytes = std::fs::read(capsnap_store::media_path(
        &r.media,
        &plate.media_sha256,
        "image/jpeg",
    ))
    .unwrap();
    let (base, token, c, loc) = (
        r.base.clone(),
        signed.token.clone(),
        plate.clone(),
        r.location.clone(),
    );
    tokio::task::spawn_blocking(move || {
        let server = Server::new(&base).unwrap();
        let media = server.upload(&token, &bytes, &c.media_sha256).unwrap();
        server
            .capture(
                &token,
                &capsnap_sync::CaptureRequest {
                    client_id: &c.client_id,
                    media_id: &media,
                    location_id: &loc,
                    menu_item_id: None,
                    capture_time_utc: &c.capture_time_utc,
                },
            )
            .unwrap();
    })
    .await
    .unwrap();

    let report = sync_pending(&r.phone, &r.media).await.unwrap();
    assert_eq!(report.synced, 1);
    assert_eq!(
        server_count(&r, "captures").await,
        1,
        "same client id, same capture"
    );
    assert_eq!(
        server_count(&r, "media_assets").await,
        1,
        "same digest, same photo"
    );
    assert!(
        r.phone
            .get(&plate.client_id)
            .await
            .unwrap()
            .unwrap()
            .guest_url
            .is_some()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_photo_the_server_no_longer_has_is_uploaded_again() {
    let r = rig().await;
    sign_in(&r.phone, &r.base, "maya@atelier", PASSWORD, "Pixel 9")
        .await
        .unwrap();
    let plate = snap(&r, None, None, 5).await;
    // As if an earlier upload had been removed by the server's retention job.
    r.phone
        .set_remote_media(
            &plate.client_id,
            Some("00000000-0000-0000-0000-000000000000"),
        )
        .await
        .unwrap();
    let report = sync_pending(&r.phone, &r.media).await.unwrap();
    assert_eq!((report.synced, report.refused), (1, 0));
    assert_ne!(
        r.phone
            .get(&plate.client_id)
            .await
            .unwrap()
            .unwrap()
            .remote_media_id
            .as_deref(),
        Some("00000000-0000-0000-0000-000000000000")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_revoked_session_signs_the_phone_out_and_keeps_its_plates() {
    let r = rig().await;
    sign_in(&r.phone, &r.base, "maya@atelier", PASSWORD, "Pixel 9")
        .await
        .unwrap();
    let plate = snap(&r, None, None, 6).await;
    // Spec runbook "Device lost": the manager revokes the phone's session.
    capsnap_server::admin::revoke_sessions(&r.server.pool, "maya@atelier")
        .await
        .unwrap();

    let report = sync_pending(&r.phone, &r.media).await.unwrap();
    assert_eq!(report.stopped.as_deref(), Some("sign in again to sync"));
    assert_eq!(r.phone.setting(Setting::SessionToken).await.unwrap(), None);
    assert_eq!(
        r.phone
            .setting(Setting::LocationId)
            .await
            .unwrap()
            .as_deref(),
        Some(DEMO_LOCATION_ID)
    );
    assert_eq!(
        r.phone
            .get(&plate.client_id)
            .await
            .unwrap()
            .unwrap()
            .sync_state,
        "pending",
        "the plate is kept"
    );
    assert_eq!(
        sync_pending(&r.phone, &r.media).await,
        Err(SyncError::SignedOut)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn refuses_bad_credentials_and_non_https_servers() {
    let r = rig().await;
    let err = sign_in(
        &r.phone,
        &r.base,
        "maya@atelier",
        "not the password",
        "Pixel 9",
    )
    .await
    .unwrap_err();
    assert_eq!(
        err,
        SyncError::SignInRefused("wrong sign-in name or password".into())
    );
    assert_eq!(r.phone.setting(Setting::SessionToken).await.unwrap(), None);

    assert!(check_server_url("https://capsnap.example.com/").is_ok());
    assert!(
        check_server_url("http://10.0.2.2:8080").is_ok(),
        "Android emulator's host"
    );
    assert!(check_server_url("http://127.0.0.1:8080").is_ok());
    for bad in [
        "http://capsnap.example.com",
        "http://192.168.1.20:8080",
        "ftp://x",
        "capsnap.example.com",
        "https://",
    ] {
        assert!(
            matches!(check_server_url(bad), Err(SyncError::BadServerUrl(_))),
            "{bad}"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_manager_reads_feedback_on_the_phone_and_a_server_account_is_refused() {
    let r = rig().await;
    sign_in(&r.phone, &r.base, "maya@atelier", PASSWORD, "Pixel 9")
        .await
        .unwrap();
    snap(&r, Some("Saffron butter cod"), None, 1).await;
    sync_pending(&r.phone, &r.media).await.unwrap();

    // A server account may not read feedback; the server's words come back.
    match feedback_page(&r.phone, 30, None).await {
        Err(SyncError::Refused { status: 403, .. }) => {}
        other => panic!("expected a refusal, got {other:?}"),
    }

    // A guest answers (straight into the server's table: the guest page has its own tests).
    sqlx::query(
        "INSERT INTO guest_feedback (id, link_id, rating, comment, created_at)
         SELECT 'fb-1', id, 4, 'Lovely', strftime('%Y-%m-%dT%H:%M:%fZ', 'now') FROM guest_links",
    )
    .execute(&r.server.pool)
    .await
    .unwrap();

    // The manager signs in on a second phone.
    let dir = tempfile::tempdir().unwrap();
    let boss = LocalStore::open(&dir.path().join("boss.db")).await.unwrap();
    boss.seed_demo_if_empty().await.unwrap();
    sign_in(&boss, &r.base, "ada@atelier", PASSWORD, "Pixel 9")
        .await
        .unwrap();
    // Admins and managers are not tied to a location in the sign-in; the phone works at the first.
    assert!(boss.setting(Setting::LocationId).await.unwrap().is_some());
    let page = feedback_page(&boss, 30, None).await.unwrap();
    assert_eq!(page.summary.count, 1);
    assert_eq!(page.summary.average, Some(4.0));
    assert_eq!(page.summary.distribution, [0, 0, 0, 1, 0]);
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].comment.as_deref(), Some("Lovely"));
    assert_eq!(
        page.items[0].dish_name.as_deref(),
        Some("Saffron butter cod")
    );
    assert!(page.next_before.is_none());

    // Only the server's own cursors are accepted.
    let bad = feedback_page(&boss, 30, Some("x&days=1".into())).await;
    assert!(matches!(bad, Err(SyncError::Local(_))), "{bad:?}");
}
