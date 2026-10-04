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
