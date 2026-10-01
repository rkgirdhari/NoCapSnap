//! A tiny HTTP/1.1 server on 127.0.0.1 for the importer tests. Routes are
//! keyed by "host/path"; every request is recorded so tests can prove what
//! was (and wasn't) fetched.
#![allow(dead_code)]

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use capsnap_onboard::{CONSENT_STATEMENT, Config, Consent};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[derive(Clone, Default)]
pub struct Route {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub hang: bool,
    /// Omit Content-Length (body ends when the connection closes).
    pub no_length: bool,
}

pub fn html(body: &str) -> Route {
    Route {
        status: 200,
        headers: vec![("Content-Type".into(), "text/html; charset=utf-8".into())],
        body: body.into(),
        ..Default::default()
    }
}

pub fn text(status: u16, body: &str) -> Route {
    Route {
        status,
        headers: vec![("Content-Type".into(), "text/plain".into())],
        body: body.into(),
        ..Default::default()
    }
}

pub fn redirect(to: &str) -> Route {
    Route {
        status: 302,
        headers: vec![("Location".into(), to.into())],
        ..Default::default()
    }
}

pub struct Server {
    pub port: u16,
    hits: Arc<Mutex<Vec<String>>>,
}

impl Server {
    pub async fn start(routes: Vec<(&str, Route)>) -> Self {
        let routes: Arc<HashMap<String, Route>> =
            Arc::new(routes.into_iter().map(|(k, v)| (k.to_owned(), v)).collect());
        let hits = Arc::new(Mutex::new(Vec::new()));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (r, h) = (routes.clone(), hits.clone());
        tokio::spawn(async move {
            loop {
                let Ok((mut conn, _)) = listener.accept().await else {
                    return;
                };
                let (routes, hits) = (r.clone(), h.clone());
                tokio::spawn(async move {
                    let mut buf = Vec::new();
                    let mut chunk = [0u8; 4096];
                    while !buf.windows(4).any(|w| w == b"\r\n\r\n") && buf.len() < 16384 {
                        match conn.read(&mut chunk).await {
                            Ok(0) | Err(_) => return,
                            Ok(n) => buf.extend_from_slice(&chunk[..n]),
                        }
                    }
                    let head = String::from_utf8_lossy(&buf).into_owned();
                    let path = head.split_whitespace().nth(1).unwrap_or("/").to_owned();
                    let host = head
                        .lines()
                        .find_map(|l| {
                            l.strip_prefix("host: ")
                                .or_else(|| l.strip_prefix("Host: "))
                        })
                        .unwrap_or("")
                        .split(':')
                        .next()
                        .unwrap_or("")
                        .to_owned();
                    let key = format!("{host}{path}");
                    hits.lock().unwrap().push(key.clone());
                    let route = routes
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(|| text(404, "not found"));
                    if route.hang {
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        return;
                    }
                    let mut out = format!("HTTP/1.1 {} X\r\nConnection: close\r\n", route.status);
                    if !route.no_length {
                        out.push_str(&format!("Content-Length: {}\r\n", route.body.len()));
                    }
                    for (k, v) in &route.headers {
                        out.push_str(&format!("{k}: {v}\r\n"));
                    }
                    out.push_str("\r\n");
                    let _ = conn.write_all(out.as_bytes()).await;
                    let _ = conn.write_all(&route.body).await;
                    let _ = conn.shutdown().await;
                });
            }
        });
        Self { port, hits }
    }

    pub fn url(&self, host: &str, path: &str) -> String {
        format!("http://{host}:{}{path}", self.port)
    }

    pub fn hits(&self) -> Vec<String> {
        self.hits.lock().unwrap().clone()
    }
}

const LOCAL: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

/// Test hosts pinned to the local server. Loopback is allowed here on
/// purpose; `the_default_policy_refuses_*` proves production refuses it.
pub fn config() -> Config {
    Config {
        allow_nonstandard_ports: true,
        ip_policy: |ip| ip.is_loopback(),
        resolve_overrides: [
            "atelier8.test",
            "www.atelier8.test",
            "menu.atelier8.test",
            "evil.test",
        ]
        .into_iter()
        .map(|h| (h.to_owned(), LOCAL))
        .collect(),
        timeout: Duration::from_secs(3),
        ..Config::default()
    }
}

pub fn consent(site_url: &str) -> Consent {
    Consent {
        requested_by: "staff-7".into(),
        site_url: site_url.into(),
        accepted_statement: CONSENT_STATEMENT.into(),
        accepted_at_utc: "2026-09-30T19:00:00.000Z".into(),
    }
}
