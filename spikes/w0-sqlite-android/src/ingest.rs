use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{SecondsFormat, Utc};
use sha2::{Digest, Sha256};

use crate::{Capture, LocalStore, NewCapture};

/// Upper bound for one photo; a full-resolution phone JPEG is well below this.
pub const MAX_MEDIA_BYTES: usize = 25 * 1024 * 1024;

#[derive(Debug)]
pub enum IngestError {
    Empty,
    TooLarge(usize),
    UnsupportedFormat,
    Io(std::io::Error),
    Db(sqlx::Error),
}

impl fmt::Display for IngestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IngestError::Empty => write!(f, "the photo is empty"),
            IngestError::TooLarge(n) => {
                write!(f, "the photo is {n} bytes; the limit is {MAX_MEDIA_BYTES}")
            }
            IngestError::UnsupportedFormat => {
                write!(f, "only JPEG, PNG or WebP photos can be saved")
            }
            IngestError::Io(e) => write!(f, "could not write the photo: {e}"),
            IngestError::Db(e) => write!(f, "could not record the capture: {e}"),
        }
    }
}

impl std::error::Error for IngestError {}

/// Identifies the image type from its file signature, not from what the
/// caller claims (Spec §5: validate file signatures).
pub fn sniff_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

/// Media files are content-addressed: `<sha256>.<ext>` under `media_dir`.
pub fn media_path(media_dir: &Path, sha256: &str, mime: &str) -> PathBuf {
    let ext = match mime {
        "image/png" => "png",
        "image/webp" => "webp",
        _ => "jpg",
    };
    media_dir.join(format!("{sha256}.{ext}"))
}

impl LocalStore {
    /// Validates the photo, writes it to `media_dir` (atomically, deduplicated
    /// by digest) and queues a new `pending` capture for it.
    pub async fn ingest(
        &self,
        media_dir: &Path,
        staff_id: &str,
        bytes: &[u8],
    ) -> Result<Capture, IngestError> {
        if bytes.is_empty() {
            return Err(IngestError::Empty);
        }
        if bytes.len() > MAX_MEDIA_BYTES {
            return Err(IngestError::TooLarge(bytes.len()));
        }
        let mime = sniff_mime(bytes).ok_or(IngestError::UnsupportedFormat)?;
        let sha256 = hex(&Sha256::digest(bytes));
        write_atomically(&media_path(media_dir, &sha256, mime), bytes).map_err(IngestError::Io)?;

        let client_id = uuid::Uuid::new_v4().to_string();
        // Taken here, at the record's construction, never shared across records.
        let capture_time_utc = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        self.enqueue(&NewCapture {
            client_id: &client_id,
            staff_id,
            media_sha256: &sha256,
            capture_time_utc: &capture_time_utc,
            media_mime: Some(mime),
            media_bytes: Some(bytes.len() as i64),
        })
        .await
        .map_err(IngestError::Db)
    }
}

fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if path.exists() {
        return Ok(()); // same digest, same bytes
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4().simple()));
    let result = (|| {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}
