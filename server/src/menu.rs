use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::admin::{self, MenuEntry};
use crate::auth::{Staff, allowed_location};
use crate::error::{ApiError, ApiResult};

const MAX_ITEMS: usize = 500;
const MAX_NAME: usize = 120;
const MAX_CATEGORY: usize = 40;

#[derive(Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct MenuItem {
    id: String,
    name: String,
    category: String,
}

/// `GET /locations/{id}/menu-items` (Spec §5), cached by the app for offline use.
pub async fn list(
    State(state): State<AppState>,
    staff: Staff,
    Path(location_id): Path<String>,
) -> ApiResult<Json<Vec<MenuItem>>> {
    let location = allowed_location(&state, &staff, &location_id).await?;
    let items = sqlx::query_as(
        "SELECT id, name, category FROM menu_items
         WHERE org_id = ? AND location_id = ? AND is_active = 1 ORDER BY sort_order, name",
    )
    .bind(&staff.org_id)
    .bind(&location.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(items))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplaceRequest {
    items: Vec<ReplaceItem>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReplaceItem {
    name: String,
    category: String,
}

/// Collapses whitespace and drops control characters; `None` when nothing is left.
fn tidy(text: &str, max: usize) -> Option<String> {
    let kept: String = text
        .chars()
        .filter(|c| !c.is_control() || c.is_whitespace())
        .collect();
    let words = kept.split_whitespace().collect::<Vec<_>>().join(" ");
    let cut: String = words.chars().take(max).collect();
    let cut = cut.trim().to_owned();
    (!cut.is_empty()).then_some(cut)
}

/// `PUT /locations/{id}/menu-items`: an admin or manager saves the menu they reviewed (from the
/// website import, ONB-1, or by hand), in the order sent. The old menu is retired, not deleted, so
/// captures already made keep their dish. The answer is the new list, as `GET` returns it.
pub async fn replace(
    State(state): State<AppState>,
    staff: Staff,
    Path(location_id): Path<String>,
    Json(req): Json<ReplaceRequest>,
) -> ApiResult<Json<Vec<MenuItem>>> {
    if !staff.role.all_locations() {
        return Err(ApiError::forbidden());
    }
    let location = allowed_location(&state, &staff, &location_id).await?;
    if req.items.is_empty() {
        return Err(ApiError::bad_request(
            "empty_menu",
            "send at least one dish; an empty menu would wipe the current one",
        ));
    }
    if req.items.len() > MAX_ITEMS {
        return Err(ApiError::bad_request(
            "too_many_items",
            format!("a menu can have at most {MAX_ITEMS} dishes"),
        ));
    }
    let mut seen = std::collections::HashSet::new();
    let mut entries = Vec::with_capacity(req.items.len());
    for item in &req.items {
        let (Some(name), Some(category)) = (
            tidy(&item.name, MAX_NAME),
            tidy(&item.category, MAX_CATEGORY),
        ) else {
            return Err(ApiError::bad_request(
                "bad_item",
                "every dish needs a name and a category",
            ));
        };
        // The same dish listed twice under one category is one dish.
        if seen.insert((name.to_lowercase(), category.to_lowercase())) {
            entries.push(MenuEntry { name, category });
        }
    }
    admin::import_menu(&state.pool, &location.id, &entries)
        .await
        .map_err(|e| ApiError::unprocessable("menu_not_saved", e))?;
    list(State(state), staff, Path(location_id)).await
}
