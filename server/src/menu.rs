use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;

use crate::AppState;
use crate::auth::{Staff, allowed_location};
use crate::error::ApiResult;

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
