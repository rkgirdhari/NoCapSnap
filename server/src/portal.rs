//! The guest page, compiled into the binary (no third-party assets; Spec §6).

use axum::extract::State;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Redirect};

use crate::AppState;

pub async fn index() -> impl IntoResponse {
    (
        [(CONTENT_TYPE, "text/html; charset=utf-8")],
        include_str!("../portal/index.html"),
    )
}

pub async fn script() -> impl IntoResponse {
    (
        [(CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../portal/portal.js"),
    )
}

pub async fn style() -> impl IntoResponse {
    (
        [(CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../portal/portal.css"),
    )
}

pub async fn redirect() -> Redirect {
    Redirect::permanent("/g/")
}

/// The privacy policy (Play and the app's Settings link to it). The operator's contact
/// comes from `CAPSNAP_PRIVACY_CONTACT`; until it is set the page says so plainly.
pub async fn privacy(State(state): State<AppState>) -> impl IntoResponse {
    let contact = state
        .cfg
        .privacy_contact
        .as_deref()
        .map(escape)
        .unwrap_or_else(|| "(not set yet: the operator must set CAPSNAP_PRIVACY_CONTACT)".into());
    (
        [(CONTENT_TYPE, "text/html; charset=utf-8")],
        include_str!("../portal/privacy.html").replace("{{CONTACT}}", &contact),
    )
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
