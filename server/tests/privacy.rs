//! W5a: the privacy policy page (Play requires a public URL; Spec §6).

mod common;

use axum::http::StatusCode;
use capsnap_server::{AppState, Config, router};
use common::*;

fn page(resp: &Resp) -> String {
    String::from_utf8(resp.body.clone()).unwrap()
}

#[tokio::test]
async fn the_policy_is_public_and_loads_nothing_from_elsewhere() {
    let w = world().await;
    let r = get(&app(&w), "/privacy", None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(
        r.headers["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    let csp = r.headers["content-security-policy"].to_str().unwrap();
    assert!(
        csp.contains("style-src 'self'") && !csp.contains("http"),
        "{csp}"
    );
    let html = page(&r);
    assert!(
        !html.contains("http://") && !html.contains("https://"),
        "no outside references"
    );
    assert!(!html.contains("<script"), "no scripts");
}

#[tokio::test]
async fn the_policy_states_the_retention_periods_the_server_enforces() {
    let w = world().await;
    let html = page(&get(&app(&w), "/privacy", None).await);
    // Spec §6 and src/retention.rs: keep these in step with the code.
    for needle in [
        "30 days after they arrive",
        "12 months",
        "Server logs: 30 days",
        "up to 30 days",                                          // backups (W4a)
        "Setting up a menu from a website", // ONB-1 / M1: the consent record has no retention job
        "administrators and managers can read the guest answers", // feedback screen: who sees it
    ] {
        assert!(html.contains(needle), "missing: {needle}");
    }
}

#[tokio::test]
async fn the_contact_is_shown_escaped_and_a_missing_one_is_said_plainly() {
    let w = world().await;
    assert!(page(&get(&app(&w), "/privacy", None).await).contains("CAPSNAP_PRIVACY_CONTACT"));

    let dir = tempfile::tempdir().unwrap();
    let mut cfg = Config::for_dir(dir.path(), "https://guests.example");
    cfg.privacy_contact = Some("privacy@example.test <b>\"x\"</b>".into());
    let state = AppState::open(cfg).await.unwrap();
    let html = page(&get(&router(state), "/privacy", None).await);
    assert!(
        html.contains("privacy@example.test &lt;b&gt;&quot;x&quot;&lt;/b&gt;"),
        "{html}"
    );
    assert!(!html.contains("<b>"));
}
