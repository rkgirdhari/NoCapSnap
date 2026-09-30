//! CapSnap's on-device store (promoted from the W0 spike in W2).
//!
//! A local-first outbox: captures are committed to SQLite as `pending`, and
//! flip to `synced` only after the server acknowledges them.
//! [`LocalStore::ingest`] processes the photo on the device (orientation,
//! 2048 px, metadata stripped), stores it next to the database and queues the
//! capture in one call. W2 adds the menu cache and device-only settings.

mod ingest;
mod menu;
pub mod photo;

pub use ingest::{
    CaptureDetails, IngestError, MAX_MEDIA_BYTES, MAX_TABLE_LABEL_CHARS, media_path, sniff_mime,
    thumb_path,
};
pub use menu::{DEMO_LOCATION_ID, DEMO_LOCATION_NAME, MenuItem, MenuSource, NewMenuItem, Setting};
/// The store's database error, so callers need no direct sqlx dependency.
pub use sqlx::Error as DbError;

use std::ffi::{CStr, c_char};
use std::path::Path;
use std::str::FromStr;

use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Capture {
    pub client_id: String,
    pub staff_id: String,
    pub media_sha256: String,
    pub capture_time_utc: String,
    pub sync_state: String,
    pub synced_at_utc: Option<String>,
    pub media_mime: Option<String>,
    pub media_bytes: Option<i64>,
    pub media_width: Option<i64>,
    pub media_height: Option<i64>,
    pub location_id: Option<String>,
    pub menu_item_id: Option<String>,
    pub dish_name: Option<String>,
    /// Device-only staff reference (owner default M6): never synced.
    pub table_label: Option<String>,
}

#[derive(Default)]
pub struct NewCapture<'a> {
    pub client_id: &'a str,
    pub staff_id: &'a str,
    pub media_sha256: &'a str,
    pub capture_time_utc: &'a str,
    pub media_mime: Option<&'a str>,
    pub media_bytes: Option<i64>,
    pub media_width: Option<i64>,
    pub media_height: Option<i64>,
    pub location_id: Option<&'a str>,
    pub menu_item_id: Option<&'a str>,
    pub dish_name: Option<&'a str>,
    pub table_label: Option<&'a str>,
}

/// `SELECT <every Capture column> FROM captures <tail>`, as a `&'static str`
/// (sqlx 0.9 only accepts static query text).
macro_rules! select_captures {
    ($tail:literal) => {
        concat!(
            "SELECT client_id, staff_id, media_sha256, capture_time_utc, sync_state, synced_at_utc,
                    media_mime, media_bytes, media_width, media_height,
                    location_id, menu_item_id, dish_name, table_label
             FROM captures ",
            $tail
        )
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    pub pending: i64,
    pub synced: i64,
}

pub struct LocalStore {
    pool: SqlitePool,
}

impl LocalStore {
    /// Opens (creating if needed) the store in WAL mode with full fsync, so a
    /// committed capture survives the app being killed or the device losing power.
    pub async fn open(path: &Path) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::from_str("sqlite://")?
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Full)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        MIGRATOR.run(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn close(self) {
        self.pool.close().await;
    }

    /// Idempotent: re-enqueuing an existing `client_id` returns the stored row
    /// unchanged instead of creating a duplicate.
    pub async fn enqueue(&self, new: &NewCapture<'_>) -> Result<Capture, sqlx::Error> {
        sqlx::query(
            "INSERT INTO captures
                 (client_id, staff_id, media_sha256, capture_time_utc, media_mime, media_bytes,
                  media_width, media_height, location_id, menu_item_id, dish_name, table_label)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (client_id) DO NOTHING",
        )
        .bind(new.client_id)
        .bind(new.staff_id)
        .bind(new.media_sha256)
        .bind(new.capture_time_utc)
        .bind(new.media_mime)
        .bind(new.media_bytes)
        .bind(new.media_width)
        .bind(new.media_height)
        .bind(new.location_id)
        .bind(new.menu_item_id)
        .bind(new.dish_name)
        .bind(new.table_label)
        .execute(&self.pool)
        .await?;
        self.get(new.client_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn get(&self, client_id: &str) -> Result<Option<Capture>, sqlx::Error> {
        sqlx::query_as(select_captures!("WHERE client_id = ?"))
            .bind(client_id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Captures still waiting for a server acknowledgement ("saved offline / QR not ready").
    pub async fn pending(&self) -> Result<Vec<Capture>, sqlx::Error> {
        sqlx::query_as(select_captures!(
            "WHERE sync_state = 'pending' ORDER BY capture_time_utc, id"
        ))
        .fetch_all(&self.pool)
        .await
    }

    /// Newest first, for the History screen.
    pub async fn list_recent(&self, limit: i64) -> Result<Vec<Capture>, sqlx::Error> {
        sqlx::query_as(select_captures!(
            "ORDER BY capture_time_utc DESC, id DESC LIMIT ?"
        ))
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn counts(&self) -> Result<Counts, sqlx::Error> {
        let (pending, synced): (i64, i64) = sqlx::query_as(
            "SELECT COUNT(*) FILTER (WHERE sync_state = 'pending'),
                    COUNT(*) FILTER (WHERE sync_state = 'synced')
             FROM captures",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(Counts { pending, synced })
    }

    /// Records the server acknowledgement. Returns false if the capture was
    /// unknown or already synced.
    pub async fn mark_synced(
        &self,
        client_id: &str,
        synced_at_utc: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE captures SET sync_state = 'synced', synced_at_utc = ?
             WHERE client_id = ? AND sync_state = 'pending'",
        )
        .bind(synced_at_utc)
        .bind(client_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn pragma(&self, name: Pragma) -> Result<String, sqlx::Error> {
        let row: (String,) = sqlx::query_as(name.query()).fetch_one(&self.pool).await?;
        Ok(row.0)
    }

    pub async fn sqlite_version(&self) -> Result<String, sqlx::Error> {
        let row: (String,) = sqlx::query_as("SELECT sqlite_version()")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Pragma {
    JournalMode,
    Synchronous,
    ForeignKeys,
}

impl Pragma {
    fn query(self) -> &'static str {
        match self {
            Pragma::JournalMode => "SELECT CAST(journal_mode AS TEXT) FROM pragma_journal_mode",
            Pragma::Synchronous => "SELECT CAST(synchronous AS TEXT) FROM pragma_synchronous",
            Pragma::ForeignKeys => "SELECT CAST(foreign_keys AS TEXT) FROM pragma_foreign_keys",
        }
    }
}

/// Full outbox round trip against a real file: open + migrate, enqueue,
/// idempotent re-enqueue, mark synced, reopen and re-read.
pub async fn selftest(path: &Path) -> Result<String, String> {
    let digest = "a".repeat(64);
    let new = NewCapture {
        client_id: "w0-selftest",
        staff_id: "staff-1",
        media_sha256: &digest,
        capture_time_utc: "2026-09-30T12:00:00Z",
        ..NewCapture::default()
    };
    let store = LocalStore::open(path)
        .await
        .map_err(|e| format!("open: {e}"))?;
    let version = store
        .sqlite_version()
        .await
        .map_err(|e| format!("version: {e}"))?;
    let journal = store
        .pragma(Pragma::JournalMode)
        .await
        .map_err(|e| format!("pragma: {e}"))?;
    store
        .enqueue(&new)
        .await
        .map_err(|e| format!("enqueue: {e}"))?;
    store
        .enqueue(&new)
        .await
        .map_err(|e| format!("re-enqueue: {e}"))?;
    let pending = store
        .pending()
        .await
        .map_err(|e| format!("pending: {e}"))?
        .len();
    let synced = store
        .mark_synced(new.client_id, "2026-09-30T12:00:05Z")
        .await
        .map_err(|e| format!("sync: {e}"))?;
    store.close().await;

    let reopened = LocalStore::open(path)
        .await
        .map_err(|e| format!("reopen: {e}"))?;
    let row = reopened
        .get(new.client_id)
        .await
        .map_err(|e| format!("get: {e}"))?;
    let still_pending = reopened
        .pending()
        .await
        .map_err(|e| format!("pending: {e}"))?
        .len();
    reopened.close().await;

    match row {
        Some(r)
            if pending == 1
                && synced
                && still_pending == 0
                && r.sync_state == "synced"
                && journal == "wal" =>
        {
            Ok(format!("ok sqlite={version} journal={journal}"))
        }
        other => Err(format!(
            "unexpected state: pending={pending} synced={synced} still_pending={still_pending} journal={journal} row={other:?}"
        )),
    }
}

/// C ABI entry point so the Android `.so` carries the whole SQLite path
/// (otherwise the linker could drop it as unreachable). Returns 0 on success.
///
/// # Safety
/// `db_path` must be a valid NUL-terminated UTF-8 path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn capsnap_spike_selftest(db_path: *const c_char) -> i32 {
    if db_path.is_null() {
        return 2;
    }
    // SAFETY: caller guarantees a valid NUL-terminated string.
    let Ok(path) = unsafe { CStr::from_ptr(db_path) }.to_str() else {
        return 2;
    };
    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return 3;
    };
    match rt.block_on(selftest(Path::new(path))) {
        Ok(_) => 0,
        Err(_) => 1,
    }
}
