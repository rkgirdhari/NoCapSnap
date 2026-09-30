use std::ffi::CString;

use capsnap_store_spike::{LocalStore, NewCapture, Pragma, capsnap_spike_selftest, selftest};

fn digest(c: char) -> String {
    c.to_string().repeat(64)
}

fn capture<'a>(client_id: &'a str, sha: &'a str, at: &'a str) -> NewCapture<'a> {
    NewCapture {
        client_id,
        staff_id: "staff-1",
        media_sha256: sha,
        capture_time_utc: at,
    }
}

#[tokio::test]
async fn opens_in_wal_with_full_sync_and_foreign_keys() {
    let dir = tempfile::tempdir().unwrap();
    let store = LocalStore::open(&dir.path().join("a.db")).await.unwrap();
    assert_eq!(store.pragma(Pragma::JournalMode).await.unwrap(), "wal");
    assert_eq!(store.pragma(Pragma::Synchronous).await.unwrap(), "2"); // FULL
    assert_eq!(store.pragma(Pragma::ForeignKeys).await.unwrap(), "1");
    let version = store.sqlite_version().await.unwrap();
    let minor: u32 = version.split('.').nth(1).unwrap().parse().unwrap();
    assert!(
        version.starts_with("3.") && minor >= 45,
        "bundled SQLite too old: {version}"
    );
}

#[tokio::test]
async fn new_captures_start_pending_in_capture_order() {
    let dir = tempfile::tempdir().unwrap();
    let store = LocalStore::open(&dir.path().join("a.db")).await.unwrap();
    let (a, b) = (digest('a'), digest('b'));
    store
        .enqueue(&capture("c2", &b, "2026-09-30T12:05:00Z"))
        .await
        .unwrap();
    let first = store
        .enqueue(&capture("c1", &a, "2026-09-30T12:00:00Z"))
        .await
        .unwrap();
    assert_eq!(first.sync_state, "pending");
    assert_eq!(first.synced_at_utc, None);

    let pending: Vec<_> = store
        .pending()
        .await
        .unwrap()
        .into_iter()
        .map(|c| c.client_id)
        .collect();
    assert_eq!(pending, ["c1", "c2"]);
}

#[tokio::test]
async fn re_enqueue_is_idempotent_on_client_id() {
    let dir = tempfile::tempdir().unwrap();
    let store = LocalStore::open(&dir.path().join("a.db")).await.unwrap();
    let (a, b) = (digest('a'), digest('b'));
    let original = store
        .enqueue(&capture("c1", &a, "2026-09-30T12:00:00Z"))
        .await
        .unwrap();
    // A retry with different metadata must not overwrite or duplicate the row.
    let retried = store
        .enqueue(&capture("c1", &b, "2026-09-30T13:00:00Z"))
        .await
        .unwrap();
    assert_eq!(retried, original);
    assert_eq!(store.pending().await.unwrap().len(), 1);
}

#[tokio::test]
async fn mark_synced_moves_capture_out_of_the_outbox_once() {
    let dir = tempfile::tempdir().unwrap();
    let store = LocalStore::open(&dir.path().join("a.db")).await.unwrap();
    let a = digest('a');
    store
        .enqueue(&capture("c1", &a, "2026-09-30T12:00:00Z"))
        .await
        .unwrap();

    assert!(
        store
            .mark_synced("c1", "2026-09-30T12:00:05Z")
            .await
            .unwrap()
    );
    assert!(
        !store
            .mark_synced("c1", "2026-09-30T12:09:00Z")
            .await
            .unwrap(),
        "second ack is a no-op"
    );
    assert!(
        !store
            .mark_synced("unknown", "2026-09-30T12:00:05Z")
            .await
            .unwrap()
    );

    let row = store.get("c1").await.unwrap().unwrap();
    assert_eq!(row.sync_state, "synced");
    assert_eq!(row.synced_at_utc.as_deref(), Some("2026-09-30T12:00:05Z"));
    assert!(store.pending().await.unwrap().is_empty());
}

#[tokio::test]
async fn schema_rejects_malformed_digests() {
    let dir = tempfile::tempdir().unwrap();
    let store = LocalStore::open(&dir.path().join("a.db")).await.unwrap();
    let upper = "A".repeat(64);
    for bad in ["abc", upper.as_str(), &"g".repeat(64)] {
        let err = store
            .enqueue(&capture("bad", bad, "2026-09-30T12:00:00Z"))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("CHECK constraint failed"),
            "{bad}: {err}"
        );
    }
    assert!(store.pending().await.unwrap().is_empty());
}

#[tokio::test]
async fn committed_captures_survive_close_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.db");
    let store = LocalStore::open(&path).await.unwrap();
    let a = digest('a');
    store
        .enqueue(&capture("c1", &a, "2026-09-30T12:00:00Z"))
        .await
        .unwrap();
    store.close().await;

    let reopened = LocalStore::open(&path).await.unwrap(); // migrations are not re-applied
    assert_eq!(reopened.pending().await.unwrap().len(), 1);
}

#[tokio::test]
async fn selftest_round_trip_passes() {
    let dir = tempfile::tempdir().unwrap();
    let summary = selftest(&dir.path().join("s.db")).await.unwrap();
    assert!(summary.starts_with("ok sqlite=3."), "{summary}");
}

#[test]
fn ffi_selftest_returns_zero_and_rejects_null() {
    let dir = tempfile::tempdir().unwrap();
    let path = CString::new(dir.path().join("f.db").to_str().unwrap()).unwrap();
    assert_eq!(unsafe { capsnap_spike_selftest(path.as_ptr()) }, 0);
    assert_eq!(unsafe { capsnap_spike_selftest(std::ptr::null()) }, 2);
}
