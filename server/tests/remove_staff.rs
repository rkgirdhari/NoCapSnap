//! W5a gap 1: a staff member's personal data can be removed on request, and what they
//! captured stays.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;

#[tokio::test]
async fn removing_staff_anonymises_them_and_keeps_their_captures() {
    let w = world().await;
    let app = app(&w);
    let token = sign_in(&app, "chen@atelier").await;
    let media = upload_ok(&app, &token, phone_jpeg(64, 48, 7)).await;
    let r = capture(&app, &token, "chen-cap-1", &media, &w.loc_a1, None).await;
    assert!(
        r.status.is_success(),
        "{}",
        String::from_utf8_lossy(&r.body)
    );

    let sessions = capsnap_server::admin::remove_staff(&w.state.pool, "Chen@Atelier")
        .await
        .unwrap();
    assert_eq!(sessions, 1);

    // The phone is signed out, and the old login and password no longer work.
    assert_eq!(
        get(&app, "/api/v1/me", Some(&token)).await.status,
        StatusCode::UNAUTHORIZED
    );
    let again = json(
        &app,
        Method::POST,
        "/api/v1/auth/sessions",
        None,
        json!({ "login": "chen@atelier", "password": PASSWORD, "deviceLabel": "Pixel test" }),
    )
    .await;
    assert_eq!(again.status, StatusCode::UNAUTHORIZED);

    // Nothing personal is left on the row; it is inactive and has no locations.
    let (login, name, active): (String, String, i64) = sqlx::query_as(
        "SELECT login, display_name, active FROM staff WHERE login LIKE 'removed-%'",
    )
    .fetch_one(&w.state.pool)
    .await
    .unwrap();
    assert!(
        !login.contains("chen") && login.starts_with("removed-"),
        "{login}"
    );
    assert_eq!((name.as_str(), active), ("Former staff", 0));
    let links: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM staff_locations sl JOIN staff s ON s.id = sl.staff_id WHERE s.active = 0",
    )
    .fetch_one(&w.state.pool)
    .await
    .unwrap();
    assert_eq!(links, 0);

    // The capture they made is still there, and colleagues are untouched.
    let captures: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM captures")
        .fetch_one(&w.state.pool)
        .await
        .unwrap();
    assert_eq!(captures, 1);
    sign_in(&app, "ada@atelier").await;
}

#[tokio::test]
async fn removing_someone_who_does_not_exist_says_so() {
    let w = world().await;
    let err = capsnap_server::admin::remove_staff(&w.state.pool, "nobody@nowhere")
        .await
        .unwrap_err();
    assert!(err.contains("no staff member"), "{err}");
}

#[tokio::test]
async fn removal_succeeds_when_the_obvious_replacement_logins_are_taken() {
    let w = world().await;
    let pool = &w.state.pool;
    let (id,): (String,) = sqlx::query_as("SELECT id FROM staff WHERE login = 'chen@atelier'")
        .fetch_one(pool)
        .await
        .unwrap();
    // Logins another admin could have chosen, including the old eight-character scheme.
    for taken in [
        format!("removed-{}", &id[..8]),
        "removed-12345678".to_owned(),
    ] {
        capsnap_server::admin::create_staff(
            pool,
            "atelier",
            &taken,
            "Squatter",
            "server",
            PASSWORD,
            &[],
        )
        .await
        .unwrap();
    }
    capsnap_server::admin::remove_staff(pool, "chen@atelier")
        .await
        .unwrap();
    // A second removal in the same state also succeeds, and the replacements are distinct.
    capsnap_server::admin::remove_staff(pool, "ada@atelier")
        .await
        .unwrap();
    let removed: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT login) FROM staff WHERE active = 0 AND display_name = 'Former staff'",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(removed, 2);
}

#[tokio::test]
async fn a_session_cannot_be_created_for_an_account_removed_meanwhile() {
    let w = world().await;
    let pool = &w.state.pool;
    let (id,): (String,) = sqlx::query_as("SELECT id FROM staff WHERE login = 'chen@atelier'")
        .fetch_one(pool)
        .await
        .unwrap();
    let now = chrono::Utc::now();
    let later = "2099-01-01T00:00:00.000Z";

    // While active, the insert works.
    assert!(
        capsnap_server::auth::create_session(pool, "s1", &id, &"a".repeat(64), "Pixel", now, later)
            .await
            .unwrap()
    );
    // Sign-in has verified the password; now the removal commits; then sign-in inserts.
    capsnap_server::admin::remove_staff(pool, "chen@atelier")
        .await
        .unwrap();
    assert!(
        !capsnap_server::auth::create_session(
            pool,
            "s2",
            &id,
            &"b".repeat(64),
            "Pixel",
            now,
            later
        )
        .await
        .unwrap()
    );
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM device_sessions WHERE staff_id = ?")
        .bind(&id)
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(left, 0, "no session data survives the removal");
}
