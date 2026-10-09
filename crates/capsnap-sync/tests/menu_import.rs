//! Paste a link, review the dishes, save them: the phone's client against the real server, with a
//! pretend restaurant website on 127.0.0.1.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::Duration;

use capsnap_store::{LocalStore, Setting};
use capsnap_sync::{IMPORT_CONSENT, MenuChoice, SyncError, import_preview, save_menu, sign_in};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const PASSWORD: &str = "correct horse battery";

const HOME: &str = r#"<html><head><title>Bistro Test</title>
<script type="application/ld+json">{"@context":"https://schema.org","@type":"Restaurant","name":"Bistro Test","hasMenu":"/menu"}</script>
</head><body><a href="/menu">Menu</a></body></html>"#;

const MENU: &str = r#"<html><head><script type="application/ld+json">{"@context":"https://schema.org","@type":"Menu","hasMenuSection":[
 {"@type":"MenuSection","name":"Desserts","hasMenuItem":{"@type":"MenuItem","name":"Olive oil cake"}},
 {"@type":"MenuSection","name":"Mains","hasMenuItem":[
   {"@type":"MenuItem","name":"Short rib","offers":{"@type":"Offer","price":"32","priceCurrency":"USD"}},
   {"@type":"MenuItem","name":"Seared salmon"}]},
 {"@type":"MenuSection","name":"Starters","hasMenuItem":{"@type":"MenuItem","name":"Burrata"}}]}</script></head><body></body></html>"#;

/// Serves the pretend site: "/" and "/menu" as HTML, robots.txt absent (nothing forbidden).
async fn restaurant_site() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        loop {
            let Ok((mut conn, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                let n = conn.read(&mut buf).await.unwrap_or(0);
                let head = String::from_utf8_lossy(&buf[..n]).into_owned();
                let path = head.split_whitespace().nth(1).unwrap_or("/").to_owned();
                let (status, body) = match path.as_str() {
                    "/" => (200, HOME),
                    "/menu" => (200, MENU),
                    _ => (404, "not found"),
                };
                let out = format!(
                    "HTTP/1.1 {status} X\r\nConnection: close\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                );
                let _ = conn.write_all(out.as_bytes()).await;
                let _ = conn.shutdown().await;
            });
        }
    });
    port
}

struct Rig {
    _dir: tempfile::TempDir,
    base: String,
    site: String,
    phone: LocalStore,
}

async fn rig() -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let mut server = capsnap_server::AppState::open(capsnap_server::Config::for_dir(
        &dir.path().join("server"),
        &base,
    ))
    .await
    .unwrap();
    // The pretend site lives on loopback; production refuses that, the test pins it on purpose.
    server.onboard = Arc::new(capsnap_onboard::Config {
        allow_nonstandard_ports: true,
        ip_policy: |ip| ip.is_loopback(),
        resolve_overrides: vec![("bistro.test".into(), IpAddr::V4(Ipv4Addr::LOCALHOST))],
        timeout: Duration::from_secs(3),
        ..capsnap_onboard::Config::default()
    });
    let pool = &server.pool;
    use capsnap_server::admin::*;
    create_org(pool, "bistro", "Bistro Test").await.unwrap();
    let location = create_location(pool, "bistro", "Bistro Test", "America/Chicago")
        .await
        .unwrap();
    for (login, role) in [("ada@bistro", "admin"), ("maya@bistro", "server")] {
        create_staff(
            pool,
            "bistro",
            login,
            login,
            role,
            PASSWORD,
            std::slice::from_ref(&location),
        )
        .await
        .unwrap();
    }
    tokio::spawn(capsnap_server::serve_listener(listener, server.clone()));
    let port = restaurant_site().await;
    let phone = LocalStore::open(&dir.path().join("phone.db"))
        .await
        .unwrap();
    phone.seed_demo_if_empty().await.unwrap();
    Rig {
        _dir: dir,
        base,
        site: format!("http://bistro.test:{port}/"),
        phone,
    }
}

async fn menu_of(phone: &LocalStore) -> Vec<(String, String)> {
    let loc = phone.setting(Setting::LocationId).await.unwrap().unwrap();
    phone
        .menu_items(&loc)
        .await
        .unwrap()
        .into_iter()
        .map(|m| (m.category, m.name))
        .collect()
}

#[test]
fn the_consent_text_matches_the_server() {
    assert_eq!(IMPORT_CONSENT, capsnap_onboard::CONSENT_STATEMENT);
}

/// The words beside the checkbox are what the owner agrees to; the phone sends `IMPORT_CONSENT`.
/// They must be the same words.
#[test]
fn the_screen_shows_the_consent_text_that_is_sent() {
    let screen = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../app/src/lib/components/MenuImport.svelte"
    ))
    .unwrap();
    assert!(
        screen.contains(IMPORT_CONSENT),
        "MenuImport.svelte shows different words"
    );
}

#[tokio::test]
async fn an_admin_pastes_a_link_reviews_the_dishes_and_saves_the_menu() {
    let r = rig().await;
    sign_in(&r.phone, &r.base, "ada@bistro", PASSWORD, "Pixel 9")
        .await
        .unwrap();
    assert!(
        menu_of(&r.phone).await.is_empty(),
        "a new restaurant starts with no dishes"
    );

    let draft = import_preview(&r.phone, &format!("  {}  ", r.site))
        .await
        .unwrap();
    assert_eq!(draft.business_name.as_deref(), Some("Bistro Test"));
    assert_eq!(draft.pages_read.len(), 2, "home page, then the menu page");
    let found: Vec<_> = draft
        .menu
        .iter()
        .map(|d| (d.category.as_str(), d.name.as_str()))
        .collect();
    assert!(
        found.contains(&("Mains", "Short rib")) && found.contains(&("Desserts", "Olive oil cake"))
    );
    assert_eq!(
        draft
            .menu
            .iter()
            .find(|d| d.name == "Short rib")
            .unwrap()
            .price
            .as_deref(),
        Some("32 USD"),
        "the price is shown to the reviewer"
    );
    assert!(
        menu_of(&r.phone).await.is_empty(),
        "the draft saves nothing"
    );

    // The owner drops a dish and saves; the phone shows the new menu at once.
    let keep: Vec<MenuChoice> = draft
        .menu
        .iter()
        .filter(|d| d.name != "Seared salmon")
        .map(|d| MenuChoice {
            name: d.name.clone(),
            category: d.category.clone(),
        })
        .collect();
    save_menu(&r.phone, keep.clone()).await.unwrap();
    let mut saved = menu_of(&r.phone).await;
    saved.sort();
    assert_eq!(
        saved,
        [
            ("Desserts".to_owned(), "Olive oil cake".to_owned()),
            ("Mains".to_owned(), "Short rib".to_owned()),
            ("Starters".to_owned(), "Burrata".to_owned()),
        ]
    );
}

#[tokio::test]
async fn a_server_member_cannot_import_and_a_signed_out_phone_cannot_either() {
    let r = rig().await;
    assert_eq!(
        import_preview(&r.phone, &r.site).await.unwrap_err(),
        SyncError::SignedOut
    );
    assert_eq!(
        save_menu(&r.phone, vec![]).await.unwrap_err(),
        SyncError::SignedOut
    );

    sign_in(&r.phone, &r.base, "maya@bistro", PASSWORD, "Pixel 9")
        .await
        .unwrap();
    let denied = import_preview(&r.phone, &r.site).await.unwrap_err();
    assert!(
        matches!(denied, SyncError::Refused { status: 403, .. }),
        "{denied:?}"
    );
    let denied = save_menu(
        &r.phone,
        vec![MenuChoice {
            name: "Burrata".into(),
            category: "Starters".into(),
        }],
    )
    .await
    .unwrap_err();
    assert!(
        matches!(denied, SyncError::Refused { status: 403, .. }),
        "{denied:?}"
    );
}

#[tokio::test]
async fn an_address_made_of_numbers_is_refused_in_plain_words() {
    let r = rig().await;
    sign_in(&r.phone, &r.base, "ada@bistro", PASSWORD, "Pixel 9")
        .await
        .unwrap();
    // The importer reads a restaurant's website by its name, never an address made of numbers.
    let err = import_preview(&r.phone, "https://127.0.0.1/")
        .await
        .unwrap_err();
    assert!(matches!(err, SyncError::Refused { .. }), "{err:?}");
    assert_eq!(err.to_string(), "use the website's name, not an IP address");
}
