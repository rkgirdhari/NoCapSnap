//! Spec §7 Phase 4 (W4a): a consistent snapshot of the database and photos, a hash
//! manifest, and a restore that verifies before it writes.
//!
//! Layout of a snapshot directory:
//!
//! ```text
//! snapshot-20261004T020000Z/
//!   manifest.json     format, creation time, and the SHA-256 and size of every other file
//!   capsnap.db        the database, taken with SQLite's own `VACUUM INTO` (never a file copy of a live WAL database)
//!   media/<key>       the photo files, except the ones marked `external` (see below)
//! ```
//!
//! Photos never change once stored, so a snapshot can leave out a photo a *previous* backup
//! already holds: the caller passes the SHA-256s it has (`known`), and those photos appear in
//! the manifest as `external` entries (path, hash, size; no file). Such a snapshot verifies
//! by itself, but restoring it needs the photos back from wherever they were kept, passed to
//! `restore_with_pool` as a directory of plain files named by SHA-256.
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
/// 1: every file is in the snapshot. 2: may hold `external` photo entries.
const FORMAT: u32 = 2;
const OLDEST_FORMAT: u32 = 1;

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
    /// A photo kept outside this snapshot, by hash (format 2).
    #[serde(default, skip_serializing_if = "is_false")]
    pub external: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
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
        external: false,
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

/// Take a full snapshot into `out_dir/snapshot-<UTC time>`; returns that path.
pub async fn snapshot(
    pool: &SqlitePool,
    media_dir: &Path,
    out_dir: &Path,
    now: DateTime<Utc>,
) -> Result<PathBuf, String> {
    snapshot_incremental(pool, media_dir, out_dir, now, &BTreeSet::new()).await
}

/// Like `snapshot`, but photos whose SHA-256 is in `known` are listed as `external`
/// instead of copied.
pub async fn snapshot_incremental(
    pool: &SqlitePool,
    media_dir: &Path,
    out_dir: &Path,
    now: DateTime<Utc>,
    known: &BTreeSet<String>,
) -> Result<PathBuf, String> {
    let name = format!("snapshot-{}", now.format("%Y%m%dT%H%M%SZ"));
    let final_dir = out_dir.join(&name);
    let partial = out_dir.join(format!("{name}.partial"));
    if final_dir.exists() || partial.exists() {
        return Err(format!("{} already exists", final_dir.display()));
    }
    std::fs::create_dir_all(&partial).map_err(|e| format!("{}: {e}", partial.display()))?;
    match build(pool, media_dir, &partial, now, known).await {
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
    known: &BTreeSet<String>,
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
        external: false,
    });

    for rel in walk(media_dir)? {
        let from = media_dir.join(&rel);
        // Photos are immutable, so a hash already in `known` is a copy kept elsewhere.
        if !known.is_empty() {
            match hash_file(&from) {
                Ok((sha256, bytes)) if known.contains(&sha256) => {
                    files.push(Entry {
                        path: format!("{MEDIA_DIR}/{rel}"),
                        sha256,
                        bytes,
                        external: true,
                    });
                    continue;
                }
                Ok(_) => {}
                Err(_) if !from.exists() => continue,
                Err(e) => return Err(e),
            }
        }
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

fn valid_sha256(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
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
    if !(OLDEST_FORMAT..=FORMAT).contains(&manifest.format) {
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
        if entry.external {
            if !entry.path.starts_with(&format!("{MEDIA_DIR}/")) {
                return Err(format!("{} cannot be external", entry.path));
            }
            if !valid_sha256(&entry.sha256) {
                return Err(format!("{} has no valid hash", entry.path));
            }
            continue;
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
    restore_with_pool(snapshot_dir, data_dir, None).await
}

/// `restore`, taking `external` photos from `pool_dir`: a directory of plain photo files named
/// by their SHA-256. Every external photo must be there with the recorded hash, or nothing is written.
pub async fn restore_with_pool(
    snapshot_dir: &Path,
    data_dir: &Path,
    pool_dir: Option<&Path>,
) -> Result<Restored, String> {
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

    let source_of = |entry: &Entry| -> Result<PathBuf, String> {
        if !entry.external {
            return Ok(snapshot_dir.join(&entry.path));
        }
        let pool = pool_dir.ok_or_else(|| {
            format!(
                "{} is kept outside this snapshot; pass the photo pool",
                entry.path
            )
        })?;
        Ok(pool.join(&entry.sha256))
    };
    // Check the pool before writing anything.
    for entry in manifest.files.iter().filter(|e| e.external) {
        let from = source_of(entry)?;
        let (sha256, bytes) = hash_file(&from)
            .map_err(|e| format!("photo {} is missing from the pool: {e}", entry.sha256))?;
        if sha256 != entry.sha256 || bytes != entry.bytes {
            return Err(format!(
                "pool photo {} does not match its hash",
                entry.sha256
            ));
        }
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
        let copied = hash_copy(&source_of(entry)?, &to)?;
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
