//! `POST /onboarding/import` (ONB-1): an admin or manager links their own
//! website and accepts the consent statement; the server records the consent
//! and returns a draft. Nothing is saved from the draft here.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use capsnap_onboard::{CONSENT_STATEMENT, Consent, Draft, ImportError};
use chrono::Utc;
use serde::Deserialize;

use crate::AppState;
use crate::auth::Staff;
use crate::error::{ApiError, ApiResult};
use crate::util::{new_id, rfc3339};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportRequest {
    site_url: String,
    accepted_statement: String,
}

pub async fn import(
    State(state): State<AppState>,
    staff: Staff,
    Json(req): Json<ImportRequest>,
) -> ApiResult<Json<Draft>> {
    if !staff.role.all_locations() {
        return Err(ApiError::forbidden());
    }
    if req.accepted_statement != CONSENT_STATEMENT {
        return Err(ApiError::bad_request(
            "no_consent",
            "accept the statement to let us read the website",
        ));
    }
    let site_url = req.site_url.trim().chars().take(512).collect::<String>();
    // The consent is recorded from the session, whatever happens next.
    let consent = Consent {
        requested_by: staff.staff_id.clone(),
        site_url: site_url.clone(),
        accepted_statement: CONSENT_STATEMENT.to_owned(),
        accepted_at_utc: rfc3339(Utc::now()),
    };
    sqlx::query(
        "INSERT INTO onboarding_consents (id, org_id, staff_id, site_url, statement, accepted_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(new_id())
    .bind(&staff.org_id)
    .bind(&staff.staff_id)
    .bind(&consent.site_url)
    .bind(&consent.accepted_statement)
    .bind(&consent.accepted_at_utc)
    .execute(&state.pool)
    .await?;

    capsnap_onboard::import(&site_url, &consent, &state.onboard)
        .await
        .map(Json)
        .map_err(|e| {
            let status = match e {
                ImportError::NoConsent | ImportError::InvalidUrl(_) | ImportError::NotOwnSite => {
                    StatusCode::BAD_REQUEST
                }
                _ => StatusCode::UNPROCESSABLE_ENTITY,
            };
            ApiError::new(status, "import_failed", e.to_string())
        })
}
