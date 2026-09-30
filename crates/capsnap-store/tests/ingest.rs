mod common;

use capsnap_store::{
    CaptureDetails, DEMO_LOCATION_ID, IngestError, LocalStore, MAX_MEDIA_BYTES, media_path,
    sniff_mime, thumb_path,
};
use common::*;
use sha2::{Digest, Sha256};

async fn store() -> (tempfile::TempDir, LocalStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = LocalStore::open(&dir.path().join("a.db")).await.unwrap();
    (dir, store)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn files_in(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn ingest_stores_the_processed_photo_and_thumbnail_and_queues_a_pending_capture() {
    let (dir, store) = store().await;
    let media = dir.path().join("media");
    let source = with_segment(&jpeg(3000, 2000), &exif_app1());

    let capture = store
        .ingest(&media, "staff-1", CaptureDetails::default(), source.clone())
        .await
        .unwrap();

    // Orientation 6 applied, then fitted to 2048.
    assert_eq!(
        (capture.media_width, capture.media_height),
        (Some(1365), Some(2048))
    );
    assert_eq!(capture.media_mime.as_deref(), Some("image/jpeg"));
    assert_eq!(capture.sync_state, "pending");
    assert_eq!(capture.staff_id, "staff-1");
    assert_eq!(capture.client_id.len(), 36, "uuid v4");
    // RFC 3339 UTC with milliseconds, so string order is time order.
    assert!(
        capture.capture_time_utc.len() == 24 && capture.capture_time_utc.ends_with('Z'),
        "{}",
        capture.capture_time_utc
    );
    assert_eq!(capture.dish_name, None);
    assert_eq!(capture.table_label, None);

    // The digest is of the stored (processed) bytes, and the original never hits disk.
    let stored = std::fs::read(media_path(&media, &capture.media_sha256, "image/jpeg")).unwrap();
    assert_eq!(hex(&Sha256::digest(&stored)), capture.media_sha256);
    assert_eq!(capture.media_bytes, Some(stored.len() as i64));
    assert_ne!(stored, source);
    assert!(thumb_path(&media, &capture.media_sha256).exists());
    assert_eq!(
        files_in(&media),
        [
            format!("{}.jpg", capture.media_sha256),
            format!("{}.thumb.jpg", capture.media_sha256)
        ]
    );
}

#[tokio::test]
async fn a_chosen_dish_and_table_label_are_recorded_with_the_dish_name_copied() {
    let (dir, store) = store().await;
    let media = dir.path().join("media");
    assert!(store.seed_demo_if_empty().await.unwrap());

    let capture = store
        .ingest(
            &media,
            "staff-1",
            CaptureDetails {
                menu_item_id: Some("demo-saffron-butter-cod"),
                table_label: Some("  12B "),
            },
            jpeg(400, 300),
        )
        .await
        .unwrap();

    assert_eq!(
        capture.menu_item_id.as_deref(),
        Some("demo-saffron-butter-cod")
    );
    assert_eq!(capture.dish_name.as_deref(), Some("Saffron butter cod"));
    assert_eq!(capture.location_id.as_deref(), Some(DEMO_LOCATION_ID));
    assert_eq!(capture.table_label.as_deref(), Some("12B"));
}

#[tokio::test]
async fn the_same_photo_twice_is_two_captures_but_one_stored_file() {
    let (dir, store) = store().await;
    let media = dir.path().join("media");
    let photo = jpeg(640, 480);
    let a = store
        .ingest(&media, "staff-1", CaptureDetails::default(), photo.clone())
        .await
        .unwrap();
    let b = store
        .ingest(&media, "staff-1", CaptureDetails::default(), photo)
        .await
        .unwrap();

    assert_ne!(a.client_id, b.client_id);
    assert_eq!(a.media_sha256, b.media_sha256);
    assert_eq!(store.counts().await.unwrap().pending, 2);
    assert_eq!(files_in(&media).len(), 2, "one photo + one thumbnail");
}

#[tokio::test]
async fn rejects_bad_input_without_side_effects() {
    let (dir, store) = store().await;
    let media = dir.path().join("media");
    store.seed_demo_if_empty().await.unwrap();
    let none = CaptureDetails::default();
    let ingest = |details, bytes| store.ingest(&media, "s", details, bytes);

    assert!(matches!(
        ingest(none, vec![]).await,
        Err(IngestError::Empty)
    ));
    let huge = vec![0xFF; MAX_MEDIA_BYTES + 1];
    assert!(matches!(
        ingest(none, huge).await,
        Err(IngestError::TooLarge(_))
    ));
    let gif = b"GIF89a\x01\x00\x01\x00".to_vec();
    assert!(matches!(
        ingest(none, gif).await,
        Err(IngestError::UnsupportedFormat)
    ));
    let fake = vec![
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00, 0xFF, 0xD9,
    ];
    assert!(matches!(
        ingest(none, fake).await,
        Err(IngestError::Undecodable(_))
    ));

    let unknown = CaptureDetails {
        menu_item_id: Some("no-such-dish"),
        table_label: None,
    };
    assert!(matches!(
        ingest(unknown, jpeg(10, 10)).await,
        Err(IngestError::UnknownDish)
    ));
    for label in ["seventeen chars!!", "Table 12 (patio)", "12\nB", "<b>"] {
        let details = CaptureDetails {
            menu_item_id: None,
            table_label: Some(label),
        };
        assert!(
            matches!(
                ingest(details, jpeg(10, 10)).await,
                Err(IngestError::InvalidTableLabel)
            ),
            "{label:?} should be refused"
        );
    }

    assert_eq!(store.counts().await.unwrap(), Default::default());
    assert!(!media.exists());
}

#[test]
fn sniffs_formats_from_signatures() {
    assert_eq!(sniff_mime(&jpeg(2, 2)), Some("image/jpeg"));
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
    let none = CaptureDetails::default();
    let first = store.ingest(&media, "s", none, jpeg(20, 20)).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let second = store.ingest(&media, "s", none, jpeg(20, 20)).await.unwrap();

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
