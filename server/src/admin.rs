//! Operator commands (no public sign-up; Spec §5). Used by the
//! `capsnap-server` binary and by the tests.

use chrono::Utc;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::auth::Role;
use crate::password;
use crate::util::{new_id, rfc3339};

pub async fn create_org(pool: &SqlitePool, slug: &str, name: &str) -> Result<String, String> {
    let id = new_id();
    sqlx::query("INSERT INTO organizations (id, name, slug, created_at) VALUES (?, ?, ?, ?)")
        .bind(&id)
        .bind(name.trim())
        .bind(slug.trim())
        .bind(rfc3339(Utc::now()))
        .execute(pool)
        .await
        .map_err(|e| format!("could not create the organization: {e}"))?;
    Ok(id)
}

async fn org_id(pool: &SqlitePool, slug: &str) -> Result<String, String> {
    let row: Option<(String,)> = sqlx::query_as("SELECT id FROM organizations WHERE slug = ?")
        .bind(slug)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;
    row.map(|r| r.0)
        .ok_or_else(|| format!("no organization with slug {slug}"))
}

/// IANA names look like `Area/City`; the server's tz database is checked when present.
pub fn valid_timezone(tz: &str) -> bool {
    let shaped = tz == "UTC"
        || (tz.contains('/')
            && tz.split('/').all(|p| {
                !p.is_empty()
                    && p.chars()
                        .all(|c| c.is_ascii_alphanumeric() || "_-+".contains(c))
            }));
    let zoneinfo = std::path::Path::new("/usr/share/zoneinfo");
    shaped && (!zoneinfo.is_dir() || zoneinfo.join(tz).is_file())
}

pub async fn create_location(
    pool: &SqlitePool,
    org_slug: &str,
    name: &str,
    timezone: &str,
) -> Result<String, String> {
    if !valid_timezone(timezone) {
        return Err(format!(
            "{timezone} is not an IANA timezone such as America/Chicago"
        ));
    }
    let org = org_id(pool, org_slug).await?;
    let id = new_id();
    sqlx::query(
        "INSERT INTO locations (id, org_id, name, timezone, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&org)
    .bind(name.trim())
    .bind(timezone)
    .bind(rfc3339(Utc::now()))
    .execute(pool)
    .await
    .map_err(|e| format!("could not create the location: {e}"))?;
    Ok(id)
}

pub async fn create_staff(
    pool: &SqlitePool,
    org_slug: &str,
    login: &str,
    display_name: &str,
    role: &str,
    password: &str,
    locations: &[String],
) -> Result<String, String> {
    Role::parse(role).ok_or("role must be admin, manager, chef or server")?;
    let org = org_id(pool, org_slug).await?;
    let pw = password.to_owned();
    let hash = tokio::task::spawn_blocking(move || password::hash(&pw))
        .await
        .map_err(|e| e.to_string())??;
    let id = new_id();
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    sqlx::query("INSERT INTO staff (id, org_id, login, display_name, password_hash, role, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(&org)
        .bind(login.trim().to_lowercase())
        .bind(display_name.trim())
        .bind(&hash)
        .bind(role)
        .bind(rfc3339(Utc::now()))
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("could not create the staff member: {e}"))?;
    for location in locations {
        let ok: Option<(String,)> =
            sqlx::query_as("SELECT id FROM locations WHERE id = ? AND org_id = ?")
                .bind(location)
                .bind(&org)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        ok.ok_or_else(|| format!("location {location} is not in {org_slug}"))?;
        sqlx::query("INSERT INTO staff_locations (staff_id, location_id) VALUES (?, ?)")
            .bind(&id)
            .bind(location)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    }
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(id)
}

#[derive(Deserialize)]
pub struct MenuEntry {
    pub name: String,
    pub category: String,
}

/// Replaces a location's menu with `items`, in the given order.
pub async fn import_menu(
    pool: &SqlitePool,
    location_id: &str,
    items: &[MenuEntry],
) -> Result<usize, String> {
    let row: Option<(String,)> = sqlx::query_as("SELECT org_id FROM locations WHERE id = ?")
        .bind(location_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;
    let (org,) = row.ok_or_else(|| format!("no location {location_id}"))?;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    // Captures keep their dish name; old menu rows are retired, not deleted.
    sqlx::query("UPDATE menu_items SET is_active = 0 WHERE location_id = ?")
        .bind(location_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    for (order, item) in items.iter().enumerate() {
        sqlx::query("INSERT INTO menu_items (id, org_id, location_id, name, category, sort_order) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(new_id())
            .bind(&org)
            .bind(location_id)
            .bind(item.name.trim())
            .bind(item.category.trim())
            .bind(order as i64)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("menu item {}: {e}", order + 1))?;
    }
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(items.len())
}

/// Spec runbook "Device lost": revoke every session of one staff member.
pub async fn revoke_sessions(pool: &SqlitePool, login: &str) -> Result<u64, String> {
    sqlx::query(
        "UPDATE device_sessions SET revoked_at = ? WHERE revoked_at IS NULL
         AND staff_id = (SELECT id FROM staff WHERE login = ?)",
    )
    .bind(rfc3339(Utc::now()))
    .bind(login.trim().to_lowercase())
    .execute(pool)
    .await
    .map(|r| r.rows_affected())
    .map_err(|e| e.to_string())
}

/// Remove a staff member's personal data on request (privacy policy; Play's account-deletion
/// rule). The row stays, so the captures and consent records that point at it keep working,
/// but it is anonymised: the login and display name are replaced, the password is replaced by
/// a hash of a random value nobody sees, the account is deactivated, its device sessions and
/// location links are deleted. Returns how many sessions were deleted.
pub async fn remove_staff(pool: &SqlitePool, login: &str) -> Result<u64, String> {
    let login = login.trim().to_lowercase();
    let row: Option<(String,)> = sqlx::query_as("SELECT id FROM staff WHERE login = ?")
        .bind(&login)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;
    let (id,) = row.ok_or_else(|| format!("no staff member with login {login}"))?;

    let secret = format!("{}{}", new_id(), new_id());
    let hash = tokio::task::spawn_blocking(move || password::hash(&secret))
        .await
        .map_err(|e| e.to_string())??;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    // A full random replacement login; if one is somehow taken (create_staff allows any
    // login of the right shape), try another. A failed statement leaves the transaction usable.
    let mut attempts = 0;
    loop {
        attempts += 1;
        let result = sqlx::query(
            "UPDATE staff SET active = 0, login = ?, display_name = 'Former staff', password_hash = ? WHERE id = ?",
        )
        .bind(format!("removed-{}", new_id()))
        .bind(&hash)
        .bind(&id)
        .execute(&mut *tx)
        .await;
        match result {
            Ok(_) => break,
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() && attempts < 5 => continue,
            Err(e) => return Err(e.to_string()),
        }
    }
    sqlx::query("DELETE FROM staff_locations WHERE staff_id = ?")
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    let sessions = sqlx::query("DELETE FROM device_sessions WHERE staff_id = ?")
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?
        .rows_affected();
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(sessions)
}
