//! Spec §6 retention, run hourly by `serve` and on demand by `capsnap-server retention`.
//!
//! - Photos: deleted 30 days after the capture reached the server; photos
//!   never attached to a capture, after 1 day.
//! - Guest feedback: deleted after 12 months.
//! - Guest links: unusable after 30 days or their first answer (enforced at
//!   use); their short sessions are deleted once expired.
//! - Device sessions: deleted 30 days after they expired or were revoked.
//! - Operational logs: kept by journald with a 30-day limit (see the README).

use std::path::Path;

use chrono::{DateTime, Months, TimeDelta, Utc};
use sqlx::SqlitePool;

use crate::media::storage_path;
use crate::util::{minus, rfc3339};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub photos_deleted: u64,
    pub feedback_deleted: u64,
    pub guest_sessions_deleted: u64,
    pub device_sessions_deleted: u64,
}

pub async fn run(
    pool: &SqlitePool,
    media_dir: &Path,
    now: DateTime<Utc>,
) -> Result<Report, sqlx::Error> {
    let mut report = Report::default();
    let month_ago = rfc3339(minus(now, TimeDelta::days(30)));
    let day_ago = rfc3339(minus(now, TimeDelta::days(1)));

    let due: Vec<(String, String)> = sqlx::query_as(
        "SELECT m.id, m.storage_key FROM media_assets m
         WHERE m.deleted_at IS NULL AND (
             (SELECT MIN(c.received_at) FROM captures c WHERE c.media_id = m.id) < ?
             OR (NOT EXISTS (SELECT 1 FROM captures c WHERE c.media_id = m.id) AND m.created_at < ?))",
    )
    .bind(&month_ago)
    .bind(&day_ago)
    .fetch_all(pool)
    .await?;
    for (id, key) in due {
        match std::fs::remove_file(storage_path(media_dir, &key)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                tracing::error!(error = %e, "retention: could not delete a photo; will retry");
                continue;
            }
        }
        sqlx::query("UPDATE media_assets SET deleted_at = ? WHERE id = ?")
            .bind(rfc3339(now))
            .bind(&id)
            .execute(pool)
            .await?;
        report.photos_deleted += 1;
    }

    let year_ago = now
        .checked_sub_months(Months::new(12))
        .unwrap_or(DateTime::<Utc>::MIN_UTC);
    report.feedback_deleted = sqlx::query("DELETE FROM guest_feedback WHERE created_at < ?")
        .bind(rfc3339(year_ago))
        .execute(pool)
        .await?
        .rows_affected();
    report.guest_sessions_deleted = sqlx::query("DELETE FROM guest_sessions WHERE expires_at < ?")
        .bind(rfc3339(now))
        .execute(pool)
        .await?
        .rows_affected();
    report.device_sessions_deleted = sqlx::query(
        "DELETE FROM device_sessions WHERE (revoked_at IS NOT NULL AND revoked_at < ?) OR expires_at < ?",
    )
    .bind(&month_ago)
    .bind(&month_ago)
    .execute(pool)
    .await?
    .rows_affected();
    Ok(report)
}
