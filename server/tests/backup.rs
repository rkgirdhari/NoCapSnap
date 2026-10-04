//! W4a: snapshot, verify and restore (Spec §7 Phase 4).

mod common;

use capsnap_server::{AppState, Config, backup};
use common::*;

/// A world with two captures (two photos), one of them answered by a guest.
async fn populated() -> (World, std::path::PathBuf) {
    let w = world().await;
    let app = app(&w);
    let token = sign_in(&app, "ada@atelier").await;
    for (n, seed) in [("one-aaaa", 1u8), ("two-bbbb", 2u8)] {
        let media = upload_ok(&app, &token, phone_jpeg(64, 48, seed)).await;
        let r = capture(&app, &token, n, &media, &w.loc_a1, None).await;
        assert!(
            r.status.is_success(),
            "{}",
            String::from_utf8_lossy(&r.body)
        );
    }
    let out = w._dir.path().join("backups");
    std::fs::create_dir_all(&out).unwrap();
    (w, out)
}

async fn snap(w: &World, out: &std::path::Path) -> std::path::PathBuf {
    backup::snapshot(
        &w.state.pool,
        &w.state.cfg.media_dir,
        out,
        chrono::Utc::now(),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn a_snapshot_verifies_and_restores_into_a_fresh_directory() {
    let (w, out) = populated().await;
    let dir = snap(&w, &out).await;

    let manifest = backup::verify(&dir).await.unwrap();
    assert_eq!(manifest.files.len(), 3, "database and two photos");
    assert!(!out.read_dir().unwrap().any(|e| {
        e.unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".partial")
    }));

    let fresh = tempfile::tempdir().unwrap();
    let restored = backup::restore(&dir, &fresh.path().join("data"))
        .await
        .unwrap();
    assert_eq!(restored.organizations, 2);
    assert_eq!(restored.captures, 2);
    assert_eq!(restored.photos, 2);
    assert_eq!(restored.photos_missing, 0);

    // The restored copy opens as a normal server and still knows the staff.
    let cfg = Config::for_dir(&fresh.path().join("data"), "https://guests.example");
    let state = AppState::open(cfg).await.unwrap();
    let staff: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM staff")
        .fetch_one(&state.pool)
        .await
        .unwrap();
    assert!(staff >= 1);
}

#[tokio::test]
async fn a_snapshot_is_refused_when_a_file_is_changed_or_added() {
    let (w, out) = populated().await;
    let dir = snap(&w, &out).await;

    fn first_file(dir: &std::path::Path) -> Option<std::path::PathBuf> {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_file() {
                return Some(p);
            }
            if let Some(f) = first_file(&p) {
                return Some(f);
            }
        }
        None
    }
    let photo = first_file(&dir.join("media")).unwrap();
    let original = std::fs::read(&photo).unwrap();
    let mut bad = original.clone();
    bad[10] ^= 0xff;
    std::fs::write(&photo, &bad).unwrap();
    let err = backup::verify(&dir).await.unwrap_err();
    assert!(err.contains("does not match"), "{err}");
    std::fs::write(&photo, &original).unwrap();
    backup::verify(&dir).await.unwrap();

    std::fs::write(dir.join("media").join("extra.jpg"), b"x").unwrap();
    let err = backup::verify(&dir).await.unwrap_err();
    assert!(err.contains("not in the manifest"), "{err}");
}

#[tokio::test]
async fn a_damaged_database_is_refused() {
    let (w, out) = populated().await;
    let dir = snap(&w, &out).await;
    // Overwrite the middle of the database and fix the manifest to match, so only
    // SQLite's own check can notice.
    let db = dir.join(backup::DB_FILE);
    let mut bytes = std::fs::read(&db).unwrap();
    let n = bytes.len();
    assert!(n > 12288);
    for b in &mut bytes[4096..12288] {
        *b = 0xAB;
    }
    std::fs::write(&db, &bytes).unwrap();
    let mut m: backup::Manifest =
        serde_json::from_slice(&std::fs::read(dir.join(backup::MANIFEST)).unwrap()).unwrap();
    use sha2::{Digest, Sha256};
    let entry = m
        .files
        .iter_mut()
        .find(|e| e.path == backup::DB_FILE)
        .unwrap();
    entry.sha256 = hex::encode(Sha256::digest(&bytes));
    std::fs::write(dir.join(backup::MANIFEST), serde_json::to_vec(&m).unwrap()).unwrap();
    assert!(backup::verify(&dir).await.is_err());
}

#[tokio::test]
async fn a_restore_never_writes_into_a_directory_that_has_data() {
    let (w, out) = populated().await;
    let dir = snap(&w, &out).await;
    let live = w._dir.path().to_path_buf();
    let err = backup::restore(&dir, &live).await.unwrap_err();
    assert!(err.contains("not empty"), "{err}");
}

#[tokio::test]
async fn a_snapshot_taken_while_the_server_writes_is_consistent() {
    let (w, out) = populated().await;
    let pool = w.state.pool.clone();
    let writer = tokio::spawn(async move {
        for i in 0..200 {
            let _ = sqlx::query(
                "INSERT INTO organizations (id, name, slug, created_at) VALUES (?, 'x', ?, '2026-10-04T00:00:00Z')",
            )
            .bind(format!("org-{i}"))
            .bind(format!("slug-{i}"))
            .execute(&pool)
            .await;
        }
    });
    let mut dirs = Vec::new();
    for second in 0..5 {
        let now = chrono::Utc::now() + chrono::TimeDelta::seconds(second);
        dirs.push(
            backup::snapshot(&w.state.pool, &w.state.cfg.media_dir, &out, now)
                .await
                .unwrap(),
        );
    }
    writer.await.unwrap();
    for d in dirs {
        backup::verify(&d).await.unwrap();
    }
}

#[tokio::test]
async fn photos_retention_already_removed_stay_gone_after_a_restore() {
    let (w, out) = populated().await;
    // Retention removes the photo files and marks them; the snapshot reflects that.
    sqlx::query("UPDATE media_assets SET deleted_at = '2026-10-04T00:00:00Z'")
        .execute(&w.state.pool)
        .await
        .unwrap();
    for e in std::fs::read_dir(&w.state.cfg.media_dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_file() {
            std::fs::remove_file(p).unwrap();
        } else {
            std::fs::remove_dir_all(p).unwrap();
        }
    }
    let dir = snap(&w, &out).await;
    let fresh = tempfile::tempdir().unwrap();
    let r = backup::restore(&dir, &fresh.path().join("d"))
        .await
        .unwrap();
    assert_eq!((r.photos, r.photos_missing), (0, 0));
}

#[test]
fn the_failure_marker_sits_in_the_data_directory() {
    let cfg = Config::for_dir(
        std::path::Path::new("/var/lib/capsnap"),
        "https://x.example",
    );
    assert_eq!(
        backup::marker_path(&cfg),
        std::path::Path::new("/var/lib/capsnap").join("BACKUP_FAILED")
    );
}
