//! HTTP with every limit enforced by us: redirects are followed by hand so
//! each hop is re-checked against the site, robots.txt and the IP policy.

use std::sync::Arc;

use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE, LOCATION};
use reqwest::{Client, StatusCode, redirect};
use url::Url;

use crate::guard::{BlockedAddress, GuardedResolver, Site};
use crate::robots::Robots;
use crate::{Config, ImportError};

pub struct Fetcher<'a> {
    client: Client,
    cfg: &'a Config,
    site: Site,
}

pub struct Page {
    pub url: Url,
    pub body: String,
}

impl<'a> Fetcher<'a> {
    pub fn new(cfg: &'a Config, site: Site) -> Result<Self, ImportError> {
        let client = Client::builder()
            // A proxy would resolve names itself and bypass the IP policy.
            .no_proxy()
            .dns_resolver(Arc::new(GuardedResolver {
                policy: cfg.ip_policy,
                overrides: Arc::new(cfg.resolve_overrides.clone()),
            }))
            .redirect(redirect::Policy::none())
            .timeout(cfg.timeout)
            .connect_timeout(cfg.timeout)
            .user_agent(cfg.user_agent.clone())
            .build()
            .map_err(|e| ImportError::Network(e.to_string()))?;
        Ok(Self { client, cfg, site })
    }

    /// RFC 9309 §2.3.1: 2xx → parse; 4xx → no restrictions; anything else
    /// (5xx, unreachable, too many redirects) → read nothing. A refused
    /// address is reported as such rather than as a robots problem.
    pub async fn robots(&self, origin: &Url) -> Result<Robots, ImportError> {
        let Ok(url) = origin.join("/robots.txt") else {
            return Ok(Robots::disallow_all());
        };
        // Any content type: a robots.txt served as text/html is still robots.txt.
        match self.get(&url, None, &[]).await {
            Ok(page) => Ok(Robots::parse(&page.body, &self.cfg.product_token)),
            Err(ImportError::Http(code)) if (400..500).contains(&code) => Ok(Robots::allow_all()),
            Err(e @ ImportError::BlockedAddress(_)) => Err(e),
            Err(_) => Ok(Robots::disallow_all()),
        }
    }

    pub async fn html(&self, url: &Url, robots: &Robots) -> Result<Page, ImportError> {
        self.get(url, Some(robots), &["text/html", "application/xhtml+xml"])
            .await
    }

    async fn get(
        &self,
        start: &Url,
        robots: Option<&Robots>,
        types: &[&str],
    ) -> Result<Page, ImportError> {
        let mut url = start.clone();
        for _ in 0..=self.cfg.max_redirects {
            if !self.site.contains(&url) {
                return Err(ImportError::OffSite(url.to_string()));
            }
            if !self.cfg.allow_nonstandard_ports && url.port().is_some() {
                return Err(ImportError::OffSite(url.to_string()));
            }
            if let Some(robots) = robots
                && !robots.allows(&path_and_query(&url))
            {
                return Err(if robots.is_unavailable() {
                    ImportError::RobotsUnavailable
                } else {
                    ImportError::RobotsDisallowed(path_and_query(&url))
                });
            }
            let mut resp = self
                .client
                .get(url.clone())
                .send()
                .await
                .map_err(classify)?;
            let status = resp.status();
            if status.is_redirection() && status != StatusCode::NOT_MODIFIED {
                let next = resp
                    .headers()
                    .get(LOCATION)
                    .and_then(|l| l.to_str().ok())
                    .and_then(|l| url.join(l).ok())
                    .ok_or(ImportError::Http(status.as_u16()))?;
                url = next;
                continue;
            }
            if !status.is_success() {
                return Err(ImportError::Http(status.as_u16()));
            }
            let content_type = resp
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_ascii_lowercase();
            let essence = content_type.split(';').next().unwrap_or("").trim();
            if !types.is_empty() && !types.contains(&essence) {
                return Err(ImportError::NotHtml(essence.to_owned()));
            }
            let declared = resp
                .headers()
                .get(CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<usize>().ok());
            if declared.is_some_and(|n| n > self.cfg.max_bytes) {
                return Err(ImportError::TooLarge);
            }
            let mut body = Vec::new();
            while let Some(chunk) = resp.chunk().await.map_err(classify)? {
                if body.len() + chunk.len() > self.cfg.max_bytes {
                    return Err(ImportError::TooLarge);
                }
                body.extend_from_slice(&chunk);
            }
            return Ok(Page {
                url,
                body: String::from_utf8_lossy(&body).into_owned(),
            });
        }
        Err(ImportError::TooManyRedirects)
    }
}

pub fn path_and_query(url: &Url) -> String {
    match url.query() {
        Some(q) => format!("{}?{q}", url.path()),
        None => url.path().to_owned(),
    }
}

fn classify(e: reqwest::Error) -> ImportError {
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(&e);
    while let Some(err) = source {
        if let Some(blocked) = err.downcast_ref::<BlockedAddress>() {
            return ImportError::BlockedAddress(blocked.0.clone());
        }
        source = err.source();
    }
    if e.is_timeout() {
        ImportError::Timeout
    } else {
        ImportError::Network(e.to_string())
    }
}
