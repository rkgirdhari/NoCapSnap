//! `POST /media` (Spec §5): streamed upload, then validation of the file
//! signature, MIME type, SHA-256 and a full decode before anything is kept.

use std::path::{Path, PathBuf};

use axum::Json;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use chrono::Utc;
use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::AppState;
use crate::auth::Staff;
use crate::error::{ApiError, ApiResult};
use crate::util::{new_id, rfc3339};

/// Spec §3: the phone sends at most 2048 px on the long edge.
pub const MAX_LONG_EDGE: u32 = 2048;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaView {
    pub media_id: String,
    pub sha256: String,
}

pub fn storage_path(media_dir: &Path, storage_key: &str) -> PathBuf {
    media_dir.join(storage_key)
}

pub async fn upload(
    State(state): State<AppState>,
    staff: Result<Staff, ApiError>,
    headers: HeaderMap,
    body: Body,
) -> ApiResult<(StatusCode, Json<MediaView>)> {
    let (staff, declared) = match staff.and_then(|staff| Ok((staff, declared_digest(&headers)?))) {
        Ok(v) => v,
        Err(e) => {
            // Read what the client is sending before refusing. Closing with the body
            // half-sent makes nginx answer 502 instead of this status.
            drain(body, state.cfg.max_media_bytes).await;
            return Err(e);
        }
    };

    // Stream to a temporary file, counting and hashing as it arrives.
    let incoming = state.cfg.media_dir.join(".incoming");
    tokio::fs::create_dir_all(&incoming).await?;
    let tmp = incoming.join(format!("{}.part", new_id()));
    let result = receive(body, &tmp, state.cfg.max_media_bytes).await;
    let (digest, bytes) = match result {
        Ok(v) => v,
        Err(e) => {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(e);
        }
    };
    let outcome = async {
        if digest != declared {
            return Err(ApiError::unprocessable("digest_mismatch", "the photo's SHA-256 doesn't match; it may have been damaged in transit"));
        }
        let data = tokio::fs::read(&tmp).await?;
        let (width, height) = tokio::task::spawn_blocking(move || validate_jpeg(&data))
            .await
            .map_err(|_| ApiError::unprocessable("undecodable", "the photo could not be read"))??;

        let key = format!("{}/{}.jpg", staff.org_id, digest);
        let path = storage_path(&state.cfg.media_dir, &key);
        if let Some(dir) = path.parent() {
            tokio::fs::create_dir_all(dir).await?;
        }
        tokio::fs::rename(&tmp, &path).await?;

        sqlx::query(
            "INSERT INTO media_assets (id, org_id, sha256, mime, bytes, width, height, storage_key, created_at)
             VALUES (?, ?, ?, 'image/jpeg', ?, ?, ?, ?, ?)
             ON CONFLICT (org_id, sha256) DO UPDATE SET deleted_at = NULL, storage_key = excluded.storage_key",
        )
        .bind(new_id())
        .bind(&staff.org_id)
        .bind(&digest)
        .bind(bytes as i64)
        .bind(i64::from(width))
        .bind(i64::from(height))
        .bind(&key)
        .bind(rfc3339(Utc::now()))
        .execute(&state.pool)
        .await?;
        let (media_id,): (String,) = sqlx::query_as("SELECT id FROM media_assets WHERE org_id = ? AND sha256 = ?")
            .bind(&staff.org_id)
            .bind(&digest)
            .fetch_one(&state.pool)
            .await?;
        Ok((StatusCode::CREATED, Json(MediaView { media_id, sha256: digest })))
    }
    .await;
    let _ = tokio::fs::remove_file(&tmp).await; // no-op after a successful rename
    outcome
}

/// The upload's declared SHA-256, once the content type is checked.
fn declared_digest(headers: &HeaderMap) -> ApiResult<String> {
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if content_type.split(';').next().map(str::trim) != Some("image/jpeg") {
        return Err(ApiError::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported_type",
            "only image/jpeg is accepted",
        ));
    }
    headers
        .get("x-content-sha256")
        .and_then(|v| v.to_str().ok())
        .filter(|v| v.len() == 64 && v.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
        .map(str::to_owned)
        .ok_or_else(|| {
            ApiError::bad_request(
                "missing_digest",
                "send the photo's SHA-256 in X-Content-SHA256",
            )
        })
}

/// Reads and discards a request body: at most `max` bytes, for at most 30 seconds.
async fn drain(body: Body, max: usize) {
    let read = async {
        let mut stream = body.into_data_stream();
        let mut total = 0usize;
        while let Some(Ok(chunk)) = stream.next().await {
            total += chunk.len();
            if total > max {
                break;
            }
        }
    };
    let _ = tokio::time::timeout(std::time::Duration::from_secs(30), read).await;
}

async fn receive(body: Body, tmp: &Path, max: usize) -> ApiResult<(String, usize)> {
    let mut file = tokio::fs::File::create(tmp).await?;
    let mut hasher = Sha256::new();
    let mut total = 0usize;
    let mut stream = body.into_data_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| {
            ApiError::bad_request("upload_interrupted", "the upload was interrupted")
        })?;
        total += chunk.len();
        if total > max {
            return Err(ApiError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "too_large",
                format!("a photo can be at most {max} bytes"),
            ));
        }
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
    }
    file.sync_all().await?;
    if total == 0 {
        return Err(ApiError::bad_request("empty", "the photo is empty"));
    }
    Ok((hex::encode(hasher.finalize()), total))
}

/// `from` is the first byte of the entropy-coded data. Skips it (stuffed `FF 00` and
/// restart markers belong to it) and requires the next marker to be EOI, at the very
/// end of the file: any other segment, or any byte after EOI, could carry hidden data.
fn reject_extra_after_scan(data: &[u8], from: usize) -> ApiResult<()> {
    let mut at = from;
    while at + 1 < data.len() {
        if data[at] != 0xFF {
            at += 1;
            continue;
        }
        match data[at + 1] {
            0x00 | 0xD0..=0xD7 => at += 2,
            0xFF => at += 1, // fill byte before a marker
            0xD9 if at + 2 == data.len() => return Ok(()),
            0xD9 => {
                return Err(ApiError::unprocessable(
                    "metadata_present",
                    "the photo has extra data after its end",
                ));
            }
            _ => {
                return Err(ApiError::unprocessable(
                    "metadata_present",
                    "the photo has extra segments after its image data",
                ));
            }
        }
    }
    Err(ApiError::unprocessable(
        "undecodable",
        "the photo could not be read",
    ))
}

/// The server's own check that the phone did its job (Spec §3, §5): a
/// baseline JPEG whose only header segment besides tables is a plain JFIF
/// APP0 without a thumbnail, that decodes completely, at ≤ 2048 px.
pub fn validate_jpeg(data: &[u8]) -> ApiResult<(u32, u32)> {
    if !data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Err(ApiError::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "not_jpeg",
            "the file is not a JPEG",
        ));
    }
    let bad = || ApiError::unprocessable("undecodable", "the photo could not be read");
    let mut at = 2;
    loop {
        if at + 4 > data.len() || data[at] != 0xFF {
            return Err(bad());
        }
        let marker = data[at + 1];
        let len = usize::from(u16::from_be_bytes([data[at + 2], data[at + 3]]));
        if len < 2 || at + 2 + len > data.len() {
            return Err(bad());
        }
        if marker == 0xDA {
            // Start of scan: no more header segments. What follows must be the
            // entropy-coded data and the end of the image, nothing else.
            at += 2 + len;
            break;
        }
        let segment = &data[at + 4..at + 2 + len];
        match marker {
            0xE1..=0xEF | 0xFE => {
                return Err(ApiError::unprocessable(
                    "metadata_present",
                    "the photo still carries metadata; it must be processed on the phone first",
                ));
            }
            // JFIF: "JFIF\0", version, units, density (4 bytes), then thumbnail width/height.
            0xE0 if !(segment.len() >= 14
                && &segment[..5] == b"JFIF\0"
                && segment[12] == 0
                && segment[13] == 0) =>
            {
                return Err(ApiError::unprocessable(
                    "metadata_present",
                    "the photo carries an embedded thumbnail or extension block",
                ));
            }
            // Baseline only: a progressive or other multi-scan image can carry segments
            // between its scans.
            0xC1..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF => {
                return Err(ApiError::unprocessable(
                    "not_baseline",
                    "the photo must be a baseline JPEG; it must be processed on the phone first",
                ));
            }
            _ => {}
        }
        at += 2 + len;
    }
    reject_extra_after_scan(data, at)?;
    let mut reader = image::ImageReader::new(std::io::Cursor::new(data));
    reader.set_format(image::ImageFormat::Jpeg);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|_| bad())?;
    let (w, h) = (decoded.width(), decoded.height());
    if w.max(h) > MAX_LONG_EDGE {
        return Err(ApiError::unprocessable(
            "too_large_dimensions",
            format!("the photo is {w}×{h}; the limit is {MAX_LONG_EDGE} px on the long edge"),
        ));
    }
    Ok((w, h))
}

#[cfg(test)]
mod tests {
    use super::validate_jpeg;

    fn encoded() -> Vec<u8> {
        let img = image::RgbImage::from_fn(64, 48, |x, y| image::Rgb([x as u8, y as u8, 7]));
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Jpeg).unwrap();
        out.into_inner()
    }

    fn code(data: &[u8]) -> &'static str {
        validate_jpeg(data).expect_err("must be refused").code
    }

    #[test]
    fn plain_baseline_jpeg_passes() {
        assert_eq!(validate_jpeg(&encoded()).unwrap(), (64, 48));
    }

    #[test]
    fn bytes_after_eoi_are_refused() {
        let mut data = encoded();
        data.extend_from_slice(b"hidden gps 41.88,-87.63");
        assert_eq!(code(&data), "metadata_present");
        let mut data = encoded();
        data.extend_from_slice(&[0xFF, 0xD9]); // a second EOI
        assert_eq!(code(&data), "metadata_present");
    }

    #[test]
    fn segment_between_scan_and_eoi_is_refused() {
        let mut data = encoded();
        let eoi = data.len() - 2;
        let mut app1 = vec![0xFF, 0xE1, 0x00, 0x10];
        app1.extend_from_slice(b"Exif\0\0MM\0\x2a\0\0\0\x08");
        data.splice(eoi..eoi, app1);
        assert_eq!(code(&data), "metadata_present");
    }

    #[test]
    fn comment_after_scan_is_refused() {
        let mut data = encoded();
        let eoi = data.len() - 2;
        data.splice(eoi..eoi, [0xFF, 0xFE, 0x00, 0x04, b'h', b'i']);
        assert_eq!(code(&data), "metadata_present");
    }

    #[test]
    fn progressive_jpeg_is_refused() {
        let mut data = encoded();
        let sof = data
            .windows(2)
            .position(|w| w == [0xFF, 0xC0])
            .expect("baseline SOF0");
        data[sof + 1] = 0xC2;
        assert_eq!(code(&data), "not_baseline");
    }

    #[test]
    fn missing_eoi_is_refused() {
        let mut data = encoded();
        data.truncate(data.len() - 2);
        assert_eq!(code(&data), "undecodable");
    }
}
