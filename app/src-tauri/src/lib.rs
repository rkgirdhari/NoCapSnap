//! CapSnap staff app — Tauri shell (W2: staff UI from the owner's mockups).
//!
//! The UI talks to the local-first store through the commands below. Photos
//! cross the IPC boundary as raw bytes in both directions (no base64), are
//! processed on the device in Rust (orientation, 2048 px, metadata stripped)
//! and land in the app's private data directory.

use std::path::PathBuf;

use capsnap_store::{
    Capture, CaptureDetails, LocalStore, MenuItem, Pragma, Setting, media_path, selftest,
    thumb_path,
};
use serde::Serialize;
use tauri::ipc::{InvokeBody, Request, Response};
use tauri::{Manager, State};

/// No sign-in until W3; real staff identities come from the server session (Spec §5).
const LOCAL_STAFF_ID: &str = "local-device";
const MAX_DISPLAY_NAME_CHARS: usize = 40;

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
    width: Option<i64>,
    height: Option<i64>,
    captured_at: String,
    sync_state: String,
    synced_at: Option<String>,
    location_id: Option<String>,
    menu_item_id: Option<String>,
    dish_name: Option<String>,
    table_label: Option<String>,
}

impl From<Capture> for CaptureDto {
    fn from(c: Capture) -> Self {
        Self {
            client_id: c.client_id,
            sha256: c.media_sha256,
            mime: c.media_mime,
            bytes: c.media_bytes,
            width: c.media_width,
            height: c.media_height,
            captured_at: c.capture_time_utc,
            sync_state: c.sync_state,
            synced_at: c.synced_at_utc,
            location_id: c.location_id,
            menu_item_id: c.menu_item_id,
            dish_name: c.dish_name,
            table_label: c.table_label,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MenuItemDto {
    id: String,
    name: String,
    category: String,
    source: String,
}

impl From<MenuItem> for MenuItemDto {
    fn from(m: MenuItem) -> Self {
        Self {
            id: m.id,
            name: m.name,
            category: m.category,
            source: m.source,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileDto {
    display_name: Option<String>,
    location_id: Option<String>,
    location_name: Option<String>,
    /// True while the profile and menu are the local demo seed (no server yet).
    is_demo: bool,
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
async fn profile_get(state: State<'_, AppState>) -> CmdResult<ProfileDto> {
    let store = &state.store;
    let location_id = store.setting(Setting::LocationId).await.map_err(text)?;
    Ok(ProfileDto {
        display_name: store
            .setting(Setting::StaffDisplayName)
            .await
            .map_err(text)?,
        location_name: store.setting(Setting::LocationName).await.map_err(text)?,
        is_demo: location_id.as_deref() == Some(capsnap_store::DEMO_LOCATION_ID),
        location_id,
    })
}

/// The name used in the Home greeting. Stored on this device only.
#[tauri::command]
async fn profile_set_name(name: String, state: State<'_, AppState>) -> CmdResult<ProfileDto> {
    let name = name.trim();
    if name.is_empty() {
        state
            .store
            .clear_setting(Setting::StaffDisplayName)
            .await
            .map_err(text)?;
    } else if name.chars().count() > MAX_DISPLAY_NAME_CHARS || name.chars().any(char::is_control) {
        return Err(format!(
            "a display name is up to {MAX_DISPLAY_NAME_CHARS} characters"
        ));
    } else {
        state
            .store
            .set_setting(Setting::StaffDisplayName, name)
            .await
            .map_err(text)?;
    }
    profile_get(state).await
}

#[tauri::command]
async fn menu_list(state: State<'_, AppState>) -> CmdResult<Vec<MenuItemDto>> {
    let Some(location_id) = state
        .store
        .setting(Setting::LocationId)
        .await
        .map_err(text)?
    else {
        return Ok(Vec::new());
    };
    let items = state.store.menu_items(&location_id).await.map_err(text)?;
    Ok(items.into_iter().map(Into::into).collect())
}

/// Headers carry the dish and table label next to the raw photo body,
/// percent-encoded by the UI (`encodeURIComponent`).
fn header(request: &Request<'_>, name: &str) -> CmdResult<Option<String>> {
    let Some(value) = request.headers().get(name) else {
        return Ok(None);
    };
    let value = value.to_str().map_err(|_| format!("{name}: not ASCII"))?;
    percent_decode(value)
        .map(Some)
        .ok_or_else(|| format!("{name}: bad percent-encoding"))
}

fn percent_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = input.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[tauri::command]
async fn capture_ingest(request: Request<'_>, state: State<'_, AppState>) -> CmdResult<CaptureDto> {
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err("expected the photo as raw bytes".into());
    };
    let menu_item_id = header(&request, "x-capsnap-menu-item")?;
    let table_label = header(&request, "x-capsnap-table-label")?;
    let details = CaptureDetails {
        menu_item_id: menu_item_id.as_deref(),
        table_label: table_label.as_deref(),
    };
    state
        .store
        .ingest(&state.media_dir, LOCAL_STAFF_ID, details, bytes.clone())
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

#[tauri::command]
async fn capture_get(
    client_id: String,
    state: State<'_, AppState>,
) -> CmdResult<Option<CaptureDto>> {
    let capture = state.store.get(&client_id).await.map_err(text)?;
    Ok(capture.map(Into::into))
}

/// Processed photo (or its thumbnail) as raw JPEG bytes. The digest is
/// checked to be 64 lowercase hex characters, so it can't name any other path.
#[tauri::command]
async fn capture_media(
    sha256: String,
    thumb: bool,
    state: State<'_, AppState>,
) -> CmdResult<Response> {
    if sha256.len() != 64
        || !sha256
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err("not a media digest".into());
    }
    let path = if thumb {
        thumb_path(&state.media_dir, &sha256)
    } else {
        media_path(&state.media_dir, &sha256, "image/jpeg")
    };
    let bytes = tauri::async_runtime::spawn_blocking(move || std::fs::read(path))
        .await
        .map_err(text)?
        .map_err(|_| "this photo is no longer on the device".to_string())?;
    Ok(Response::new(bytes))
}

/// Runs the outbox round trip inside the app's own sandbox, on a scratch
/// database next to the real one (Settings › Device check).
#[tauri::command]
async fn run_selftest(state: State<'_, AppState>) -> CmdResult<String> {
    let path = state.data_dir.join("device-check.db");
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
    }
    selftest(&path)
        .await
        .map(|summary| format!("{summary} · app sandbox"))
}

/// Debug builds only: stands in for the server acknowledging the oldest
/// pending capture, so the "QR ready" states can be reviewed before W3.
#[cfg(debug_assertions)]
#[tauri::command]
async fn simulate_ack(state: State<'_, AppState>) -> CmdResult<Option<CaptureDto>> {
    use chrono::{SecondsFormat, Utc};
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

/// Debug builds add `simulate_ack`; release bundles don't carry it (W2 scope).
#[cfg(debug_assertions)]
fn handlers() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        store_status,
        profile_get,
        profile_set_name,
        menu_list,
        capture_ingest,
        list_captures,
        capture_get,
        capture_media,
        run_selftest,
        simulate_ack
    ]
}

#[cfg(not(debug_assertions))]
fn handlers() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        store_status,
        profile_get,
        profile_set_name,
        menu_list,
        capture_ingest,
        list_captures,
        capture_get,
        capture_media,
        run_selftest
    ]
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let store = tauri::async_runtime::block_on(async {
                let store = LocalStore::open(&data_dir.join("capsnap.db")).await?;
                // First run: a clearly labelled demo location and menu (W3 replaces it).
                store.seed_demo_if_empty().await?;
                Ok::<_, capsnap_store::DbError>(store)
            })?;
            app.manage(AppState {
                store,
                media_dir: data_dir.join("media"),
                data_dir,
            });
            Ok(())
        })
        .invoke_handler(handlers())
        .run(tauri::generate_context!())
        .expect("error while running CapSnap");
}
