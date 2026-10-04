use std::net::SocketAddr;
use std::path::PathBuf;

/// Read from the environment (`CAPSNAP_*`); see the server README.
#[derive(Debug, Clone)]
pub struct Config {
    pub db_path: PathBuf,
    pub media_dir: PathBuf,
    pub bind: SocketAddr,
    /// Where guest links point, e.g. `https://nocapsnap.example` (owner decision G1).
    pub public_base_url: String,
    /// `Secure` on the guest cookie. Only turned off for plain-http tests.
    pub cookie_secure: bool,
    pub session_days: i64,
    pub guest_link_days: i64,
    pub guest_session_minutes: i64,
    pub max_media_bytes: usize,
    /// Who a privacy request goes to (shown on `/privacy`); `CAPSNAP_PRIVACY_CONTACT`.
    pub privacy_contact: Option<String>,
}

impl Config {
    pub fn for_dir(dir: &std::path::Path, public_base_url: &str) -> Self {
        Self {
            db_path: dir.join("capsnap-server.db"),
            media_dir: dir.join("media"),
            bind: "127.0.0.1:8080".parse().unwrap(),
            public_base_url: public_base_url.trim_end_matches('/').to_owned(),
            cookie_secure: true,
            session_days: 30,
            guest_link_days: 30,
            guest_session_minutes: 30,
            max_media_bytes: 10 * 1024 * 1024,
            privacy_contact: None,
        }
    }

    pub fn from_env() -> Result<Self, String> {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        let data = PathBuf::from(var("CAPSNAP_DATA_DIR").unwrap_or_else(|| "./data".into()));
        let base = var("CAPSNAP_PUBLIC_BASE_URL")
            .ok_or("set CAPSNAP_PUBLIC_BASE_URL (where guest links point)")?;
        if !(base.starts_with("https://")
            || base.starts_with("http://127.0.0.1")
            || base.starts_with("http://localhost"))
        {
            return Err(
                "CAPSNAP_PUBLIC_BASE_URL must be https (plain http only for localhost)".into(),
            );
        }
        let mut cfg = Self::for_dir(&data, &base);
        if let Some(bind) = var("CAPSNAP_BIND") {
            cfg.bind = bind
                .parse()
                .map_err(|_| format!("CAPSNAP_BIND is not an address: {bind}"))?;
        }
        cfg.cookie_secure = var("CAPSNAP_INSECURE_COOKIES").is_none();
        cfg.privacy_contact = var("CAPSNAP_PRIVACY_CONTACT");
        Ok(cfg)
    }
}
