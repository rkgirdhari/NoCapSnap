//! Phone-side sync (W3a): sign in to the restaurant's server, keep the menu
//! cache current, and move the outbox (Spec §3) to the server:
//! photo upload (once) → capture post → acknowledgement with the guest link.
//!
//! All HTTP is blocking (`ureq`) and runs on Tokio's blocking pool, so callers
//! stay async. The WebView never talks to the server (its CSP stays closed).

use std::path::Path;
use std::time::Duration;

use capsnap_store::{Ack, Capture, LocalStore, MenuSource, NewMenuItem, Setting, media_path};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

// ---------- Errors ----------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncError {
    /// The address isn't one the app will send staff credentials to.
    BadServerUrl(String),
    /// No answer from the server (no network, server down, timeout).
    Offline(String),
    /// The session is missing, expired or revoked: sign in again.
    SignedOut,
    /// Wrong sign-in name or password, or too many attempts.
    SignInRefused(String),
    /// The server refused this item (4xx); retrying the same request won't help.
    Refused {
        status: u16,
        code: String,
        message: String,
    },
    /// The server failed (5xx); try again later.
    Server(String),
    /// Something on this phone (database, file).
    Local(String),
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::BadServerUrl(why) => write!(f, "{why}"),
            SyncError::Offline(_) => write!(f, "can't reach the restaurant's server right now"),
            SyncError::SignedOut => write!(f, "sign in again to sync"),
            SyncError::SignInRefused(msg) => write!(f, "{msg}"),
            SyncError::Refused { message, .. } => write!(f, "{message}"),
            SyncError::Server(_) => {
                write!(f, "the restaurant's server had a problem; it will retry")
            }
            SyncError::Local(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for SyncError {}

impl From<capsnap_store::DbError> for SyncError {
    fn from(e: capsnap_store::DbError) -> Self {
        SyncError::Local(e.to_string())
    }
}

// ---------- HTTP client ----------

/// A server address the app may use: `https://…`, or plain http only to this
/// phone itself or the Android emulator's host (development).
pub fn check_server_url(raw: &str) -> Result<String, SyncError> {
    let url = raw.trim().trim_end_matches('/');
    let host_of = |rest: &str| {
        rest.split(['/', ':'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase()
    };
    if let Some(rest) = url.strip_prefix("https://") {
        if !host_of(rest).is_empty() {
            return Ok(url.to_owned());
        }
    } else if let Some(rest) = url.strip_prefix("http://") {
        if matches!(
            host_of(rest).as_str(),
            "127.0.0.1" | "localhost" | "10.0.2.2"
        ) {
            return Ok(url.to_owned());
        }
        return Err(SyncError::BadServerUrl(
            "the server address must start with https://".into(),
        ));
    }
    Err(SyncError::BadServerUrl(
        "enter the server address, e.g. https://capsnap.example.com".into(),
    ))
}

#[derive(Clone)]
pub struct Server {
    base: String,
    agent: ureq::Agent,
}

#[derive(Deserialize)]
struct ApiErr {
    error: String,
    message: String,
}

impl Server {
    pub fn new(url: &str) -> Result<Self, SyncError> {
        let base = check_server_url(url)?;
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(60)))
            .timeout_connect(Some(Duration::from_secs(10)))
            .http_status_as_error(false)
            .https_only(!base.starts_with("http://"))
            .build()
            .into();
        Ok(Self { base, agent })
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api/v1{path}", self.base)
    }

    fn finish<T: for<'de> Deserialize<'de>>(
        result: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
    ) -> Result<T, SyncError> {
        let mut resp = result.map_err(|e| SyncError::Offline(e.to_string()))?;
        let status = resp.status().as_u16();
        if (200..300).contains(&status) {
            return resp
                .body_mut()
                .read_json::<T>()
                .map_err(|e| SyncError::Server(format!("unexpected answer: {e}")));
        }
        let err = resp.body_mut().read_json::<ApiErr>().unwrap_or(ApiErr {
            error: "unknown".into(),
            message: format!("HTTP {status}"),
        });
        Err(match status {
            401 if err.error == "bad_credentials" => SyncError::SignInRefused(err.message),
            401 => SyncError::SignedOut,
            429 => SyncError::SignInRefused(err.message),
            400..=499 => SyncError::Refused {
                status,
                code: err.error,
                message: err.message,
            },
            _ => SyncError::Server(format!("HTTP {status}: {}", err.message)),
        })
    }

    pub fn sign_in(
        &self,
        login: &str,
        password: &str,
        device_label: &str,
    ) -> Result<SignedIn, SyncError> {
        let body = serde_json::json!({ "login": login, "password": password, "deviceLabel": device_label });
        Self::finish(self.agent.post(&self.url("/auth/sessions")).send_json(body))
    }

    pub fn sign_out(&self, token: &str) -> Result<(), SyncError> {
        let resp = self
            .agent
            .delete(&self.url("/auth/sessions/current"))
            .header("authorization", &format!("Bearer {token}"))
            .call()
            .map_err(|e| SyncError::Offline(e.to_string()))?;
        match resp.status().as_u16() {
            204 | 401 => Ok(()),
            s => Err(SyncError::Server(format!("HTTP {s}"))),
        }
    }

    pub fn me(&self, token: &str) -> Result<Me, SyncError> {
        Self::finish(
            self.agent
                .get(&self.url("/me"))
                .header("authorization", &format!("Bearer {token}"))
                .call(),
        )
    }

    pub fn menu(&self, token: &str, location_id: &str) -> Result<Vec<RemoteMenuItem>, SyncError> {
        Self::finish(
            self.agent
                .get(&self.url(&format!("/locations/{location_id}/menu-items")))
                .header("authorization", &format!("Bearer {token}"))
                .call(),
        )
    }

    /// Asks the server to read the restaurant's own website and draft a menu. Nothing is saved.
    pub fn import_site(&self, token: &str, site_url: &str) -> Result<ImportDraft, SyncError> {
        let body = serde_json::json!({ "siteUrl": site_url, "acceptedStatement": IMPORT_CONSENT });
        Self::finish(
            self.agent
                .post(&self.url("/onboarding/import"))
                .header("authorization", &format!("Bearer {token}"))
                .send_json(body),
        )
    }

    /// Saves the reviewed menu for a location, in order. The old menu is retired, not deleted.
    pub fn replace_menu(
        &self,
        token: &str,
        location_id: &str,
        items: &[MenuChoice],
    ) -> Result<Vec<RemoteMenuItem>, SyncError> {
        let body = serde_json::json!({ "items": items });
        Self::finish(
            self.agent
                .put(&self.url(&format!("/locations/{location_id}/menu-items")))
                .header("authorization", &format!("Bearer {token}"))
                .send_json(body),
        )
    }

    pub fn upload(&self, token: &str, jpeg: &[u8], sha256: &str) -> Result<String, SyncError> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Media {
            media_id: String,
        }
        let media: Media = Self::finish(
            self.agent
                .post(&self.url("/media"))
                .header("authorization", &format!("Bearer {token}"))
                .header("content-type", "image/jpeg")
                .header("x-content-sha256", sha256)
                .send(jpeg),
        )?;
        Ok(media.media_id)
    }

    pub fn capture(&self, token: &str, req: &CaptureRequest<'_>) -> Result<CaptureAck, SyncError> {
        Self::finish(
            self.agent
                .post(&self.url("/captures"))
                .header("authorization", &format!("Bearer {token}"))
                .send_json(req),
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedIn {
    pub token: String,
    pub expires_at: String,
    #[serde(flatten)]
    pub me: Me,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Me {
    pub staff: RemoteStaff,
    pub organization: RemoteOrg,
    pub locations: Vec<RemoteLocation>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteStaff {
    pub id: String,
    pub display_name: String,
    pub role: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteOrg {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteLocation {
    pub id: String,
    pub name: String,
    pub timezone: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteMenuItem {
    pub id: String,
    pub name: String,
    pub category: String,
}

/// A dish as saved: a name and a course.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MenuChoice {
    pub name: String,
    pub category: String,
}

/// What the restaurant must accept before the server reads its website (ONB-1). The server compares
/// this word for word, and a test in `tests/menu_import.rs` keeps the two copies equal.
pub const IMPORT_CONSENT: &str =
    "I own or manage this website and allow NO CAP SNAP to read its public pages for this setup.";

/// One dish found on the restaurant's website. Prices and descriptions are shown to the reviewer
/// only: CapSnap keeps a name and a course.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportedDish {
    pub name: String,
    pub category: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub price: Option<String>,
}

/// The server's draft menu, for the owner to check and edit before anything is saved.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportDraft {
    pub business_name: Option<String>,
    pub menu: Vec<ImportedDish>,
    pub pages_read: Vec<String>,
    pub notes: Vec<String>,
}

/// Only what the server needs. The table label is device-only (owner default
/// M6) and is never sent; the staff member comes from the session.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRequest<'a> {
    pub client_id: &'a str,
    pub media_id: &'a str,
    pub location_id: &'a str,
    pub menu_item_id: Option<&'a str>,
    pub capture_time_utc: &'a str,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureAck {
    pub capture_id: String,
    pub received_at: String,
    pub guest_url: Option<String>,
    pub guest_expires_at: Option<String>,
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, SyncError> + Send + 'static,
) -> Result<T, SyncError> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| SyncError::Local(e.to_string()))?
}

fn now_utc() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

// ---------- Session on this phone ----------

/// The stored session, if this phone is signed in.
pub async fn session(store: &LocalStore) -> Result<Option<(Server, String)>, SyncError> {
    let (Some(url), Some(token)) = (
        store.setting(Setting::ServerUrl).await?,
        store.setting(Setting::SessionToken).await?,
    ) else {
        return Ok(None);
    };
    Ok(Some((Server::new(&url)?, token)))
}

/// Signs in and makes this phone the server's: staff name, organization, a
/// location (the current one if still allowed, else the first), and its menu.
pub async fn sign_in(
    store: &LocalStore,
    server_url: &str,
    login: &str,
    password: &str,
    device_label: &str,
) -> Result<SignedIn, SyncError> {
    let server = Server::new(server_url)?;
    let (s, l, p, d) = (
        server.clone(),
        login.to_owned(),
        password.to_owned(),
        device_label.to_owned(),
    );
    let signed = blocking(move || s.sign_in(&l, &p, &d)).await?;
    if signed.me.locations.is_empty() {
        let (s, t) = (server.clone(), signed.token.clone());
        let _ = blocking(move || s.sign_out(&t)).await;
        return Err(SyncError::SignInRefused(
            "this account has no location yet; ask your manager".into(),
        ));
    }
    store.set_setting(Setting::ServerUrl, server.base()).await?;
    store
        .set_setting(Setting::SessionToken, &signed.token)
        .await?;
    store
        .set_setting(Setting::SessionExpiresAt, &signed.expires_at)
        .await?;
    apply_me(store, &signed.me).await?;
    refresh_menu(store).await?;
    Ok(signed)
}

async fn apply_me(store: &LocalStore, me: &Me) -> Result<(), SyncError> {
    store.set_setting(Setting::StaffId, &me.staff.id).await?;
    store
        .set_setting(Setting::StaffRole, &me.staff.role)
        .await?;
    store
        .set_setting(Setting::StaffDisplayName, &me.staff.display_name)
        .await?;
    store
        .set_setting(Setting::OrganizationName, &me.organization.name)
        .await?;
    let current = store.setting(Setting::LocationId).await?;
    let location = me
        .locations
        .iter()
        .find(|l| Some(&l.id) == current.as_ref())
        .or_else(|| me.locations.first())
        .ok_or_else(|| SyncError::SignInRefused("this account has no location".into()))?;
    store.set_setting(Setting::LocationId, &location.id).await?;
    store
        .set_setting(Setting::LocationName, &location.name)
        .await?;
    Ok(())
}

/// Re-reads who we are and the current location's menu (on app start, after
/// switching location). Signs this phone out locally if the server says so.
pub async fn refresh(store: &LocalStore) -> Result<Vec<RemoteLocation>, SyncError> {
    let Some((server, token)) = session(store).await? else {
        return Err(SyncError::SignedOut);
    };
    let me = match blocking(move || server.me(&token)).await {
        Err(SyncError::SignedOut) => {
            forget_session(store).await?;
            return Err(SyncError::SignedOut);
        }
        other => other?,
    };
    apply_me(store, &me).await?;
    refresh_menu(store).await?;
    Ok(me.locations)
}

pub async fn choose_location(
    store: &LocalStore,
    location: &RemoteLocation,
) -> Result<(), SyncError> {
    store.set_setting(Setting::LocationId, &location.id).await?;
    store
        .set_setting(Setting::LocationName, &location.name)
        .await?;
    refresh_menu(store).await
}

async fn refresh_menu(store: &LocalStore) -> Result<(), SyncError> {
    let (Some((server, token)), Some(location)) = (
        session(store).await?,
        store.setting(Setting::LocationId).await?,
    ) else {
        return Ok(());
    };
    let loc = location.clone();
    let items = blocking(move || server.menu(&token, &loc)).await?;
    let rows: Vec<NewMenuItem<'_>> = items
        .iter()
        .map(|i| NewMenuItem {
            id: &i.id,
            name: &i.name,
            category: &i.category,
            is_active: true,
        })
        .collect();
    store
        .replace_menu(&location, MenuSource::Server, &rows)
        .await?;
    Ok(())
}

/// Reads the restaurant's website through the server and returns a draft menu. Only an admin or
/// manager may; a server refusal (not your own site, robots.txt, no menu found) comes back as the
/// server's own plain-language message.
pub async fn import_preview(store: &LocalStore, site_url: &str) -> Result<ImportDraft, SyncError> {
    let (server, token) = session(store).await?.ok_or(SyncError::SignedOut)?;
    let url = site_url.trim().to_owned();
    blocking(move || server.import_site(&token, &url)).await
}

/// Saves the reviewed menu for the current location, then refreshes this phone's copy of it.
pub async fn save_menu(store: &LocalStore, items: Vec<MenuChoice>) -> Result<(), SyncError> {
    let (server, token) = session(store).await?.ok_or(SyncError::SignedOut)?;
    let location = store
        .setting(Setting::LocationId)
        .await?
        .ok_or_else(|| SyncError::Local("no location is selected on this phone".into()))?;
    blocking(move || server.replace_menu(&token, &location, &items).map(|_| ())).await?;
    refresh_menu(store).await
}

/// Signs out on the server (best effort) and here. Captures stay on the phone;
/// the app goes back to the labelled demo location.
pub async fn sign_out(store: &LocalStore) -> Result<(), SyncError> {
    if let Some((server, token)) = session(store).await? {
        let _ = blocking(move || server.sign_out(&token)).await;
    }
    forget_session(store).await
}

async fn forget_session(store: &LocalStore) -> Result<(), SyncError> {
    for key in [
        Setting::SessionToken,
        Setting::SessionExpiresAt,
        Setting::StaffId,
        Setting::StaffRole,
        Setting::OrganizationName,
        Setting::LocationId,
        Setting::LocationName,
    ] {
        store.clear_setting(key).await?;
    }
    store.seed_demo_if_empty().await?;
    Ok(())
}

// ---------- Outbox ----------

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub synced: usize,
    pub refused: usize,
    pub remaining: usize,
    /// Why the run stopped early (offline, signed out, server error), if it did.
    pub stopped: Option<String>,
}

/// Sends every syncable pending capture, oldest first. A network, session or
/// server problem stops the run (the rest would fail the same way); a capture
/// the server refuses is recorded and skipped.
pub async fn sync_pending(store: &LocalStore, media_dir: &Path) -> Result<SyncReport, SyncError> {
    let Some((server, token)) = session(store).await? else {
        return Err(SyncError::SignedOut);
    };
    let mut report = SyncReport::default();
    let pending = store.syncable_pending().await?;
    for capture in &pending {
        match sync_one(store, media_dir, &server, &token, capture).await {
            Ok(()) => report.synced += 1,
            Err(e) => {
                store
                    .record_sync_error(&capture.client_id, &e.to_string(), &now_utc())
                    .await?;
                match e {
                    SyncError::Refused { .. } => report.refused += 1,
                    SyncError::SignedOut => {
                        forget_session(store).await?;
                        report.stopped = Some(e.to_string());
                        break;
                    }
                    _ => {
                        report.stopped = Some(e.to_string());
                        break;
                    }
                }
            }
        }
    }
    report.remaining = store.syncable_pending().await?.len();
    Ok(report)
}

async fn sync_one(
    store: &LocalStore,
    media_dir: &Path,
    server: &Server,
    token: &str,
    capture: &Capture,
) -> Result<(), SyncError> {
    let location = capture
        .location_id
        .clone()
        .ok_or_else(|| SyncError::Local("capture has no location".into()))?;
    let mut media_id = capture.remote_media_id.clone();
    for attempt in 0..2 {
        let id = match &media_id {
            Some(id) => id.clone(),
            None => {
                let path = media_path(
                    media_dir,
                    &capture.media_sha256,
                    capture.media_mime.as_deref().unwrap_or("image/jpeg"),
                );
                let bytes = std::fs::read(&path)
                    .map_err(|e| SyncError::Local(format!("photo missing on this phone: {e}")))?;
                let (s, t, sha) = (
                    server.clone(),
                    token.to_owned(),
                    capture.media_sha256.clone(),
                );
                let id = blocking(move || s.upload(&t, &bytes, &sha)).await?;
                store
                    .set_remote_media(&capture.client_id, Some(&id))
                    .await?;
                id
            }
        };
        let (s, t, c) = (server.clone(), token.to_owned(), capture.clone());
        let (loc, mid) = (location.clone(), id.clone());
        let result = blocking(move || {
            s.capture(
                &t,
                &CaptureRequest {
                    client_id: &c.client_id,
                    media_id: &mid,
                    location_id: &loc,
                    menu_item_id: c.menu_item_id.as_deref(),
                    capture_time_utc: &c.capture_time_utc,
                },
            )
        })
        .await;
        match result {
            Ok(ack) => {
                store
                    .record_ack(
                        &capture.client_id,
                        &Ack {
                            synced_at_utc: &ack.received_at,
                            server_capture_id: &ack.capture_id,
                            guest_url: ack.guest_url.as_deref(),
                            guest_expires_at: ack.guest_expires_at.as_deref(),
                        },
                    )
                    .await?;
                return Ok(());
            }
            // The server no longer has our upload (its retention removes photos
            // never attached to a capture): upload again, once.
            Err(SyncError::Refused {
                status: 404,
                ref message,
                ..
            }) if attempt == 0 && message.starts_with("photo") => {
                store.set_remote_media(&capture.client_id, None).await?;
                media_id = None;
            }
            Err(e) => return Err(e),
        }
    }
    Err(SyncError::Local(
        "the photo could not be re-uploaded".into(),
    ))
}
