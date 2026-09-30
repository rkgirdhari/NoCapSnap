//! CapSnap staff app — W1 Tauri shell.
//!
//! The UI talks to the local-first store through five commands. Photos cross
//! the IPC boundary as raw bytes (no base64), are validated and hashed in
//! Rust, and land in the app's private data directory.

use std::path::PathBuf;

use capsnap_store_spike::{Capture, LocalStore, Pragma, selftest};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use tauri::ipc::{InvokeBody, Request};
use tauri::{Manager, State};

/// W1 has no sign-in yet; real staff identities come from the server session (Spec §5).
const SPIKE_STAFF_ID: &str = "w1-spike-device";

struct AppState {
    store: LocalStore,
    data_dir: PathBuf,
    media_dir: PathBuf,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureDto {
    client_id: String,
    sha256: String,
    mime: Option<String>,
    bytes: Option<i64>,
    captured_at: String,
    sync_state: String,
    synced_at: Option<String>,
}

impl From<Capture> for CaptureDto {
    fn from(c: Capture) -> Self {
        Self {
            client_id: c.client_id,
            sha256: c.media_sha256,
            mime: c.media_mime,
            bytes: c.media_bytes,
            captured_at: c.capture_time_utc,
            sync_state: c.sync_state,
            synced_at: c.synced_at_utc,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StoreStatusDto {
    sqlite_version: String,
    journal_mode: String,
    pending: i64,
    synced: i64,
}

type CmdResult<T> = Result<T, String>;

fn text(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
async fn store_status(state: State<'_, AppState>) -> CmdResult<StoreStatusDto> {
    let counts = state.store.counts().await.map_err(text)?;
    Ok(StoreStatusDto {
        sqlite_version: state.store.sqlite_version().await.map_err(text)?,
        journal_mode: state
            .store
            .pragma(Pragma::JournalMode)
            .await
            .map_err(text)?,
        pending: counts.pending,
        synced: counts.synced,
    })
}

#[tauri::command]
async fn capture_ingest(request: Request<'_>, state: State<'_, AppState>) -> CmdResult<CaptureDto> {
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err("expected the photo as raw bytes".into());
    };
    state
        .store
        .ingest(&state.media_dir, SPIKE_STAFF_ID, bytes)
        .await
        .map(Into::into)
        .map_err(text)
}

#[tauri::command]
async fn list_captures(
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> CmdResult<Vec<CaptureDto>> {
    let limit = limit.unwrap_or(200).clamp(1, 1000);
    let captures = state.store.list_recent(limit).await.map_err(text)?;
    Ok(captures.into_iter().map(Into::into).collect())
}

/// Runs the W0 outbox round trip inside the app's own sandbox, on a scratch
/// database next to the real one.
#[tauri::command]
async fn run_selftest(state: State<'_, AppState>) -> CmdResult<String> {
    let path = state.data_dir.join("w1-selftest.db");
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
    }
    selftest(&path)
        .await
        .map(|summary| format!("{summary} · app sandbox"))
}

/// Spike only: stands in for the server acknowledging the oldest pending capture.
#[tauri::command]
async fn simulate_ack(state: State<'_, AppState>) -> CmdResult<Option<CaptureDto>> {
    let Some(oldest) = state
        .store
        .pending()
        .await
        .map_err(text)?
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    state
        .store
        .mark_synced(&oldest.client_id, &now)
        .await
        .map_err(text)?;
    let updated = state.store.get(&oldest.client_id).await.map_err(text)?;
    Ok(updated.map(Into::into))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let store =
                tauri::async_runtime::block_on(LocalStore::open(&data_dir.join("capsnap.db")))?;
            app.manage(AppState {
                store,
                media_dir: data_dir.join("media"),
                data_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            store_status,
            capture_ingest,
            list_captures,
            run_selftest,
            simulate_ack
        ])
        .run(tauri::generate_context!())
        .expect("error while running CapSnap");
}
