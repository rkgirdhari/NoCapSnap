#![allow(dead_code)]

use std::io::Cursor;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use capsnap_server::{AppState, Config, admin, router};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

pub const PASSWORD: &str = "correct horse battery";

pub struct World {
    pub _dir: tempfile::TempDir,
    pub state: AppState,
    pub loc_a1: String,
    pub loc_a2: String,
    pub loc_b: String,
}

/// Two restaurants on one server: "atelier" (two locations, an admin and a
/// chef who works only at the first) and "bistro" (one location, one server).
pub async fn world() -> World {
    let dir = tempfile::tempdir().unwrap();
    let cfg = Config::for_dir(dir.path(), "https://guests.example");
    let state = AppState::open(cfg).await.unwrap();
    let pool = &state.pool;
    admin::create_org(pool, "atelier", "Atelier No. 8")
        .await
        .unwrap();
    admin::create_org(pool, "bistro", "Bistro Rosa")
        .await
        .unwrap();
    let loc_a1 = admin::create_location(pool, "atelier", "Atelier No. 8", "America/Chicago")
        .await
        .unwrap();
    let loc_a2 = admin::create_location(pool, "atelier", "Atelier Annex", "America/Chicago")
        .await
        .unwrap();
    let loc_b = admin::create_location(pool, "bistro", "Bistro Rosa", "America/New_York")
        .await
        .unwrap();
    admin::create_staff(
        pool,
        "atelier",
        "ada@atelier",
        "Ada",
        "admin",
        PASSWORD,
        &[],
    )
    .await
    .unwrap();
    admin::create_staff(
        pool,
        "atelier",
        "chen@atelier",
        "Chen",
        "chef",
        PASSWORD,
        std::slice::from_ref(&loc_a1),
    )
    .await
    .unwrap();
    admin::create_staff(
        pool,
        "bistro",
        "bo@bistro",
        "Bo",
        "server",
        PASSWORD,
        std::slice::from_ref(&loc_b),
    )
    .await
    .unwrap();
    let menu = vec![
        admin::MenuEntry {
            name: "Saffron butter cod".into(),
            category: "Mains".into(),
        },
        admin::MenuEntry {
            name: "Pork belly bao".into(),
            category: "Starters".into(),
        },
    ];
    admin::import_menu(pool, &loc_a1, &menu).await.unwrap();
    admin::import_menu(
        pool,
        &loc_b,
        &[admin::MenuEntry {
            name: "Rosa's ragù".into(),
            category: "Mains".into(),
        }],
    )
    .await
    .unwrap();
    World {
        _dir: dir,
        state,
        loc_a1,
        loc_a2,
        loc_b,
    }
}

pub struct Resp {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

impl Resp {
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or(Value::Null)
    }
    pub fn error(&self) -> String {
        self.json()["error"].as_str().unwrap_or_default().to_owned()
    }
}

pub async fn call(
    app: &Router,
    method: Method,
    uri: &str,
    headers: &[(&str, &str)],
    body: Body,
) -> Resp {
    let mut req = Request::builder().method(method).uri(uri);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let resp = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = resp.status();
    let headers = resp.headers().clone();
    let body = resp
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();
    Resp {
        status,
        headers,
        body,
    }
}

pub async fn json(
    app: &Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Value,
) -> Resp {
    let auth = token.map(|t| format!("Bearer {t}"));
    let mut headers = vec![("content-type", "application/json")];
    if let Some(a) = &auth {
        headers.push(("authorization", a));
    }
    call(app, method, uri, &headers, Body::from(body.to_string())).await
}

pub async fn get(app: &Router, uri: &str, token: Option<&str>) -> Resp {
    let auth = token.map(|t| format!("Bearer {t}"));
    let headers: Vec<(&str, &str)> = auth.iter().map(|a| ("authorization", a.as_str())).collect();
    call(app, Method::GET, uri, &headers, Body::empty()).await
}

pub fn app(w: &World) -> Router {
    router(w.state.clone())
}

pub async fn sign_in(app: &Router, login: &str) -> String {
    let r = json(
        app,
        Method::POST,
        "/api/v1/auth/sessions",
        None,
        serde_json::json!({
            "login": login, "password": PASSWORD, "deviceLabel": "Pixel test"
        }),
    )
    .await;
    assert_eq!(
        r.status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    r.json()["token"].as_str().unwrap().to_owned()
}

/// A photo exactly as the phone sends it: processed by capsnap-store.
pub fn phone_jpeg(w: u32, h: u32, seed: u8) -> Vec<u8> {
    let img = image::RgbImage::from_fn(w, h, |x, y| {
        image::Rgb([(x as u8).wrapping_add(seed), y as u8, seed])
    });
    let mut raw = Cursor::new(Vec::new());
    img.write_to(&mut raw, image::ImageFormat::Jpeg).unwrap();
    capsnap_store::photo::process(&raw.into_inner(), image::ImageFormat::Jpeg)
        .unwrap()
        .jpeg
}

/// A JPEG straight from the encoder, not processed (for the size check).
pub fn raw_jpeg(w: u32, h: u32) -> Vec<u8> {
    let img = image::RgbImage::new(w, h);
    let mut raw = Cursor::new(Vec::new());
    img.write_to(&mut raw, image::ImageFormat::Jpeg).unwrap();
    raw.into_inner()
}

pub fn sha256(bytes: &[u8]) -> String {
    capsnap_server::util::sha256_hex(bytes)
}

pub async fn upload(
    app: &Router,
    token: &str,
    bytes: Vec<u8>,
    sha: &str,
    content_type: &str,
) -> Resp {
    let auth = format!("Bearer {token}");
    call(
        app,
        Method::POST,
        "/api/v1/media",
        &[
            ("authorization", &auth),
            ("content-type", content_type),
            ("x-content-sha256", sha),
        ],
        Body::from(bytes),
    )
    .await
}

pub async fn upload_ok(app: &Router, token: &str, bytes: Vec<u8>) -> String {
    let sha = sha256(&bytes);
    let r = upload(app, token, bytes, &sha, "image/jpeg").await;
    assert_eq!(
        r.status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    r.json()["mediaId"].as_str().unwrap().to_owned()
}

pub async fn menu_item(app: &Router, token: &str, location: &str, name: &str) -> String {
    let r = get(
        app,
        &format!("/api/v1/locations/{location}/menu-items"),
        Some(token),
    )
    .await;
    r.json()
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["name"] == name)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

pub async fn capture(
    app: &Router,
    token: &str,
    client_id: &str,
    media: &str,
    location: &str,
    dish: Option<&str>,
) -> Resp {
    json(
        app,
        Method::POST,
        "/api/v1/captures",
        Some(token),
        serde_json::json!({
            "clientId": client_id, "mediaId": media, "locationId": location, "menuItemId": dish,
            "captureTimeUtc": "2026-09-30T19:24:00.000Z"
        }),
    )
    .await
}

pub fn token_from(guest_url: &str) -> String {
    guest_url.split_once("/g/#").unwrap().1.to_owned()
}

pub async fn guest_session(app: &Router, token: &str) -> Resp {
    json(
        app,
        Method::POST,
        "/api/v1/guest/session",
        None,
        serde_json::json!({ "token": token }),
    )
    .await
}

pub fn cookie(resp: &Resp) -> String {
    let set = resp.headers.get("set-cookie").unwrap().to_str().unwrap();
    set.split(';').next().unwrap().to_owned()
}

pub async fn feedback(app: &Router, cookie: &str, body: Value) -> Resp {
    call(
        app,
        Method::POST,
        "/api/v1/guest/feedback",
        &[("content-type", "application/json"), ("cookie", cookie)],
        Body::from(body.to_string()),
    )
    .await
}

/// Collects everything the server logs, for the "no secrets in logs" check.
#[derive(Clone, Default)]
pub struct LogBuffer(pub Arc<Mutex<Vec<u8>>>);

impl std::io::Write for LogBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogBuffer {
    type Writer = LogBuffer;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}
