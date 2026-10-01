use capsnap_store::{
    CaptureDetails, DEMO_LOCATION_ID, DEMO_LOCATION_NAME, IngestError, LocalStore, MenuSource,
    NewMenuItem, Setting,
};

async fn store() -> (tempfile::TempDir, LocalStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = LocalStore::open(&dir.path().join("a.db")).await.unwrap();
    (dir, store)
}

fn tiny_jpeg() -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    image::RgbImage::new(8, 8)
        .write_to(&mut out, image::ImageFormat::Jpeg)
        .unwrap();
    out.into_inner()
}

#[tokio::test]
async fn demo_seed_runs_once_and_every_row_is_labelled_demo() {
    let (_dir, store) = store().await;
    assert!(store.seed_demo_if_empty().await.unwrap());
    assert!(
        !store.seed_demo_if_empty().await.unwrap(),
        "second run is a no-op"
    );

    let menu = store.menu_items(DEMO_LOCATION_ID).await.unwrap();
    let names: Vec<_> = menu.iter().take(3).map(|m| m.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Saffron butter cod",
            "Wild mushroom risotto",
            "Charred heritage carrots"
        ],
        "mockup 02 order"
    );
    assert!(menu.iter().all(|m| m.source == "demo" && m.is_active));
    assert!(menu.iter().any(|m| m.category == "Starters"));
    assert_eq!(
        store
            .setting(Setting::LocationName)
            .await
            .unwrap()
            .as_deref(),
        Some(DEMO_LOCATION_NAME)
    );
}

#[tokio::test]
async fn replacing_a_menu_keeps_order_hides_inactive_dishes_and_old_captures_keep_their_name() {
    let (dir, store) = store().await;
    store.seed_demo_if_empty().await.unwrap();
    let before = store
        .ingest(
            &dir.path().join("media"),
            "s",
            CaptureDetails {
                menu_item_id: Some("demo-tuna-crudo"),
                table_label: None,
            },
            tiny_jpeg(),
        )
        .await
        .unwrap();

    store
        .replace_menu(
            DEMO_LOCATION_ID,
            MenuSource::Server,
            &[
                NewMenuItem {
                    id: "b",
                    name: "Bao",
                    category: "Starters",
                    is_active: true,
                },
                NewMenuItem {
                    id: "a",
                    name: "Apple tart",
                    category: "Desserts",
                    is_active: true,
                },
                NewMenuItem {
                    id: "off",
                    name: "Off tonight",
                    category: "Mains",
                    is_active: false,
                },
            ],
        )
        .await
        .unwrap();

    let menu = store.menu_items(DEMO_LOCATION_ID).await.unwrap();
    let ids: Vec<_> = menu.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["b", "a"], "given order, inactive hidden");
    assert!(menu.iter().all(|m| m.source == "server"));
    assert!(store.menu_items("other-location").await.unwrap().is_empty());

    let kept = store.get(&before.client_id).await.unwrap().unwrap();
    assert_eq!(kept.dish_name.as_deref(), Some("Tuna crudo, yuzu kosho"));

    let off = CaptureDetails {
        menu_item_id: Some("off"),
        table_label: None,
    };
    assert!(matches!(
        store
            .ingest(&dir.path().join("media"), "s", off, tiny_jpeg())
            .await,
        Err(IngestError::UnknownDish)
    ));
}

#[tokio::test]
async fn settings_upsert_and_clear() {
    let (_dir, store) = store().await;
    assert_eq!(
        store.setting(Setting::StaffDisplayName).await.unwrap(),
        None
    );
    store
        .set_setting(Setting::StaffDisplayName, "Maya")
        .await
        .unwrap();
    store
        .set_setting(Setting::StaffDisplayName, "Rosa")
        .await
        .unwrap();
    assert_eq!(
        store
            .setting(Setting::StaffDisplayName)
            .await
            .unwrap()
            .as_deref(),
        Some("Rosa")
    );
    store
        .clear_setting(Setting::StaffDisplayName)
        .await
        .unwrap();
    assert_eq!(
        store.setting(Setting::StaffDisplayName).await.unwrap(),
        None
    );
    // The column CHECK bounds what a setting can hold.
    assert!(
        store
            .set_setting(Setting::StaffDisplayName, &"x".repeat(257))
            .await
            .is_err()
    );
}
