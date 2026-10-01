//! Spec §6: logs carry no tokens, comments or image bytes. Its own test
//! binary so it can install a process-wide log subscriber.
mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::json;

#[tokio::test]
async fn logs_carry_no_tokens_passwords_comments_or_image_bytes() {
    // Global subscriber: this test has its own process (a thread-local one
    // misses events whose call sites were first seen by other test threads).
    let logs = LogBuffer::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(logs.clone())
        .with_ansi(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let w = world().await;
    let app = app(&w);
    let t = sign_in(&app, "chen@atelier").await;
    let media = upload_ok(&app, &t, phone_jpeg(320, 240, 8)).await;
    let ack = capture(&app, &t, "log-check-0001", &media, &w.loc_a1, None)
        .await
        .json();
    let token = token_from(ack["guestUrl"].as_str().unwrap());
    let session = guest_session(&app, &token).await;
    let c = cookie(&session);
    let comment = "the sauce was split, tell Chen";
    assert_eq!(
        feedback(&app, &c, json!({ "rating": 2, "comment": comment }))
            .await
            .status,
        StatusCode::CREATED
    );
    get(
        &app,
        &format!("/api/v1/locations/{}/menu-items", w.loc_a1),
        Some(&t),
    )
    .await;

    let text = String::from_utf8(logs.0.lock().unwrap().clone()).unwrap();
    assert!(
        text.contains(r#"route="/api/v1/captures""#)
            && text.contains(r#"route="/api/v1/locations/{id}/menu-items""#),
        "{text}"
    );
    let session_token = c.split_once('=').unwrap().1;
    for secret in [
        t.as_str(),
        token.as_str(),
        session_token,
        PASSWORD,
        comment,
        &w.loc_a1,
    ] {
        assert!(!text.contains(secret), "log contains {secret:?}");
    }
    assert!(!text.contains("JFIF"), "no image bytes");
}
