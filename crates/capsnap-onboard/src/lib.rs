//! ONB-1: consent-based import from a restaurant's own website (owner
//! decision O1). A server-side crate; the W3 server exposes it as
//! `POST /api/v1/onboarding/import`.
//!
//! The result is only a **draft** for the requester to review and edit;
//! nothing is saved until they confirm. No photos are imported and raw HTML
//! is never stored. There is no third-party search (Spec §2, §6): "search"
//! means finding the menu pages inside the linked site.

mod extract;
mod fetch;
pub mod guard;
pub mod robots;

use std::fmt;
use std::net::IpAddr;
use std::time::Duration;

use serde::Serialize;

pub use guard::{is_denied_host, is_public_ip, parse_site_url};

/// Shown to the requester next to the URL field; they must accept exactly this.
pub const CONSENT_STATEMENT: &str =
    "I own or manage this website and allow NO CAP SNAP to read its public pages for this setup.";

/// Who agreed, to what, for which site, and when. Kept with the draft so the
/// server can record it (the crate itself stores nothing).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Consent {
    /// The signed-in staff member or prospect (from the session, never the form).
    pub requested_by: String,
    pub site_url: String,
    pub accepted_statement: String,
    pub accepted_at_utc: String,
}

#[derive(Debug, Clone)]
pub struct Config {
    /// robots.txt product token; matched case-insensitively (RFC 9309).
    pub product_token: String,
    pub user_agent: String,
    pub timeout: Duration,
    pub max_bytes: usize,
    pub max_redirects: usize,
    /// Home page included.
    pub max_pages: usize,
    pub allow_nonstandard_ports: bool,
    pub ip_policy: fn(IpAddr) -> bool,
    /// Fixed answers for chosen host names (tests, or pinning); still checked by `ip_policy`.
    pub resolve_overrides: Vec<(String, IpAddr)>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            product_token: "NoCapSnapBot".into(),
            user_agent:
                "NoCapSnapBot/0.1 (restaurant onboarding import, authorized by the site owner)"
                    .into(),
            timeout: Duration::from_secs(10),
            max_bytes: 2 * 1024 * 1024,
            max_redirects: 3,
            max_pages: 4,
            allow_nonstandard_ports: false,
            ip_policy: is_public_ip,
            resolve_overrides: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DraftAddress {
    pub street: Option<String>,
    pub locality: Option<String>,
    pub region: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DraftMenuItem {
    pub name: String,
    pub category: String,
    pub description: Option<String>,
    /// As published, e.g. "24.00 USD". For the reviewer; CapSnap stores no prices.
    pub price: Option<String>,
}

/// Everything found, for the requester to check. Every field may be wrong or
/// missing; the review screen shows it as editable suggestions.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    pub consent: Consent,
    pub source_url: String,
    pub business_name: Option<String>,
    pub telephone: Option<String>,
    pub website: Option<String>,
    pub address: Option<DraftAddress>,
    pub cuisine: Vec<String>,
    pub opening_hours: Vec<String>,
    pub menu: Vec<DraftMenuItem>,
    pub pages_read: Vec<String>,
    /// Plain-language notes for the reviewer (what was skipped and why).
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    NoConsent,
    InvalidUrl(&'static str),
    /// A review, delivery, social or search site rather than the restaurant's own.
    NotOwnSite,
    BlockedAddress(String),
    OffSite(String),
    RobotsDisallowed(String),
    /// robots.txt answered with a server error or not at all (RFC 9309: read nothing).
    RobotsUnavailable,
    Http(u16),
    NotHtml(String),
    TooLarge,
    TooManyRedirects,
    Timeout,
    Network(String),
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportError::NoConsent => {
                write!(f, "confirm that you own or manage this website first")
            }
            ImportError::InvalidUrl(why) => write!(f, "{why}"),
            ImportError::NotOwnSite => write!(
                f,
                "link the restaurant's own website; review, delivery and social sites can't be imported"
            ),
            ImportError::BlockedAddress(host) => write!(f, "{host} isn't a public website"),
            ImportError::OffSite(url) => write!(
                f,
                "the site sent us to another website ({url}); stopped there"
            ),
            ImportError::RobotsDisallowed(path) => {
                write!(f, "the site's robots.txt asks not to read {path}")
            }
            ImportError::RobotsUnavailable => {
                write!(
                    f,
                    "the site's robots.txt could not be read, so nothing was read; try again later"
                )
            }
            ImportError::Http(code) => write!(f, "the site answered with HTTP {code}"),
            ImportError::NotHtml(kind) => write!(f, "that address is not a web page ({kind})"),
            ImportError::TooLarge => write!(f, "the page is too large to read"),
            ImportError::TooManyRedirects => write!(f, "the site redirected too many times"),
            ImportError::Timeout => write!(f, "the site took too long to answer"),
            ImportError::Network(e) => write!(f, "could not reach the site: {e}"),
        }
    }
}

impl std::error::Error for ImportError {}

/// Reads the linked site (home page, then declared or linked menu pages) and
/// returns a draft. Checks consent and the URL before any network access.
pub async fn import(site_url: &str, consent: &Consent, cfg: &Config) -> Result<Draft, ImportError> {
    let (start, site) = parse_site_url(site_url, cfg.allow_nonstandard_ports)?;
    let consented_to = parse_site_url(&consent.site_url, cfg.allow_nonstandard_ports)
        .map(|(u, _)| u)
        .ok();
    if consent.accepted_statement != CONSENT_STATEMENT
        || consent.requested_by.trim().is_empty()
        || consent.accepted_at_utc.trim().is_empty()
        || consented_to.as_ref() != Some(&start)
    {
        return Err(ImportError::NoConsent);
    }

    let fetcher = fetch::Fetcher::new(cfg, site.clone())?;
    let robots = fetcher.robots(&start).await?;
    let home = fetcher.html(&start, &robots).await?;
    let found = extract::extract(&home.body, &home.url);
    drop(home.body); // raw HTML is not kept

    let mut notes = Vec::new();
    if let Some(source) = found
        .name_source
        .filter(|s| *s != "the site's structured data")
    {
        notes.push(format!(
            "The business name was taken from {source}; check it."
        ));
    }
    let mut pages_read = vec![home.url.to_string()];
    let mut menu = found.menu;

    // Only go looking when the home page had no menu of its own.
    if menu.is_empty() {
        for link in &found.menu_links {
            if pages_read.len() >= cfg.max_pages {
                notes.push("Stopped after the page limit; add any other dishes by hand.".into());
                break;
            }
            if !site.contains(link) {
                continue;
            }
            match fetcher.html(link, &robots).await {
                Ok(page) => {
                    let more = extract::extract(&page.body, &page.url);
                    pages_read.push(page.url.to_string());
                    for item in more.menu {
                        if !menu.iter().any(|m: &DraftMenuItem| {
                            m.name == item.name && m.category == item.category
                        }) {
                            menu.push(item);
                        }
                    }
                }
                Err(e) => notes.push(format!("{}: {e}", fetch::path_and_query(link))),
            }
        }
    }
    if menu.is_empty() {
        notes.push("No menu in a form we can read was found; add dishes by hand.".into());
    }

    Ok(Draft {
        consent: consent.clone(),
        source_url: start.to_string(),
        business_name: found.name,
        telephone: found.telephone,
        website: found.website,
        address: found.address,
        cuisine: found.cuisine,
        opening_hours: found.opening_hours,
        menu,
        pages_read,
        notes,
    })
}
