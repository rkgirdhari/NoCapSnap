//! CapSnap staff app — Tauri shell (W2 staff UI; W3a sign-in and sync).
//!
//! The UI talks to the local-first store through the commands below. Photos
//! are processed on the device in Rust (orientation, 2048 px, metadata
//! stripped) and land in the app's private data directory. All server traffic
//! happens here in Rust (`capsnap-sync`); the WebView never talks to the
//! server, so its CSP stays closed to the network.

use std::path::PathBuf;

use capsnap_store::{
    Capture, CaptureDetails, LocalStore, MenuItem, Pragma, Setting, media_path, photo_from_base64,
    selftest, thumb_path,
};
use serde::Serialize;
use tauri::ipc::{InvokeBody, Request, Response};
use tauri::{AppHandle, Emitter, Manager, State};

/// Staff id recorded on captures made while signed out (demo). Signed-in
/// captures record the server's staff id; the server itself only trusts the session.
const LOCAL_STAFF_ID: &str = "local-device";
const MAX_DISPLAY_NAME_CHARS: usize = 40;
/// How often the phone retries the outbox while the app is open.
const SYNC_EVERY: std::time::Duration = std::time::Duration::from_secs(60);

struct AppState {
    store: LocalStore,
    data_dir: PathBuf,
    media_dir: PathBuf,
    /// One sync run at a time (button, timer and post-capture trigger share it).
    sync_lock: tokio::sync::Mutex<()>,
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
    /// Made at the demo location: stays on this phone, never synced.
    is_demo: bool,
    guest_url: Option<String>,
    guest_expires_at: Option<String>,
    last_sync_error: Option<String>,
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
            is_demo: c
                .location_id
                .as_deref()
                .is_none_or(|l| l == capsnap_store::DEMO_LOCATION_ID),
            location_id: c.location_id,
            menu_item_id: c.menu_item_id,
            dish_name: c.dish_name,
            table_label: c.table_label,
            guest_url: c.guest_url,
            guest_expires_at: c.guest_expires_at,
            last_sync_error: c.last_sync_error,
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
    /// True while the profile and menu are the local demo seed (signed out).
    is_demo: bool,
    signed_in: bool,
    server_url: Option<String>,
    organization_name: Option<String>,
    role: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionDto {
    profile: ProfileDto,
    locations: Vec<capsnap_sync::RemoteLocation>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfoDto {
    version: String,
    /// Debug Rust core: the UI may offer developer-only tools such as `simulate_ack`.
    debug: bool,
}

type CmdResult<T> = Result<T, String>;

fn text(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
fn app_info(app: tauri::AppHandle) -> AppInfoDto {
    AppInfoDto {
        version: app.package_info().version.to_string(),
        debug: cfg!(debug_assertions),
    }
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
        signed_in: store
            .setting(Setting::SessionToken)
            .await
            .map_err(text)?
            .is_some(),
        server_url: store.setting(Setting::ServerUrl).await.map_err(text)?,
        organization_name: store
            .setting(Setting::OrganizationName)
            .await
            .map_err(text)?,
        role: store.setting(Setting::StaffRole).await.map_err(text)?,
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

/// Reads the restaurant's own website through the server and returns a draft menu to review.
/// Nothing is saved. Only an admin or manager account may; the server says so in plain words.
#[tauri::command]
async fn menu_import_preview(
    site_url: String,
    state: State<'_, AppState>,
) -> CmdResult<capsnap_sync::ImportDraft> {
    capsnap_sync::import_preview(&state.store, &site_url)
        .await
        .map_err(text)
}

/// One page of what guests said about this location's dishes. Admin or manager accounts only;
/// the server says so in plain words to anyone else.
#[tauri::command]
async fn feedback_list(
    days: i64,
    before: Option<String>,
    state: State<'_, AppState>,
) -> CmdResult<capsnap_sync::FeedbackPage> {
    capsnap_sync::feedback_page(&state.store, days, before)
        .await
        .map_err(text)
}

/// Saves the reviewed menu for this location and returns the phone's refreshed menu.
#[tauri::command]
async fn menu_import_save(
    items: Vec<capsnap_sync::MenuChoice>,
    state: State<'_, AppState>,
) -> CmdResult<Vec<MenuItemDto>> {
    capsnap_sync::save_menu(&state.store, items)
        .await
        .map_err(text)?;
    menu_list(state).await
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

/// The photo arrives as the raw IPC body where the platform supports one, or
/// as `{ "photoBase64": … }` on Android, whose WebView cannot pass a request
/// body to the app (Tauri then carries IPC over `postMessage` as text).
fn photo_bytes(body: &InvokeBody) -> CmdResult<Vec<u8>> {
    match body {
        InvokeBody::Raw(bytes) => Ok(bytes.clone()),
        InvokeBody::Json(serde_json::Value::Object(fields)) => match fields.get("photoBase64") {
            Some(serde_json::Value::String(encoded)) => photo_from_base64(encoded).map_err(text),
            _ => Err("expected photoBase64".into()),
        },
        _ => Err("expected the photo as raw bytes or base64".into()),
    }
}

#[tauri::command]
async fn capture_ingest(
    request: Request<'_>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<CaptureDto> {
    let bytes = photo_bytes(request.body())?;
    let menu_item_id = header(&request, "x-capsnap-menu-item")?;
    let table_label = header(&request, "x-capsnap-table-label")?;
    let details = CaptureDetails {
        menu_item_id: menu_item_id.as_deref(),
        table_label: table_label.as_deref(),
    };
    let staff_id = state
        .store
        .setting(Setting::StaffId)
        .await
        .map_err(text)?
        .unwrap_or_else(|| LOCAL_STAFF_ID.to_owned());
    let capture = state
        .store
        .ingest(&state.media_dir, &staff_id, details, bytes)
        .await
        .map_err(text)?;
    // Try to sync straight away; the capture is already safe on the phone.
    spawn_sync(app);
    Ok(capture.into())
}

// ---------- Sign-in and sync (W3a) ----------

#[tauri::command]
async fn session_sign_in(
    server_url: String,
    login: String,
    password: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> CmdResult<SessionDto> {
    let signed = capsnap_sync::sign_in(
        &state.store,
        &server_url,
        &login,
        &password,
        "CapSnap on Android",
    )
    .await
    .map_err(text)?;
    spawn_sync(app);
    Ok(SessionDto {
        profile: profile_get(state).await?,
        locations: signed.me.locations,
    })
}

/// Where the privacy policy lives for a given server address: the server's own `/privacy` page.
fn privacy_url(server_url: &str) -> Result<String, capsnap_sync::SyncError> {
    Ok(format!(
        "{}/privacy",
        capsnap_sync::check_server_url(server_url)?
    ))
}

/// Opens the server's privacy policy in the phone's browser. Rust opens it, from the address the
/// app is signed in to: the WebView gets no opener permission and cannot open any other address.
#[tauri::command]
async fn open_privacy_policy(app: AppHandle, state: State<'_, AppState>) -> CmdResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let server = state
        .store
        .setting(Setting::ServerUrl)
        .await
        .map_err(text)?
        .ok_or_else(|| {
            "sign in to a server first; the privacy policy is on its web address".to_string()
        })?;
    let url = privacy_url(&server).map_err(text)?;
    app.opener().open_url(url, None::<&str>).map_err(text)
}

#[tauri::command]
async fn session_sign_out(state: State<'_, AppState>) -> CmdResult<ProfileDto> {
    capsnap_sync::sign_out(&state.store).await.map_err(text)?;
    profile_get(state).await
}

/// Refreshes who this phone belongs to and the menu; lists allowed locations.
#[tauri::command]
async fn session_refresh(state: State<'_, AppState>) -> CmdResult<SessionDto> {
    let locations = capsnap_sync::refresh(&state.store).await.map_err(text)?;
    Ok(SessionDto {
        profile: profile_get(state).await?,
        locations,
    })
}

#[tauri::command]
async fn session_choose_location(
    location_id: String,
    state: State<'_, AppState>,
) -> CmdResult<ProfileDto> {
    let locations = capsnap_sync::refresh(&state.store).await.map_err(text)?;
    let location = locations
        .iter()
        .find(|l| l.id == location_id)
        .ok_or("that location isn't available to you")?;
    capsnap_sync::choose_location(&state.store, location)
        .await
        .map_err(text)?;
    profile_get(state).await
}

#[tauri::command]
async fn sync_now(app: AppHandle) -> CmdResult<capsnap_sync::SyncReport> {
    run_sync(&app).await.map_err(text)
}

async fn run_sync(app: &AppHandle) -> Result<capsnap_sync::SyncReport, capsnap_sync::SyncError> {
    let state = app.state::<AppState>();
    let _one_at_a_time = state.sync_lock.lock().await;
    let report = capsnap_sync::sync_pending(&state.store, &state.media_dir).await;
    // Screens refresh on this event after every real run (not while signed out).
    if report.is_ok() {
        let _ = app.emit("sync-updated", ());
    }
    report
}

fn spawn_sync(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let _ = run_sync(&app).await;
    });
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
        app_info,
        store_status,
        profile_get,
        profile_set_name,
        menu_list,
        menu_import_preview,
        feedback_list,
        menu_import_save,
        session_sign_in,
        open_privacy_policy,
        session_sign_out,
        session_refresh,
        session_choose_location,
        sync_now,
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
        app_info,
        store_status,
        profile_get,
        profile_set_name,
        menu_list,
        menu_import_preview,
        feedback_list,
        menu_import_save,
        session_sign_in,
        open_privacy_policy,
        session_sign_out,
        session_refresh,
        session_choose_location,
        sync_now,
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
        .plugin(tauri_plugin_opener::init())
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
                sync_lock: tokio::sync::Mutex::new(()),
            });
            // While the app is open: sync now and then every minute (a no-op when signed out).
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let _ = run_sync(&handle).await;
                    tokio::time::sleep(SYNC_EVERY).await;
                }
            });
            Ok(())
        })
        .invoke_handler(handlers())
        .run(tauri::generate_context!())
        .expect("error while running CapSnap");
}

#[cfg(test)]
mod tests {
    use super::privacy_url;

    #[test]
    fn privacy_url_is_the_servers_privacy_page() {
        assert_eq!(
            privacy_url("https://capsnap.example.com/ ").unwrap(),
            "https://capsnap.example.com/privacy"
        );
        assert_eq!(
            privacy_url("http://10.0.2.2:8080").unwrap(),
            "http://10.0.2.2:8080/privacy"
        );
    }

    #[test]
    fn privacy_url_refuses_other_addresses() {
        assert!(privacy_url("http://evil.example.com").is_err());
        assert!(privacy_url("javascript:alert(1)").is_err());
        assert!(privacy_url("").is_err());
    }
}
