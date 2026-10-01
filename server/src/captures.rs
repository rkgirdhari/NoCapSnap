//! `POST /captures` (Spec §5): idempotent on the phone's `client_id`. The
//! answer is the acknowledgement the phone waits for before it shows
//! "Synced · QR ready" (Spec §3), and it carries the guest link.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use chrono::{TimeDelta, Utc};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::auth::{Staff, allowed_location};
use crate::error::{ApiError, ApiResult};
use crate::util::{new_id, new_token, parse_time, plus, rfc3339, token_hash};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewCapture {
    client_id: String,
    media_id: String,
    location_id: String,
    menu_item_id: Option<String>,
    capture_time_utc: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CaptureAck {
    pub capture_id: String,
    pub received_at: String,
    /// `<public base>/g/#<token>`; absent once the guest has already answered.
    pub guest_url: Option<String>,
    pub guest_expires_at: Option<String>,
}

pub async fn create(
    State(state): State<AppState>,
    staff: Staff,
    Json(req): Json<NewCapture>,
) -> ApiResult<(StatusCode, Json<CaptureAck>)> {
    if !(8..=64).contains(&req.client_id.len())
        || !req
            .client_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(ApiError::bad_request(
            "invalid_client_id",
            "clientId must be 8–64 letters, digits or dashes",
        ));
    }
    let now = Utc::now();
    let taken = parse_time(&req.capture_time_utc)
        .filter(|t| *t <= plus(now, TimeDelta::days(1)))
        .ok_or_else(|| {
            ApiError::bad_request(
                "invalid_time",
                "captureTimeUtc must be an RFC 3339 time, not in the future",
            )
        })?;

    // Everything below is looked up inside the session's organization only.
    let location = allowed_location(&state, &staff, &req.location_id).await?;
    let media: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM media_assets WHERE id = ? AND org_id = ? AND deleted_at IS NULL",
    )
    .bind(&req.media_id)
    .bind(&staff.org_id)
    .fetch_optional(&state.pool)
    .await?;
    media.ok_or_else(|| ApiError::not_found("photo"))?;
    let dish_name = match &req.menu_item_id {
        Some(id) => {
            let row: Option<(String,)> = sqlx::query_as(
                "SELECT name FROM menu_items WHERE id = ? AND org_id = ? AND location_id = ?",
            )
            .bind(id)
            .bind(&staff.org_id)
            .bind(&location.id)
            .fetch_optional(&state.pool)
            .await?;
            Some(row.ok_or_else(|| ApiError::not_found("dish"))?.0)
        }
        None => None,
    };

    let mut tx = state.pool.begin().await?;
    let inserted = sqlx::query(
        "INSERT INTO captures (id, org_id, location_id, staff_id, client_id, media_id, menu_item_id, dish_name, capture_time_utc, received_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (org_id, client_id) DO NOTHING",
    )
    .bind(new_id())
    .bind(&staff.org_id)
    .bind(&location.id)
    .bind(&staff.staff_id)
    .bind(&req.client_id)
    .bind(&req.media_id)
    .bind(&req.menu_item_id)
    .bind(&dish_name)
    .bind(rfc3339(taken))
    .bind(rfc3339(now))
    .execute(&mut *tx)
    .await?
    .rows_affected()
        == 1;
    let (capture_id, media_id, received_at): (String, String, String) = sqlx::query_as(
        "SELECT id, media_id, received_at FROM captures WHERE org_id = ? AND client_id = ?",
    )
    .bind(&staff.org_id)
    .bind(&req.client_id)
    .fetch_one(&mut *tx)
    .await?;
    if media_id != req.media_id {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "client_id_reused",
            "this capture id was already used for another photo",
        ));
    }

    // The guest link. Only its hash is stored, so a retried request (the phone
    // never saw the first answer) gets a fresh token and the old one stops
    // working — unless the guest has already answered, which ends the link.
    let link: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT id, used_at FROM guest_links WHERE capture_id = ?")
            .bind(&capture_id)
            .fetch_optional(&mut *tx)
            .await?;
    let issued = Utc::now(); // this link's own timestamp
    let expires_at = rfc3339(plus(issued, TimeDelta::days(state.cfg.guest_link_days)));
    let token = new_token();
    let guest = match link {
        None => {
            sqlx::query("INSERT INTO guest_links (id, capture_id, token_hash, created_at, expires_at) VALUES (?, ?, ?, ?, ?)")
                .bind(new_id())
                .bind(&capture_id)
                .bind(token_hash(&token))
                .bind(rfc3339(issued))
                .bind(&expires_at)
                .execute(&mut *tx)
                .await?;
            true
        }
        Some((link_id, None)) => {
            sqlx::query("UPDATE guest_links SET token_hash = ?, created_at = ?, expires_at = ? WHERE id = ?")
                .bind(token_hash(&token))
                .bind(rfc3339(issued))
                .bind(&expires_at)
                .bind(&link_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM guest_sessions WHERE link_id = ?")
                .bind(&link_id)
                .execute(&mut *tx)
                .await?;
            true
        }
        Some((_, Some(_))) => false,
    };
    tx.commit().await?;

    let ack = CaptureAck {
        capture_id,
        received_at,
        guest_url: guest.then(|| format!("{}/g/#{token}", state.cfg.public_base_url)),
        guest_expires_at: guest.then_some(expires_at),
    };
    Ok((
        if inserted {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        Json(ack),
    ))
}
