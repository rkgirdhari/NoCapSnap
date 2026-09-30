//! CapSnap server (W3a): the staff API, media validation and the guest
//! feedback portal, on Axum + SQLite (Spec §2, §5). Self-hosted; no cloud
//! services, no third-party calls except the owner-consented ONB-1 import.

pub mod admin;
pub mod auth;
pub mod captures;
pub mod config;
pub mod db;
pub mod error;
pub mod guest;
pub mod media;
pub mod menu;
pub mod onboarding;
pub mod password;
pub mod portal;
pub mod retention;
pub mod util;

use std::sync::Arc;
use std::time::Instant;

use axum::extract::{DefaultBodyLimit, MatchedPath, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use sqlx::SqlitePool;

pub use config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub cfg: Arc<Config>,
    pub limiter: Arc<auth::LoginLimiter>,
    pub onboard: Arc<capsnap_onboard::Config>,
}

impl AppState {
    pub async fn open(cfg: Config) -> Result<Self, sqlx::Error> {
        std::fs::create_dir_all(&cfg.media_dir).map_err(sqlx::Error::Io)?;
        let pool = db::open(&cfg.db_path).await?;
        Ok(Self {
            pool,
            cfg: Arc::new(cfg),
            limiter: Arc::default(),
            onboard: Arc::new(capsnap_onboard::Config::default()),
        })
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/v1/health",
            get(|| async { Json(serde_json::json!({ "status": "ok" })) }),
        )
        .route("/api/v1/auth/sessions", post(auth::sign_in))
        .route("/api/v1/auth/sessions/current", delete(auth::sign_out))
        .route("/api/v1/me", get(auth::me))
        .route("/api/v1/locations/{id}/menu-items", get(menu::list))
        // Streamed and size-checked inside the handler.
        .route(
            "/api/v1/media",
            post(media::upload).layer(DefaultBodyLimit::disable()),
        )
        .route("/api/v1/captures", post(captures::create))
        .route("/api/v1/guest/session", post(guest::exchange))
        .route("/api/v1/guest/photo", get(guest::photo))
        .route("/api/v1/guest/feedback", post(guest::submit))
        .route("/api/v1/onboarding/import", post(onboarding::import))
        .route("/g", get(portal::redirect))
        .route("/g/", get(portal::index))
        .route("/g/portal.js", get(portal::script))
        .route("/g/portal.css", get(portal::style))
        .fallback(|| async {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "not_found", "message": "no such page" })),
            )
        })
        .layer(middleware::from_fn_with_state(state.clone(), harden))
        .with_state(state)
}

/// Security headers on every response, and a request log that records the
/// route *template*, never the raw path, query or body (Spec §6: logs carry
/// no tokens, comments or image bytes).
async fn harden(State(_): State<AppState>, req: Request, next: Next) -> Response {
    let started = Instant::now();
    let method = req.method().clone();
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_owned())
        .unwrap_or_else(|| "unmatched".into());
    let portal = route.starts_with("/g");
    let mut resp = next.run(req).await;
    let h = resp.headers_mut();
    let csp = if portal {
        "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; connect-src 'self'; \
         form-action 'none'; frame-ancestors 'none'; base-uri 'none'"
    } else {
        "default-src 'none'; frame-ancestors 'none'"
    };
    h.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(csp),
    );
    h.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    h.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    h.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    h.insert(
        "permissions-policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    h.entry(header::CACHE_CONTROL)
        .or_insert(HeaderValue::from_static("no-store"));
    tracing::info!(%method, route, status = resp.status().as_u16(), ms = started.elapsed().as_millis() as u64, "request");
    resp
}

/// Runs the server until Ctrl-C / SIGTERM, with the retention job hourly.
pub async fn serve(state: AppState) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(state.cfg.bind).await?;
    tracing::info!(addr = %state.cfg.bind, "listening");
    let (pool, media) = (state.pool.clone(), state.cfg.media_dir.clone());
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(3600));
        loop {
            tick.tick().await;
            match retention::run(&pool, &media, chrono::Utc::now()).await {
                Ok(r) => tracing::info!(?r, "retention"),
                Err(e) => tracing::error!(error = %e, "retention failed"),
            }
        }
    });
    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown())
        .await
}

async fn shutdown() {
    let ctrl_c = async { tokio::signal::ctrl_c().await.ok() };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
}
