//! Argon2id (the crate's default parameters) for staff passwords.

use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};

pub const MIN_PASSWORD_CHARS: usize = 10;

pub fn hash(password: &str) -> Result<String, String> {
    if password.chars().count() < MIN_PASSWORD_CHARS {
        return Err(format!(
            "a password needs at least {MIN_PASSWORD_CHARS} characters"
        ));
    }
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

pub fn verify(password: &str, phc: &str) -> bool {
    PasswordHash::new(phc)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

/// Verified against when the sign-in name doesn't exist, so a wrong name and a
/// wrong password take about the same time.
pub fn dummy_hash() -> &'static str {
    static DUMMY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    DUMMY.get_or_init(|| hash("not-a-real-password-for-timing").expect("argon2 works"))
}
