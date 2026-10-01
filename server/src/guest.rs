//! Guest side (Spec §4). The token lives in the URL fragment, so it never
//! reaches the server in a page request; the page posts it once, receives a
//! short-lived cookie session and removes the token from history.
//!
//! Every way a link can be unusable (unknown, expired, answered) gets the same
//! answer, and the same form is shown whatever the guest's rating: no review
//! gating (Spec §4 neutrality mandate).

use axum::Json;
use axum::extract::State;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::{TimeDelta, Utc};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::error::{ApiError, ApiResult};
use crate::media::storage_path;
use crate::util::{looks_like_token, new_id, new_token, parse_time, plus, rfc3339, token_hash};

const COOKIE_NAME: &str = "capsnap_guest";
pub const MAX_COMMENT_CHARS: usize = 2000;

fn unavailable() -> ApiError {
    ApiError::new(
        StatusCode::NOT_FOUND,
        "link_unavailable",
        "this feedback link is no longer available; it may have expired or already been used",
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exchange {
    token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuestView {
    dish_name: Option<String>,
    location_name: String,
    has_photo: bool,
}

pub async fn exchange(
    State(state): State<AppState>,
    Json(req): Json<Exchange>,
) -> ApiResult<Response> {
    if !looks_like_token(&req.token) {
        return Err(unavailable());
    }
    let now = Utc::now();
    let row: Option<(String, String, Option<String>, String, bool)> = sqlx::query_as(
        "SELECT gl.id, gl.expires_at, c.dish_name, l.name, m.deleted_at IS NULL
         FROM guest_links gl
         JOIN captures c ON c.id = gl.capture_id
         JOIN locations l ON l.id = c.location_id
         JOIN media_assets m ON m.id = c.media_id
         JOIN organizations o ON o.id = c.org_id
         WHERE gl.token_hash = ? AND gl.used_at IS NULL AND gl.expires_at > ? AND o.status = 'active'",
    )
    .bind(token_hash(&req.token))
    .bind(rfc3339(now))
    .fetch_optional(&state.pool)
    .await?;
    let (link_id, link_expires, dish_name, location_name, has_photo) =
        row.ok_or_else(unavailable)?;

    let session = new_token();
    let session_expires = plus(now, TimeDelta::minutes(state.cfg.guest_session_minutes))
        .min(parse_time(&link_expires).unwrap_or(now));
    sqlx::query(
        "INSERT INTO guest_sessions (id, link_id, token_hash, expires_at) VALUES (?, ?, ?, ?)",
    )
    .bind(new_id())
    .bind(&link_id)
    .bind(token_hash(&session))
    .bind(rfc3339(session_expires))
    .execute(&state.pool)
    .await?;
    let max_age = (session_expires - now).num_seconds().max(0);
    let secure = if state.cfg.cookie_secure {
        "; Secure"
    } else {
        ""
    };
    let cookie = format!(
        "{COOKIE_NAME}={session}; HttpOnly; SameSite=Strict; Path=/api/v1/guest; Max-Age={max_age}{secure}"
    );
    let mut resp = Json(GuestView {
        dish_name,
        location_name,
        has_photo,
    })
    .into_response();
    resp.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_str(&cookie).expect("cookie is ASCII"),
    );
    Ok(resp)
}

/// The guest session behind a request, from its cookie: (session id, link id).
async fn guest_session(state: &AppState, headers: &HeaderMap) -> ApiResult<(String, String)> {
    let token = headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|kv| {
            kv.trim()
                .strip_prefix(&format!("{COOKIE_NAME}="))
                .map(str::to_owned)
        })
        .find(|t| looks_like_token(t))
        .ok_or_else(unavailable)?;
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT gs.id, gs.link_id FROM guest_sessions gs JOIN guest_links gl ON gl.id = gs.link_id
         WHERE gs.token_hash = ? AND gs.expires_at > ? AND gl.used_at IS NULL",
    )
    .bind(token_hash(&token))
    .bind(rfc3339(Utc::now()))
    .fetch_optional(&state.pool)
    .await?;
    row.ok_or_else(unavailable)
}

pub async fn photo(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Response> {
    let (_, link_id) = guest_session(&state, &headers).await?;
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT m.storage_key FROM guest_links gl JOIN captures c ON c.id = gl.capture_id
         JOIN media_assets m ON m.id = c.media_id WHERE gl.id = ? AND m.deleted_at IS NULL",
    )
    .bind(&link_id)
    .fetch_optional(&state.pool)
    .await?;
    let (key,) = row.ok_or_else(|| ApiError::not_found("photo"))?;
    let bytes = tokio::fs::read(storage_path(&state.cfg.media_dir, &key))
        .await
        .map_err(|_| ApiError::not_found("photo"))?;
    Ok((
        [(CONTENT_TYPE, "image/jpeg"), (CACHE_CONTROL, "no-store")],
        bytes,
    )
        .into_response())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Feedback {
    rating: i64,
    comment: Option<String>,
}

pub async fn submit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<Feedback>,
) -> ApiResult<Response> {
    let (_, link_id) = guest_session(&state, &headers).await?;
    if !(1..=5).contains(&req.rating) {
        return Err(ApiError::bad_request("invalid_rating", "choose 1 to 5"));
    }
    let comment = req
        .comment
        .map(|c| c.trim().to_owned())
        .filter(|c| !c.is_empty());
    if comment
        .as_ref()
        .is_some_and(|c| c.chars().count() > MAX_COMMENT_CHARS)
    {
        return Err(ApiError::bad_request(
            "comment_too_long",
            format!("a comment can be up to {MAX_COMMENT_CHARS} characters"),
        ));
    }
    let now = rfc3339(Utc::now());
    let mut tx = state.pool.begin().await?;
    let inserted = sqlx::query(
        "INSERT INTO guest_feedback (id, link_id, rating, comment, created_at) VALUES (?, ?, ?, ?, ?)
         ON CONFLICT (link_id) DO NOTHING",
    )
    .bind(new_id())
    .bind(&link_id)
    .bind(req.rating)
    .bind(&comment)
    .bind(&now)
    .execute(&mut *tx)
    .await?
    .rows_affected()
        == 1;
    if !inserted {
        return Err(unavailable());
    }
    sqlx::query("UPDATE guest_links SET used_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&link_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM guest_sessions WHERE link_id = ?")
        .bind(&link_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let clear = format!("{COOKIE_NAME}=; HttpOnly; SameSite=Strict; Path=/api/v1/guest; Max-Age=0");
    let mut resp = (
        StatusCode::CREATED,
        Json(serde_json::json!({ "received": true })),
    )
        .into_response();
    resp.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_str(&clear).expect("cookie is ASCII"),
    );
    Ok(resp)
}
