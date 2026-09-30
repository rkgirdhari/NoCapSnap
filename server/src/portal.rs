//! The guest page, compiled into the binary (no third-party assets; Spec §6).

use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Redirect};

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
