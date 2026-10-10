//! Staff read guest feedback: `GET /locations/{id}/feedback`.
//!
//! Spec §1 and §4: feedback is an internal quality-control tool. It is shown only to the
//! restaurant's administrators and managers (the roles that already manage a location's menu),
//! for locations of their own organization, and never filtered or ranked by rating.

use axum::Json;
use axum::extract::{Path, Query, State};
use chrono::{TimeDelta, Utc};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::auth::{Staff, allowed_location};
use crate::error::{ApiError, ApiResult};
use crate::util::{minus, rfc3339};

const DEFAULT_DAYS: i64 = 30;
/// Feedback is deleted after 12 months (Spec §6), so there is nothing older to ask for.
const MAX_DAYS: i64 = 365;
const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 100;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Params {
    days: Option<i64>,
    limit: Option<i64>,
    /// The `nextBefore` of the previous page.
    before: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    id: String,
    rating: i64,
    comment: Option<String>,
    created_at: String,
    dish_name: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    count: i64,
    /// `null` when there is no feedback in the period.
    average: Option<f64>,
    /// How many guests gave 1, 2, 3, 4 and 5.
    distribution: [i64; 5],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    days: i64,
    summary: Summary,
    items: Vec<Item>,
    /// Pass this as `before` for the next (older) page; `null` on the last page.
    next_before: Option<String>,
}

/// A cursor is `<created_at>~<id>`, so two answers in the same millisecond are never skipped.
fn parse_cursor(raw: &str) -> Option<(&str, &str)> {
    let (at, id) = raw.split_once('~')?;
    (!at.is_empty() && !id.is_empty() && raw.len() <= 80).then_some((at, id))
}

pub async fn list(
    State(state): State<AppState>,
    staff: Staff,
    Path(location_id): Path<String>,
    Query(params): Query<Params>,
) -> ApiResult<Json<Page>> {
    if !staff.role.all_locations() {
        return Err(ApiError::forbidden());
    }
    let location = allowed_location(&state, &staff, &location_id).await?;
    let days = params.days.unwrap_or(DEFAULT_DAYS).clamp(1, MAX_DAYS);
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let since = rfc3339(minus(Utc::now(), TimeDelta::days(days)));
    let cursor = match params.before.as_deref() {
        None => None,
        Some(raw) => Some(parse_cursor(raw).ok_or_else(|| {
            ApiError::bad_request("bad_cursor", "`before` must be the value of `nextBefore`")
        })?),
    };

    // The summary covers the whole period, not just this page.
    let counts: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT f.rating, COUNT(*)
         FROM guest_feedback f
         JOIN guest_links l ON l.id = f.link_id
         JOIN captures c ON c.id = l.capture_id
         WHERE c.org_id = ? AND c.location_id = ? AND f.created_at >= ?
         GROUP BY f.rating",
    )
    .bind(&staff.org_id)
    .bind(&location.id)
    .bind(&since)
    .fetch_all(&state.pool)
    .await?;
    let mut distribution = [0i64; 5];
    for (rating, n) in counts {
        if let Some(slot) = usize::try_from(rating - 1)
            .ok()
            .and_then(|i| distribution.get_mut(i))
        {
            *slot = n;
        }
    }
    let count: i64 = distribution.iter().sum();
    let total: i64 = distribution
        .iter()
        .zip(1i64..)
        .map(|(n, rating)| n * rating)
        .sum();
    let average = (count > 0).then(|| ((total as f64 / count as f64) * 100.0).round() / 100.0);

    let (before_at, before_id) = cursor.unzip();
    let mut items: Vec<Item> = sqlx::query_as(
        "SELECT f.id, f.rating, f.comment, f.created_at, c.dish_name
         FROM guest_feedback f
         JOIN guest_links l ON l.id = f.link_id
         JOIN captures c ON c.id = l.capture_id
         WHERE c.org_id = ? AND c.location_id = ? AND f.created_at >= ?
           AND (? IS NULL OR (f.created_at, f.id) < (?, ?))
         ORDER BY f.created_at DESC, f.id DESC
         LIMIT ?",
    )
    .bind(&staff.org_id)
    .bind(&location.id)
    .bind(&since)
    .bind(before_at)
    .bind(before_at)
    .bind(before_id)
    .bind(limit + 1)
    .fetch_all(&state.pool)
    .await?;
    let next_before = if items.len() as i64 > limit {
        items.truncate(limit as usize);
        items.last().map(|i| format!("{}~{}", i.created_at, i.id))
    } else {
        None
    };

    Ok(Json(Page {
        days,
        summary: Summary {
            count,
            average,
            distribution,
        },
        items,
        next_before,
    }))
}
