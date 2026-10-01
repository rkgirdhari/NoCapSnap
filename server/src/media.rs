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
    staff: Staff,
    headers: HeaderMap,
    body: Body,
) -> ApiResult<(StatusCode, Json<MediaView>)> {
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
    let declared = headers
        .get("x-content-sha256")
        .and_then(|v| v.to_str().ok())
        .filter(|v| v.len() == 64 && v.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
        .ok_or_else(|| {
            ApiError::bad_request(
                "missing_digest",
                "send the photo's SHA-256 in X-Content-SHA256",
            )
        })?
        .to_owned();

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
        if marker == 0xDA {
            break; // start of scan: no more header segments
        }
        let len = usize::from(u16::from_be_bytes([data[at + 2], data[at + 3]]));
        if len < 2 || at + 2 + len > data.len() {
            return Err(bad());
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
            _ => {}
        }
        at += 2 + len;
    }
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
