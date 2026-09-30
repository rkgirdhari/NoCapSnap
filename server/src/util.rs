//! Time, ids and capability tokens.

use base64::Engine as _;
use chrono::{DateTime, SecondsFormat, TimeDelta, Utc};
use sha2::{Digest, Sha256};

/// RFC 3339 UTC with milliseconds; text order is time order.
pub fn rfc3339(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn parse_time(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

/// `t + delta`, saturating at the far future instead of overflowing.
pub fn plus(t: DateTime<Utc>, delta: TimeDelta) -> DateTime<Utc> {
    t.checked_add_signed(delta)
        .unwrap_or(DateTime::<Utc>::MAX_UTC)
}

/// `t - delta`, saturating at the far past.
pub fn minus(t: DateTime<Utc>, delta: TimeDelta) -> DateTime<Utc> {
    t.checked_sub_signed(delta)
        .unwrap_or(DateTime::<Utc>::MIN_UTC)
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// 256 bits from the operating system's generator, base64url without padding
/// (43 characters). Used for guest links, guest sessions and device sessions.
pub fn new_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("the OS random generator is unavailable");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Tokens are 256-bit random, so a plain SHA-256 is enough to store them
/// (no slow hash needed); the database never holds a usable token.
pub fn token_hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

pub fn looks_like_token(s: &str) -> bool {
    s.len() == 43
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
