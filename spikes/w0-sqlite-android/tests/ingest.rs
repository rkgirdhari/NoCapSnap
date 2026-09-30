use capsnap_store_spike::{IngestError, LocalStore, MAX_MEDIA_BYTES, media_path, sniff_mime};

const JPEG: &[u8] = &[
    0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00, 0xFF, 0xD9,
];
// sha256 of JPEG above, computed independently with `sha256sum`.
const JPEG_SHA256: &str = "f46e8d0056321744073c78d41d11d020ffabd11d595b347e5ba24b363de48016";

async fn store() -> (tempfile::TempDir, LocalStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = LocalStore::open(&dir.path().join("a.db")).await.unwrap();
    (dir, store)
}

#[tokio::test]
async fn ingest_writes_the_file_and_queues_a_pending_capture() {
    let (dir, store) = store().await;
    let media = dir.path().join("media");

    let capture = store.ingest(&media, "staff-1", JPEG).await.unwrap();

    assert_eq!(capture.media_sha256, JPEG_SHA256);
    assert_eq!(capture.media_mime.as_deref(), Some("image/jpeg"));
    assert_eq!(capture.media_bytes, Some(JPEG.len() as i64));
    assert_eq!(capture.sync_state, "pending");
    assert_eq!(capture.staff_id, "staff-1");
    assert_eq!(capture.client_id.len(), 36, "uuid v4");
    // RFC 3339 UTC with milliseconds, so string order is time order.
    assert!(
        capture.capture_time_utc.len() == 24 && capture.capture_time_utc.ends_with('Z'),
        "{}",
        capture.capture_time_utc
    );

    let path = media_path(&media, JPEG_SHA256, "image/jpeg");
    assert_eq!(std::fs::read(&path).unwrap(), JPEG);
    let leftovers: Vec<_> = std::fs::read_dir(&media)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .filter(|n| n.to_string_lossy().contains("tmp-"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "temp files left behind: {leftovers:?}"
    );
}

#[tokio::test]
async fn the_same_photo_twice_is_two_captures_but_one_file() {
    let (dir, store) = store().await;
    let media = dir.path().join("media");
    let a = store.ingest(&media, "staff-1", JPEG).await.unwrap();
    let b = store.ingest(&media, "staff-1", JPEG).await.unwrap();

    assert_ne!(a.client_id, b.client_id);
    assert_eq!(store.counts().await.unwrap().pending, 2);
    assert_eq!(std::fs::read_dir(&media).unwrap().count(), 1);
}

#[tokio::test]
async fn rejects_empty_oversized_and_non_image_input_without_side_effects() {
    let (dir, store) = store().await;
    let media = dir.path().join("media");

    assert!(matches!(
        store.ingest(&media, "s", b"").await,
        Err(IngestError::Empty)
    ));
    let huge = vec![0xFF; MAX_MEDIA_BYTES + 1];
    assert!(matches!(
        store.ingest(&media, "s", &huge).await,
        Err(IngestError::TooLarge(_))
    ));
    let gif = b"GIF89a\x01\x00\x01\x00";
    assert!(matches!(
        store.ingest(&media, "s", gif).await,
        Err(IngestError::UnsupportedFormat)
    ));

    assert_eq!(store.counts().await.unwrap(), Default::default());
    assert!(!media.exists());
}

#[test]
fn sniffs_formats_from_signatures() {
    assert_eq!(sniff_mime(JPEG), Some("image/jpeg"));
    assert_eq!(sniff_mime(b"\x89PNG\r\n\x1a\n rest"), Some("image/png"));
    assert_eq!(
        sniff_mime(b"RIFF\x10\x00\x00\x00WEBPVP8 "),
        Some("image/webp")
    );
    assert_eq!(sniff_mime(b"RIFF\x10\x00\x00\x00WAVEfmt "), None);
    assert_eq!(sniff_mime(b"<svg"), None);
}

#[tokio::test]
async fn list_recent_is_newest_first_and_counts_track_sync_state() {
    let (dir, store) = store().await;
    let media = dir.path().join("media");
    let first = store.ingest(&media, "s", JPEG).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let second = store.ingest(&media, "s", JPEG).await.unwrap();

    let ids: Vec<_> = store
        .list_recent(10)
        .await
        .unwrap()
        .into_iter()
        .map(|c| c.client_id)
        .collect();
    assert_eq!(ids, [second.client_id.clone(), first.client_id.clone()]);

    store
        .mark_synced(&first.client_id, "2026-09-30T12:00:00.000Z")
        .await
        .unwrap();
    let counts = store.counts().await.unwrap();
    assert_eq!((counts.pending, counts.synced), (1, 1));
}
