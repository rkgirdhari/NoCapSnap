//! What the importer may touch: which URLs, which sites, which IP addresses.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use url::{Host, Url};

use crate::ImportError;

/// Review aggregators, delivery platforms, social networks and search/maps.
/// Spec §1: no review data; and these are not the restaurant's own website.
/// Matched against any host label except the top-level one ("yelp" matches
/// `www.yelp.co.uk`, not `yelpcafe.com`).
const DENIED_LABELS: &[&str] = &[
    "google",
    "yelp",
    "tripadvisor",
    "opentable",
    "doordash",
    "ubereats",
    "grubhub",
    "seamless",
    "postmates",
    "deliveroo",
    "justeat",
    "zomato",
    "thefork",
    "resy",
    "tock",
    "foursquare",
    "trustpilot",
    "facebook",
    "instagram",
    "tiktok",
    "twitter",
];
/// Short links and hosts whose names don't carry the brand as a label.
const DENIED_HOSTS: &[&str] = &[
    "goo.gl",
    "g.page",
    "g.co",
    "fb.com",
    "fb.me",
    "instagr.am",
    "x.com",
];

/// A site is the given host without a leading `www.`, plus its subdomains, so
/// `menu.atelier8.com` belongs to `www.atelier8.com` but `atelier8.com.evil.io`
/// does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    base: String,
}

impl Site {
    pub fn contains(&self, url: &Url) -> bool {
        matches!(url.scheme(), "http" | "https")
            && match url.host() {
                Some(Host::Domain(host)) => {
                    host == self.base
                        || host
                            .strip_suffix(&self.base)
                            .is_some_and(|prefix| prefix.ends_with('.'))
                }
                _ => false,
            }
    }

    pub fn base(&self) -> &str {
        &self.base
    }
}

/// Normalises what a person typed ("atelier8.com", " https://www.atelier8.com/ ")
/// into the start URL and its site, refusing anything that isn't a plain
/// public website address.
pub fn parse_site_url(
    raw: &str,
    allow_nonstandard_ports: bool,
) -> Result<(Url, Site), ImportError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(ImportError::InvalidUrl(
            "enter the restaurant's website address",
        ));
    }
    let with_scheme = if raw.contains("://") {
        raw.to_owned()
    } else {
        format!("https://{raw}")
    };
    let mut url = Url::parse(&with_scheme)
        .map_err(|_| ImportError::InvalidUrl("that isn't a web address"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(ImportError::InvalidUrl(
            "only http and https addresses can be read",
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(ImportError::InvalidUrl(
            "the address must not contain a login",
        ));
    }
    let host = match url.host() {
        Some(Host::Domain(host)) => host.to_ascii_lowercase(),
        Some(_) => {
            return Err(ImportError::InvalidUrl(
                "use the website's name, not an IP address",
            ));
        }
        None => return Err(ImportError::InvalidUrl("that isn't a web address")),
    };
    if !host.contains('.') {
        return Err(ImportError::InvalidUrl(
            "use the website's full name, such as atelier8.com",
        ));
    }
    if !allow_nonstandard_ports && url.port().is_some() {
        return Err(ImportError::InvalidUrl(
            "only standard web ports can be read",
        ));
    }
    if is_denied_host(&host) {
        return Err(ImportError::NotOwnSite);
    }
    url.set_fragment(None);
    let base = host.strip_prefix("www.").unwrap_or(&host).to_owned();
    Ok((url, Site { base }))
}

pub fn is_denied_host(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    if DENIED_HOSTS
        .iter()
        .any(|d| host == *d || host.ends_with(&format!(".{d}")))
    {
        return true;
    }
    let labels: Vec<&str> = host.split('.').collect();
    labels[..labels.len().saturating_sub(1)]
        .iter()
        .any(|label| DENIED_LABELS.contains(label))
}

/// True only for globally routable unicast addresses. Everything a server
/// could reach on its own network (loopback, private ranges, link-local and
/// cloud metadata, CGNAT, ULA, multicast, reserved, documentation) is refused.
pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_public_v4(v4);
            }
            let s = v6.segments();
            !(v6.is_unspecified()
                || v6.is_loopback()
                || v6.is_multicast()
                || (s[0] & 0xfe00) == 0xfc00 // unique local fc00::/7
                || (s[0] & 0xffc0) == 0xfe80 // link-local fe80::/10
                || (s[0] == 0x2001 && s[1] == 0x0db8) // documentation
                || (s[0] == 0x0064 && s[1] == 0xff9b) // NAT64, could reach IPv4 internals
                || s[0] == 0x0100 && s[1] == 0 && s[2] == 0 && s[3] == 0 // discard-only
                || (s[0] == 0x2002)) // 6to4, embeds arbitrary IPv4
        }
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local() // 169.254/16, incl. cloud metadata 169.254.169.254
        || ip.is_broadcast()
        || ip.is_multicast()
        || ip.is_documentation()
        || a == 0
        || (a == 100 && (64..128).contains(&b)) // CGNAT 100.64/10
        || (a == 192 && b == 0 && c == 0) // IETF protocol assignments
        || (a == 198 && (18..20).contains(&b)) // benchmarking
        || a >= 240) // reserved
}

/// Resolves names itself and hands the HTTP client only addresses that pass
/// the policy. The client connects to exactly these addresses, so a name
/// can't be re-resolved to an internal address between check and connect.
pub struct GuardedResolver {
    pub policy: fn(IpAddr) -> bool,
    pub overrides: Arc<Vec<(String, IpAddr)>>,
}

/// Raised inside DNS resolution; found again by walking the client error's sources.
#[derive(Debug)]
pub struct BlockedAddress(pub String);

impl std::fmt::Display for BlockedAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} does not resolve to a public address", self.0)
    }
}

impl std::error::Error for BlockedAddress {}

impl Resolve for GuardedResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_ascii_lowercase();
        let policy = self.policy;
        let pinned = self
            .overrides
            .iter()
            .find(|(h, _)| *h == host)
            .map(|(_, ip)| *ip);
        Box::pin(async move {
            let found: Vec<SocketAddr> = match pinned {
                Some(ip) => vec![SocketAddr::new(ip, 0)],
                None => tokio::net::lookup_host((host.as_str(), 0)).await?.collect(),
            };
            let allowed: Vec<SocketAddr> = found.into_iter().filter(|a| policy(a.ip())).collect();
            if allowed.is_empty() {
                return Err(
                    Box::new(BlockedAddress(host)) as Box<dyn std::error::Error + Send + Sync>
                );
            }
            Ok(Box::new(allowed.into_iter()) as Addrs)
        })
    }
}
