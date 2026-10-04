//! Staff sign-in and device sessions (Spec §5: tenant and role come from the
//! session, never from the request body).

use std::collections::HashMap;
use std::sync::Mutex;

use axum::Json;
use axum::extract::{FromRequestParts, State};
use axum::http::StatusCode;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{ApiError, ApiResult};
use crate::util::{looks_like_token, new_id, new_token, plus, rfc3339, token_hash};
use crate::{AppState, password};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Manager,
    Chef,
    Server,
}

impl Role {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(Role::Admin),
            "manager" => Some(Role::Manager),
            "chef" => Some(Role::Chef),
            "server" => Some(Role::Server),
            _ => None,
        }
    }
    /// Admins and managers see every location of their organization.
    pub fn all_locations(self) -> bool {
        matches!(self, Role::Admin | Role::Manager)
    }
}

/// The signed-in staff member behind a request. Handlers take this as an
/// argument, so an unauthenticated request never reaches them.
#[derive(Debug, Clone)]
pub struct Staff {
    pub session_id: String,
    pub staff_id: String,
    pub org_id: String,
    pub role: Role,
}

impl FromRequestParts<AppState> for Staff {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .filter(|t| looks_like_token(t))
            .ok_or_else(ApiError::unauthorized)?;
        let now = rfc3339(Utc::now());
        let row: Option<(String, String, String, String)> = sqlx::query_as(
            "SELECT s.id, st.id, st.org_id, st.role
             FROM device_sessions s
             JOIN staff st ON st.id = s.staff_id
             JOIN organizations o ON o.id = st.org_id
             WHERE s.token_hash = ? AND s.revoked_at IS NULL AND s.expires_at > ?
               AND st.active = 1 AND o.status = 'active'",
        )
        .bind(token_hash(token))
        .bind(&now)
        .fetch_optional(&state.pool)
        .await?;
        let (session_id, staff_id, org_id, role) = row.ok_or_else(ApiError::unauthorized)?;
        sqlx::query("UPDATE device_sessions SET last_seen_at = ? WHERE id = ?")
            .bind(&now)
            .bind(&session_id)
            .execute(&state.pool)
            .await?;
        Ok(Staff {
            session_id,
            staff_id,
            org_id,
            role: Role::parse(&role).ok_or_else(ApiError::unauthorized)?,
        })
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub id: String,
    pub name: String,
    pub timezone: String,
}

/// The locations this staff member may work at.
pub async fn locations_for(state: &AppState, staff: &Staff) -> ApiResult<Vec<Location>> {
    let rows = if staff.role.all_locations() {
        sqlx::query_as("SELECT id, name, timezone FROM locations WHERE org_id = ? ORDER BY name")
            .bind(&staff.org_id)
            .fetch_all(&state.pool)
            .await?
    } else {
        sqlx::query_as(
            "SELECT l.id, l.name, l.timezone FROM locations l
             JOIN staff_locations sl ON sl.location_id = l.id
             WHERE l.org_id = ? AND sl.staff_id = ? ORDER BY l.name",
        )
        .bind(&staff.org_id)
        .bind(&staff.staff_id)
        .fetch_all(&state.pool)
        .await?
    };
    Ok(rows)
}

/// A location of the session's organization that this staff member may use;
/// anything else is "not found", so other tenants' ids reveal nothing.
pub async fn allowed_location(
    state: &AppState,
    staff: &Staff,
    location_id: &str,
) -> ApiResult<Location> {
    locations_for(state, staff)
        .await?
        .into_iter()
        .find(|l| l.id == location_id)
        .ok_or_else(|| ApiError::not_found("location"))
}

// ---------- Sign-in rate limit ----------

const MAX_FAILURES: usize = 5;
const WINDOW_MINUTES: i64 = 15;

/// Failed sign-ins per sign-in name, in memory (one server process). Per name
/// rather than per address, because behind a proxy every client shares one.
#[derive(Default)]
pub struct LoginLimiter {
    failures: Mutex<HashMap<String, Vec<DateTime<Utc>>>>,
}

impl LoginLimiter {
    fn recent(list: &mut Vec<DateTime<Utc>>, now: DateTime<Utc>) {
        let cutoff = crate::util::minus(now, TimeDelta::minutes(WINDOW_MINUTES));
        list.retain(|t| *t > cutoff);
    }

    pub fn blocked(&self, login: &str, now: DateTime<Utc>) -> bool {
        let mut map = self.failures.lock().unwrap();
        match map.get_mut(login) {
            Some(list) => {
                Self::recent(list, now);
                list.len() >= MAX_FAILURES
            }
            None => false,
        }
    }

    pub fn record_failure(&self, login: &str, now: DateTime<Utc>) {
        let mut map = self.failures.lock().unwrap();
        // Keep the map from growing without bound under a spray of made-up names.
        if map.len() > 10_000 {
            map.retain(|_, list| {
                Self::recent(list, now);
                !list.is_empty()
            });
        }
        let list = map.entry(login.to_owned()).or_default();
        Self::recent(list, now);
        list.push(now);
    }

    pub fn clear(&self, login: &str) {
        self.failures.lock().unwrap().remove(login);
    }
}

// ---------- Routes ----------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SignInRequest {
    login: String,
    password: String,
    device_label: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Me {
    pub staff: StaffView,
    pub organization: OrgView,
    pub locations: Vec<Location>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaffView {
    pub id: String,
    pub display_name: String,
    pub role: Role,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgView {
    pub id: String,
    pub name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedIn {
    pub token: String,
    pub expires_at: String,
    #[serde(flatten)]
    pub me: Me,
}

pub async fn sign_in(
    State(state): State<AppState>,
    Json(req): Json<SignInRequest>,
) -> ApiResult<(StatusCode, Json<SignedIn>)> {
    let login = req.login.trim().to_lowercase();
    let label = req.device_label.trim().chars().take(60).collect::<String>();
    if login.is_empty() || label.is_empty() || req.password.len() > 1024 {
        return Err(ApiError::bad_request(
            "invalid",
            "sign-in name, password and device name are required",
        ));
    }
    let now = Utc::now();
    if state.limiter.blocked(&login, now) {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            format!("too many failed attempts; wait {WINDOW_MINUTES} minutes"),
        ));
    }
    let row: Option<(String, String, String, String, String)> = sqlx::query_as(
        "SELECT st.id, st.org_id, st.role, st.display_name, st.password_hash
         FROM staff st JOIN organizations o ON o.id = st.org_id
         WHERE st.login = ? AND st.active = 1 AND o.status = 'active'",
    )
    .bind(&login)
    .fetch_optional(&state.pool)
    .await?;

    // Argon2 is deliberately slow; keep it off the async workers.
    let phc = row
        .as_ref()
        .map(|r| r.4.clone())
        .unwrap_or_else(|| password::dummy_hash().to_owned());
    let pw = req.password;
    let ok = tokio::task::spawn_blocking(move || password::verify(&pw, &phc))
        .await
        .unwrap_or(false);
    let Some((staff_id, org_id, role, _, _)) = row.filter(|_| ok) else {
        state.limiter.record_failure(&login, now);
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "bad_credentials",
            "wrong sign-in name or password",
        ));
    };
    state.limiter.clear(&login);

    let token = new_token();
    // Timestamps taken here, for this session record.
    let created = Utc::now();
    let expires_at = rfc3339(plus(created, TimeDelta::days(state.cfg.session_days)));
    let session_id = new_id();
    let created_ok = create_session(
        &state.pool,
        &session_id,
        &staff_id,
        &token_hash(&token),
        &label,
        created,
        &expires_at,
    )
    .await?;
    if !created_ok {
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "bad_credentials",
            "wrong sign-in name or password",
        ));
    }
    let staff = Staff {
        session_id,
        staff_id,
        org_id,
        role: Role::parse(&role).ok_or_else(ApiError::unauthorized)?,
    };
    let me = me_for(&state, &staff).await?;
    tracing::info!(staff = %staff.staff_id, "signed in");
    Ok((
        StatusCode::CREATED,
        Json(SignedIn {
            token,
            expires_at,
            me,
        }),
    ))
}

pub async fn sign_out(State(state): State<AppState>, staff: Staff) -> ApiResult<StatusCode> {
    sqlx::query("UPDATE device_sessions SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL")
        .bind(rfc3339(Utc::now()))
        .bind(&staff.session_id)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn me(State(state): State<AppState>, staff: Staff) -> ApiResult<Json<Me>> {
    Ok(Json(me_for(&state, &staff).await?))
}

async fn me_for(state: &AppState, staff: &Staff) -> ApiResult<Me> {
    let (display_name,): (String,) = sqlx::query_as("SELECT display_name FROM staff WHERE id = ?")
        .bind(&staff.staff_id)
        .fetch_one(&state.pool)
        .await?;
    let (org_name,): (String,) = sqlx::query_as("SELECT name FROM organizations WHERE id = ?")
        .bind(&staff.org_id)
        .fetch_one(&state.pool)
        .await?;
    Ok(Me {
        staff: StaffView {
            id: staff.staff_id.clone(),
            display_name,
            role: staff.role,
        },
        organization: OrgView {
            id: staff.org_id.clone(),
            name: org_name,
        },
        locations: locations_for(state, staff).await?,
    })
}

/// Record a device session, but only while the account is still active, in one statement.
/// A removal (`admin::remove_staff`) that commits while sign-in is checking the password
/// must not be followed by a fresh session row. Returns false when nothing was inserted.
pub async fn create_session(
    pool: &sqlx::SqlitePool,
    session_id: &str,
    staff_id: &str,
    token_hash: &str,
    label: &str,
    created: DateTime<Utc>,
    expires_at: &str,
) -> Result<bool, sqlx::Error> {
    let inserted = sqlx::query(
        "INSERT INTO device_sessions (id, staff_id, token_hash, device_label, created_at, last_seen_at, expires_at)
         SELECT ?, ?, ?, ?, ?, ?, ? WHERE EXISTS (SELECT 1 FROM staff WHERE id = ? AND active = 1)",
    )
    .bind(session_id)
    .bind(staff_id)
    .bind(token_hash)
    .bind(label)
    .bind(rfc3339(created))
    .bind(rfc3339(created))
    .bind(expires_at)
    .bind(staff_id)
    .execute(pool)
    .await?
    .rows_affected();
    Ok(inserted == 1)
}
