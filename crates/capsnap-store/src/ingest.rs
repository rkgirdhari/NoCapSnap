use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{SecondsFormat, Utc};
use sha2::{Digest, Sha256};

use crate::photo::{self, PhotoError};
use crate::{Capture, LocalStore, NewCapture};

/// Upper bound for one photo as received; a full-resolution phone JPEG is well below this.
pub const MAX_MEDIA_BYTES: usize = 25 * 1024 * 1024;
/// Table labels are short staff references such as "12B" (owner default M6).
pub const MAX_TABLE_LABEL_CHARS: usize = 16;

/// What the staff member chose on "Which dish?". Both are optional.
#[derive(Debug, Default, Clone, Copy)]
pub struct CaptureDetails<'a> {
    pub menu_item_id: Option<&'a str>,
    pub table_label: Option<&'a str>,
}

#[derive(Debug)]
pub enum IngestError {
    Empty,
    TooLarge(usize),
    UnsupportedFormat,
    /// The bytes carried a known signature but did not decode as a photo.
    Undecodable(PhotoError),
    UnknownDish,
    InvalidTableLabel,
    /// The photo arrived as text that is not valid base64.
    BadEncoding,
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
            IngestError::Undecodable(e) => write!(f, "the photo could not be read: {e}"),
            IngestError::UnknownDish => write!(f, "that dish is not on this menu"),
            IngestError::BadEncoding => write!(f, "the photo did not arrive intact; try again"),
            IngestError::InvalidTableLabel => write!(
                f,
                "a table label is up to {MAX_TABLE_LABEL_CHARS} letters, numbers or spaces"
            ),
            IngestError::Io(e) => write!(f, "could not write the photo: {e}"),
            IngestError::Db(e) => write!(f, "could not record the capture: {e}"),
        }
    }
}

impl std::error::Error for IngestError {}

/// Decodes a photo sent as base64 text. On Android, Tauri carries IPC over
/// `postMessage` as a string because the WebView cannot hand a request body to
/// the app, so the UI base64-encodes the bytes there (a plain byte array would
/// arrive as a JSON list of numbers, several times larger). The size limit is
/// checked on the encoded length, before anything is decoded.
pub fn photo_from_base64(encoded: &str) -> Result<Vec<u8>, IngestError> {
    use base64::Engine as _;
    let max_encoded = MAX_MEDIA_BYTES.div_ceil(3) * 4;
    if encoded.len() > max_encoded {
        return Err(IngestError::TooLarge(encoded.len() / 4 * 3));
    }
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| IngestError::BadEncoding)
}

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
/// Since W2 every stored photo is a processed JPEG.
pub fn media_path(media_dir: &Path, sha256: &str, mime: &str) -> PathBuf {
    let ext = match mime {
        "image/png" => "png",
        "image/webp" => "webp",
        _ => "jpg",
    };
    media_dir.join(format!("{sha256}.{ext}"))
}

/// The list/grid thumbnail stored beside each photo. Device-only; never synced.
pub fn thumb_path(media_dir: &Path, sha256: &str) -> PathBuf {
    media_dir.join(format!("{sha256}.thumb.jpg"))
}

fn image_format(mime: &str) -> image::ImageFormat {
    match mime {
        "image/png" => image::ImageFormat::Png,
        "image/webp" => image::ImageFormat::WebP,
        _ => image::ImageFormat::Jpeg,
    }
}

/// Trimmed; empty means "no label". Short, table-style references only:
/// letters, digits, spaces and `-#/.`, nothing that renders as markup.
fn normalize_table_label(raw: Option<&str>) -> Result<Option<String>, IngestError> {
    let Some(label) = raw.map(str::trim).filter(|l| !l.is_empty()) else {
        return Ok(None);
    };
    let allowed = |c: char| c.is_alphanumeric() || matches!(c, ' ' | '-' | '#' | '/' | '.');
    if label.chars().count() > MAX_TABLE_LABEL_CHARS || !label.chars().all(allowed) {
        return Err(IngestError::InvalidTableLabel);
    }
    Ok(Some(label.to_owned()))
}

impl LocalStore {
    /// Validates the photo and the chosen dish, processes the photo on the
    /// device (orientation applied, at most 2048 px, every metadata segment
    /// dropped), writes photo and thumbnail to `media_dir` (atomically,
    /// deduplicated by digest) and queues a new `pending` capture.
    ///
    /// The original bytes are never written to disk.
    pub async fn ingest(
        &self,
        media_dir: &Path,
        staff_id: &str,
        details: CaptureDetails<'_>,
        bytes: Vec<u8>,
    ) -> Result<Capture, IngestError> {
        if bytes.is_empty() {
            return Err(IngestError::Empty);
        }
        if bytes.len() > MAX_MEDIA_BYTES {
            return Err(IngestError::TooLarge(bytes.len()));
        }
        let format = image_format(sniff_mime(&bytes).ok_or(IngestError::UnsupportedFormat)?);
        let table_label = normalize_table_label(details.table_label)?;
        // A capture belongs to the dish's location, or else to the current one.
        let current_location = self
            .setting(crate::Setting::LocationId)
            .await
            .map_err(IngestError::Db)?;
        let dish = match details.menu_item_id {
            Some(id) => match self.menu_item(id).await.map_err(IngestError::Db)? {
                Some(item) if item.is_active => Some(item),
                _ => return Err(IngestError::UnknownDish),
            },
            None => None,
        };

        let processed = tokio::task::spawn_blocking(move || photo::process(&bytes, format))
            .await
            .map_err(|e| IngestError::Io(std::io::Error::other(e)))?
            .map_err(IngestError::Undecodable)?;

        let sha256 = hex(&Sha256::digest(&processed.jpeg));
        let mime = "image/jpeg";
        // Thumbnail first: a photo on disk always has its thumbnail.
        write_atomically(&thumb_path(media_dir, &sha256), &processed.thumb_jpeg)
            .map_err(IngestError::Io)?;
        write_atomically(&media_path(media_dir, &sha256, mime), &processed.jpeg)
            .map_err(IngestError::Io)?;

        let client_id = uuid::Uuid::new_v4().to_string();
        // Taken here, at the record's construction, never shared across records.
        let capture_time_utc = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        self.enqueue(&NewCapture {
            client_id: &client_id,
            staff_id,
            media_sha256: &sha256,
            capture_time_utc: &capture_time_utc,
            media_mime: Some(mime),
            media_bytes: Some(processed.jpeg.len() as i64),
            media_width: Some(i64::from(processed.width)),
            media_height: Some(i64::from(processed.height)),
            location_id: dish
                .as_ref()
                .map(|d| d.location_id.as_str())
                .or(current_location.as_deref()),
            menu_item_id: dish.as_ref().map(|d| d.id.as_str()),
            dish_name: dish.as_ref().map(|d| d.name.as_str()),
            table_label: table_label.as_deref(),
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
