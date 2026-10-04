//! Spec §7 Phase 4 (W4a): a consistent snapshot of the database and photos, a hash
//! manifest, and a restore that verifies before it writes.
//!
//! Layout of a snapshot directory:
//!
//! ```text
//! snapshot-20261004T020000Z/
//!   manifest.json     format, creation time, and the SHA-256 and size of every other file
//!   capsnap.db        the database, taken with SQLite's own `VACUUM INTO` (never a file copy of a live WAL database)
//!   media/<key>       the photo files
//! ```
//!
//! A snapshot is built in `<name>.partial` and renamed only when complete, so a
//! half-written one is never mistaken for a good one. Encryption and the copies
//! happen outside this module (`deploy/bin/capsnap-backup`); this module never
//! sees a key.

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use crate::config::Config;

pub const MANIFEST: &str = "manifest.json";
pub const DB_FILE: &str = "capsnap.db";
pub const MEDIA_DIR: &str = "media";
const FORMAT: u32 = 1;

/// Present in the data directory while the last backup failed. Retention (and the
/// release switch in `capsnap-release`) refuse to run while it exists: Spec §7,
/// *Backup failure: pause destructive retention or migration tasks*.
pub const FAILED_MARKER: &str = "BACKUP_FAILED";

pub fn marker_path(cfg: &Config) -> PathBuf {
    cfg.db_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(FAILED_MARKER)
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    pub format: u32,
    pub created_at: String,
    pub files: Vec<Entry>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Restored {
    pub organizations: i64,
    pub captures: i64,
    pub photos: usize,
    /// Photos the database lists that the snapshot does not hold. Expected only for
    /// photos retention removed while the snapshot ran (Spec §6); they must stay gone.
    pub photos_missing: usize,
}

fn hash_copy(from: &Path, to: &Path) -> Result<Entry, String> {
    let mut src = std::fs::File::open(from).map_err(|e| format!("{}: {e}", from.display()))?;
    if let Some(dir) = to.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let mut dst = std::fs::File::create(to).map_err(|e| format!("{}: {e}", to.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    let mut bytes = 0u64;
    loop {
        let n = src.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        dst.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        bytes += n as u64;
    }
    dst.sync_all().map_err(|e| e.to_string())?;
    Ok(Entry {
        path: String::new(),
        sha256: hex::encode(hasher.finalize()),
        bytes,
    })
}

fn hash_file(path: &Path) -> Result<(String, u64), String> {
    let mut f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    let mut bytes = 0u64;
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        bytes += n as u64;
    }
    Ok((hex::encode(hasher.finalize()), bytes))
}

/// Every regular file under `dir`, as `/`-separated paths relative to it, sorted.
fn walk(dir: &Path) -> Result<Vec<String>, String> {
    fn go(base: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), String> {
        let read = match std::fs::read_dir(dir) {
            Ok(r) => r,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(format!("{}: {e}", dir.display())),
        };
        for entry in read {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_dir() {
                go(base, &path, out)?;
            } else if kind.is_file() {
                let rel = path.strip_prefix(base).map_err(|e| e.to_string())?;
                let parts: Vec<String> = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect();
                out.push(parts.join("/"));
            }
            // Symlinks and anything else are never followed or copied.
        }
        Ok(())
    }
    let mut out = Vec::new();
    go(dir, dir, &mut out)?;
    out.sort();
    Ok(out)
}

/// Take a snapshot into `out_dir/snapshot-<UTC time>`; returns that path.
pub async fn snapshot(
    pool: &SqlitePool,
    media_dir: &Path,
    out_dir: &Path,
    now: DateTime<Utc>,
) -> Result<PathBuf, String> {
    let name = format!("snapshot-{}", now.format("%Y%m%dT%H%M%SZ"));
    let final_dir = out_dir.join(&name);
    let partial = out_dir.join(format!("{name}.partial"));
    if final_dir.exists() || partial.exists() {
        return Err(format!("{} already exists", final_dir.display()));
    }
    std::fs::create_dir_all(&partial).map_err(|e| format!("{}: {e}", partial.display()))?;
    match build(pool, media_dir, &partial, now).await {
        Ok(()) => {
            std::fs::rename(&partial, &final_dir).map_err(|e| e.to_string())?;
            Ok(final_dir)
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&partial);
            Err(e)
        }
    }
}

async fn build(
    pool: &SqlitePool,
    media_dir: &Path,
    dir: &Path,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let mut files = Vec::new();

    // The database first, then the photos: a photo uploaded after this point is simply
    // not referenced by the copied database, while the other order could reference
    // photos the copy lacks.
    let db_out = dir.join(DB_FILE);
    sqlx::query("VACUUM INTO ?")
        .bind(db_out.to_string_lossy().into_owned())
        .execute(pool)
        .await
        .map_err(|e| format!("could not snapshot the database: {e}"))?;
    let (sha256, bytes) = hash_file(&db_out)?;
    files.push(Entry {
        path: DB_FILE.into(),
        sha256,
        bytes,
    });

    for rel in walk(media_dir)? {
        let to = dir.join(MEDIA_DIR).join(&rel);
        match hash_copy(&media_dir.join(&rel), &to) {
            Ok(mut entry) => {
                entry.path = format!("{MEDIA_DIR}/{rel}");
                files.push(entry);
            }
            // Retention can remove a photo between the listing and the copy.
            Err(_) if !media_dir.join(&rel).exists() => {}
            Err(e) => return Err(e),
        }
    }

    let manifest = Manifest {
        format: FORMAT,
        created_at: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        files,
    };
    let json = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    let mut f = std::fs::File::create(dir.join(MANIFEST)).map_err(|e| e.to_string())?;
    f.write_all(&json).map_err(|e| e.to_string())?;
    f.sync_all().map_err(|e| e.to_string())
}

fn safe_relative(path: &str) -> bool {
    !path.is_empty()
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        && !path.contains('\\')
}

/// Check a snapshot directory end to end: the manifest parses, every file in it
/// exists with the recorded hash and size, nothing else is present, and the
/// database passes SQLite's integrity and foreign-key checks.
pub async fn verify(dir: &Path) -> Result<Manifest, String> {
    let raw = std::fs::read(dir.join(MANIFEST))
        .map_err(|e| format!("{}: {e}", dir.join(MANIFEST).display()))?;
    let manifest: Manifest =
        serde_json::from_slice(&raw).map_err(|e| format!("{MANIFEST} is not valid: {e}"))?;
    if manifest.format != FORMAT {
        return Err(format!("unknown snapshot format {}", manifest.format));
    }

    let mut listed = BTreeSet::new();
    for entry in &manifest.files {
        if !safe_relative(&entry.path) {
            return Err(format!("unsafe path in the manifest: {}", entry.path));
        }
        if !listed.insert(entry.path.clone()) {
            return Err(format!("{} is listed twice", entry.path));
        }
        let (sha256, bytes) = hash_file(&dir.join(&entry.path))?;
        if sha256 != entry.sha256 || bytes != entry.bytes {
            return Err(format!("{} does not match its recorded hash", entry.path));
        }
    }
    if !listed.contains(DB_FILE) {
        return Err(format!("the manifest has no {DB_FILE}"));
    }
    for rel in walk(dir)? {
        if rel != MANIFEST && !listed.contains(&rel) {
            return Err(format!("{rel} is in the snapshot but not in the manifest"));
        }
    }

    check_database(&dir.join(DB_FILE)).await?;
    Ok(manifest)
}

async fn check_database(path: &Path) -> Result<(), String> {
    let options = SqliteConnectOptions::from_str("sqlite://")
        .map_err(|e| e.to_string())?
        .filename(path)
        .read_only(true)
        .immutable(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(|e| format!("the snapshot database does not open: {e}"))?;
    let result = async {
        let checks: Vec<(String,)> = sqlx::query_as("PRAGMA integrity_check")
            .fetch_all(&pool)
            .await
            .map_err(|e| e.to_string())?;
        if checks.len() != 1 || checks[0].0 != "ok" {
            return Err(format!("the database is damaged: {checks:?}"));
        }
        let broken = sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&pool)
            .await
            .map_err(|e| e.to_string())?;
        if !broken.is_empty() {
            return Err(format!("{} foreign-key violations", broken.len()));
        }
        Ok(())
    }
    .await;
    pool.close().await;
    result
}

/// Verify `snapshot_dir`, then restore it into `data_dir`, which must not exist or
/// must be empty: a restore never overwrites live data. Opens the restored database
/// (migrations included) and counts what came back.
pub async fn restore(snapshot_dir: &Path, data_dir: &Path) -> Result<Restored, String> {
    let manifest = verify(snapshot_dir).await?;
    match std::fs::read_dir(data_dir) {
        Ok(mut entries) => {
            if entries.next().is_some() {
                return Err(format!(
                    "{} is not empty; restore into a fresh directory",
                    data_dir.display()
                ));
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("{}: {e}", data_dir.display())),
    }

    let target = Config::for_dir(data_dir, "https://restore.invalid");
    std::fs::create_dir_all(&target.media_dir).map_err(|e| e.to_string())?;
    for entry in &manifest.files {
        let to = if entry.path == DB_FILE {
            target.db_path.clone()
        } else {
            let key = entry
                .path
                .strip_prefix(&format!("{MEDIA_DIR}/"))
                .ok_or_else(|| format!("unexpected file in the manifest: {}", entry.path))?;
            target.media_dir.join(key)
        };
        let copied = hash_copy(&snapshot_dir.join(&entry.path), &to)?;
        if copied.sha256 != entry.sha256 {
            return Err(format!("{} changed while it was restored", entry.path));
        }
    }

    let pool = crate::db::open(&target.db_path)
        .await
        .map_err(|e| format!("the restored database does not open: {e}"))?;
    let count = |sql: &'static str| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, i64>(sql)
                .fetch_one(&pool)
                .await
                .map_err(|e| e.to_string())
        }
    };
    let organizations = count("SELECT COUNT(*) FROM organizations").await?;
    let captures = count("SELECT COUNT(*) FROM captures").await?;
    let keys: Vec<(String,)> =
        sqlx::query_as("SELECT storage_key FROM media_assets WHERE deleted_at IS NULL")
            .fetch_all(&pool)
            .await
            .map_err(|e| e.to_string())?;
    let photos_missing = keys
        .iter()
        .filter(|k| !crate::media::storage_path(&target.media_dir, &k.0).exists())
        .count();
    pool.close().await;
    Ok(Restored {
        organizations,
        captures,
        photos: manifest.files.len() - 1,
        photos_missing,
    })
}
