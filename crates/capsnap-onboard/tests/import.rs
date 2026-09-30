mod common;

use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

use capsnap_onboard::{
    CONSENT_STATEMENT, Config, ImportError, import, is_denied_host, is_public_ip, parse_site_url,
};
use common::*;

const HOME: &str = r#"<!doctype html><html><head><title>Atelier No. 8 — Home</title>
<script type="application/ld+json">{"@context":"https://schema.org","@graph":[
 {"@type":"WebSite","name":"Atelier site"},
 {"@type":["Restaurant"],"name":"Atelier No. 8","telephone":"+1 312 555 0108","url":"/",
  "servesCuisine":["Asian fusion","Midwestern"],
  "address":{"@type":"PostalAddress","streetAddress":"8 W Kinzie St","addressLocality":"Chicago",
             "addressRegion":"IL","postalCode":"60654","addressCountry":{"@type":"Country","name":"US"}},
  "openingHoursSpecification":[{"dayOfWeek":["https://schema.org/Tuesday","Wednesday"],"opens":"17:00:00","closes":"22:00:00"}],
  "hasMenu":"/menu","image":"/hero.jpg"}]}</script>
</head><body><h1>Welcome</h1><img src="/hero.jpg" alt="">
<a href="/menu">Our menu</a> <a href="/private-menu">Staff menu</a> <a href="/about">About</a></body></html>"#;

const MENU: &str = r#"<html><head>
<script type="application/ld+json">{ this is not json }</script>
<script type="application/ld+json">{"@context":"https://schema.org","@type":"Menu","name":"Dinner","hasMenuSection":[
 {"@type":"MenuSection","name":"Mains","hasMenuItem":[
   {"@type":"MenuItem","name":"Saffron butter cod","description":"Brown butter,   saffron beurre blanc",
    "offers":{"@type":"Offer","price":"34","priceCurrency":"USD"},"image":"/cod.jpg"},
   {"@type":"MenuItem","name":"Wild   mushroom\n risotto"}]},
 {"@type":"MenuSection","name":"Starters","hasMenuItem":{"@type":"MenuItem","name":"Pork belly bao"}}]}</script>
</head><body><img src="/cod.jpg"></body></html>"#;

async fn atelier() -> Server {
    Server::start(vec![
        (
            "atelier8.test/robots.txt",
            text(200, "User-agent: *\nDisallow: /private\n"),
        ),
        ("atelier8.test/", html(HOME)),
        ("atelier8.test/menu", html(MENU)),
        ("atelier8.test/private-menu", html(MENU)),
    ])
    .await
}

#[tokio::test]
async fn reads_business_and_menu_from_structured_data() {
    let server = atelier().await;
    let site = server.url("atelier8.test", "/");

    let draft = import(&site, &consent(&site), &config()).await.unwrap();

    assert_eq!(draft.business_name.as_deref(), Some("Atelier No. 8"));
    assert_eq!(draft.telephone.as_deref(), Some("+1 312 555 0108"));
    let address = draft.address.as_ref().unwrap();
    assert_eq!(address.street.as_deref(), Some("8 W Kinzie St"));
    assert_eq!(address.locality.as_deref(), Some("Chicago"));
    assert_eq!(address.country.as_deref(), Some("US"));
    assert_eq!(draft.cuisine, ["Asian fusion", "Midwestern"]);
    assert_eq!(draft.opening_hours, ["Tu,We 17:00–22:00"]);

    let menu: Vec<_> = draft
        .menu
        .iter()
        .map(|m| (m.category.as_str(), m.name.as_str()))
        .collect();
    assert_eq!(
        menu,
        [
            ("Mains", "Saffron butter cod"),
            ("Mains", "Wild mushroom risotto"),
            ("Starters", "Pork belly bao")
        ]
    );
    assert_eq!(draft.menu[0].price.as_deref(), Some("34 USD"));
    assert_eq!(
        draft.menu[0].description.as_deref(),
        Some("Brown butter, saffron beurre blanc")
    );

    // Declared menu read; robots-disallowed page skipped with a note; no images fetched.
    assert_eq!(
        draft.pages_read,
        [site.clone(), server.url("atelier8.test", "/menu")]
    );
    let hits = server.hits();
    assert_eq!(
        hits,
        [
            "atelier8.test/robots.txt",
            "atelier8.test/",
            "atelier8.test/menu"
        ]
    );
    assert!(!hits.iter().any(|h| h.ends_with(".jpg")));
    assert!(
        draft
            .notes
            .iter()
            .any(|n| n.contains("/private-menu") && n.contains("robots.txt")),
        "{:?}",
        draft.notes
    );

    // The consent travels with the draft so the server can record it.
    let json = serde_json::to_value(&draft).unwrap();
    assert_eq!(json["consent"]["requestedBy"], "staff-7");
    assert_eq!(json["consent"]["acceptedStatement"], CONSENT_STATEMENT);
}

#[tokio::test]
async fn falls_back_to_open_graph_tel_links_and_microdata() {
    let home = r#"<html><head><meta property="og:site_name" content="Rosa's   Kitchen">
<title>Home | Rosa's</title></head><body>
<a href="tel:+13125550199">Call us</a>
<div itemscope itemtype="https://schema.org/MenuSection"><h2 itemprop="name">Desserts</h2>
  <div itemscope itemtype="https://schema.org/MenuItem"><span itemprop="name">Apple tart</span></div>
  <div itemscope itemtype="https://schema.org/MenuItem"><span itemprop="name">Sorghum pie</span></div>
</div><a href="/menu">Menu</a></body></html>"#;
    let server = Server::start(vec![("atelier8.test/", html(home))]).await;
    let site = server.url("atelier8.test", "/");

    let draft = import(&site, &consent(&site), &config()).await.unwrap();

    assert_eq!(draft.business_name.as_deref(), Some("Rosa's Kitchen"));
    assert!(draft.notes.iter().any(|n| n.contains("site name")));
    assert_eq!(draft.telephone.as_deref(), Some("+13125550199"));
    let menu: Vec<_> = draft
        .menu
        .iter()
        .map(|m| (m.category.as_str(), m.name.as_str()))
        .collect();
    assert_eq!(
        menu,
        [("Desserts", "Apple tart"), ("Desserts", "Sorghum pie")]
    );
    // The home page had a menu, so no further page was requested; robots.txt 404 = allowed.
    assert_eq!(
        server.hits(),
        ["atelier8.test/robots.txt", "atelier8.test/"]
    );
}

#[tokio::test]
async fn robots_rules_are_honoured() {
    // Disallow everything: only robots.txt itself is read.
    let server = Server::start(vec![
        (
            "atelier8.test/robots.txt",
            text(200, "User-agent: *\nDisallow: /\n"),
        ),
        ("atelier8.test/", html(HOME)),
    ])
    .await;
    let site = server.url("atelier8.test", "/");
    assert_eq!(
        import(&site, &consent(&site), &config()).await,
        Err(ImportError::RobotsDisallowed("/".into()))
    );
    assert_eq!(server.hits(), ["atelier8.test/robots.txt"]);

    // A named group for us overrides the wildcard.
    let server = Server::start(vec![
        (
            "atelier8.test/robots.txt",
            text(
                200,
                "User-agent: *\nDisallow: /\n\nUser-agent: NoCapSnapBot\nAllow: /\n",
            ),
        ),
        ("atelier8.test/", html(HOME)),
    ])
    .await;
    let site = server.url("atelier8.test", "/");
    assert!(import(&site, &consent(&site), &config()).await.is_ok());

    // A server error on robots.txt means "read nothing" (RFC 9309 §2.3.1.4).
    let server = Server::start(vec![
        ("atelier8.test/robots.txt", text(503, "busy")),
        ("atelier8.test/", html(HOME)),
    ])
    .await;
    let site = server.url("atelier8.test", "/");
    assert_eq!(
        import(&site, &consent(&site), &config()).await,
        Err(ImportError::RobotsUnavailable)
    );
    assert_eq!(server.hits(), ["atelier8.test/robots.txt"]);
}

#[tokio::test]
async fn redirects_stay_on_the_site_and_are_bounded() {
    let server = Server::start(vec![
        ("www.atelier8.test/", redirect("/home")),
        ("www.atelier8.test/home", html("<title>Atelier</title>")),
        ("atelier8.test/away", redirect("http://evil.test/")),
        ("atelier8.test/r1", redirect("/r2")),
        ("atelier8.test/r2", redirect("/r3")),
        ("atelier8.test/r3", redirect("/r4")),
        ("atelier8.test/r4", redirect("/r5")),
    ])
    .await;
    let cfg = config();

    let site = server.url("www.atelier8.test", "/");
    let draft = import(&site, &consent(&site), &cfg).await.unwrap();
    assert_eq!(draft.pages_read, [server.url("www.atelier8.test", "/home")]);

    let site = server.url("atelier8.test", "/away");
    assert!(matches!(
        import(&site, &consent(&site), &cfg).await,
        Err(ImportError::OffSite(_))
    ));
    assert!(
        !server.hits().iter().any(|h| h.starts_with("evil.test")),
        "never contacted the other site"
    );

    let site = server.url("atelier8.test", "/r1");
    assert_eq!(
        import(&site, &consent(&site), &cfg).await,
        Err(ImportError::TooManyRedirects)
    );
}

#[tokio::test]
async fn size_type_and_time_limits() {
    let big = "<p>x</p>".repeat(1000);
    let mut unsized_big = html(&big);
    unsized_big.no_length = true;
    let mut pdf = html("%PDF-1.7");
    pdf.headers = vec![("Content-Type".into(), "application/pdf".into())];
    let hang = Route {
        hang: true,
        ..Default::default()
    };
    let server = Server::start(vec![
        ("atelier8.test/big", html(&big)),
        ("atelier8.test/big-unsized", unsized_big),
        ("atelier8.test/menu.pdf", pdf),
        ("atelier8.test/slow", hang),
    ])
    .await;
    let cfg = Config {
        max_bytes: 4000,
        timeout: Duration::from_millis(400),
        ..config()
    };
    let run = |path: &str| {
        let site = server.url("atelier8.test", path);
        let cfg = cfg.clone();
        async move { import(&site, &consent(&site), &cfg).await }
    };

    assert_eq!(run("/big").await, Err(ImportError::TooLarge)); // declared length
    assert_eq!(run("/big-unsized").await, Err(ImportError::TooLarge)); // counted while streaming
    assert_eq!(
        run("/menu.pdf").await,
        Err(ImportError::NotHtml("application/pdf".into()))
    );
    let started = std::time::Instant::now();
    assert_eq!(run("/slow").await, Err(ImportError::Timeout));
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[tokio::test]
async fn follows_at_most_the_page_limit() {
    let links: String = (1..=6)
        .map(|i| format!(r#"<a href="/menu-{i}">Menu {i}</a>"#))
        .collect();
    let menu_page = |i: u32| {
        html(&format!(
            r#"<script type="application/ld+json">{{"@type":"MenuItem","name":"Dish {i}"}}</script>"#
        ))
    };
    let home = format!("<title>Atelier</title>{links}");
    let server = Server::start(vec![
        ("atelier8.test/", html(&home)),
        ("atelier8.test/menu-1", menu_page(1)),
        ("atelier8.test/menu-2", menu_page(2)),
        ("atelier8.test/menu-3", menu_page(3)),
        ("atelier8.test/menu-4", menu_page(4)),
    ])
    .await;
    let site = server.url("atelier8.test", "/");
    let cfg = Config {
        max_pages: 3,
        ..config()
    };

    let draft = import(&site, &consent(&site), &cfg).await.unwrap();

    assert_eq!(draft.pages_read.len(), 3);
    assert_eq!(
        draft
            .menu
            .iter()
            .map(|m| m.name.as_str())
            .collect::<Vec<_>>(),
        ["Dish 1", "Dish 2"]
    );
    assert!(draft.notes.iter().any(|n| n.contains("page limit")));
    assert_eq!(server.hits().len(), 1 + 3, "robots.txt + 3 pages");
}

#[tokio::test]
async fn nothing_is_fetched_without_matching_consent() {
    let server = atelier().await;
    let site = server.url("atelier8.test", "/");
    let cfg = config();

    let mut wrong_text = consent(&site);
    wrong_text.accepted_statement = "ok".into();
    let mut nobody = consent(&site);
    nobody.requested_by = "  ".into();
    let other_site = consent(&server.url("evil.test", "/"));
    let mut undated = consent(&site);
    undated.accepted_at_utc = String::new();

    for c in [wrong_text, nobody, other_site, undated] {
        assert_eq!(import(&site, &c, &cfg).await, Err(ImportError::NoConsent));
    }
    assert!(server.hits().is_empty());
}

#[tokio::test]
async fn the_default_policy_refuses_internal_addresses_before_connecting() {
    let server = atelier().await;
    let site = server.url("atelier8.test", "/");
    let pinned = |ip: IpAddr| Config {
        allow_nonstandard_ports: true,
        resolve_overrides: vec![("atelier8.test".into(), ip)],
        ..Config::default() // production IP policy
    };
    for ip in [
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, 8)),
        IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254)),
    ] {
        assert_eq!(
            import(&site, &consent(&site), &pinned(ip)).await,
            Err(ImportError::BlockedAddress("atelier8.test".into())),
            "{ip}"
        );
    }
    assert!(server.hits().is_empty(), "the server was never contacted");
}

#[test]
fn only_globally_routable_addresses_are_public() {
    let blocked = [
        "127.0.0.1",
        "10.1.2.3",
        "172.16.0.1",
        "192.168.1.1",
        "169.254.169.254",
        "100.64.0.1",
        "0.0.0.0",
        "224.0.0.1",
        "255.255.255.255",
        "192.0.2.1",
        "198.18.0.1",
        "240.0.0.1",
        "::1",
        "::",
        "fc00::1",
        "fd12::1",
        "fe80::1",
        "::ffff:127.0.0.1",
        "::ffff:10.0.0.1",
        "64:ff9b::a00:1",
        "2002:a00:1::1",
        "2001:db8::1",
    ];
    for ip in blocked {
        assert!(!is_public_ip(ip.parse().unwrap()), "{ip} must be refused");
    }
    for ip in [
        "93.184.216.34",
        "1.1.1.1",
        "8.8.8.8",
        "2606:4700:4700::1111",
        "::ffff:93.184.216.34",
    ] {
        assert!(is_public_ip(ip.parse().unwrap()), "{ip} is public");
    }
}

#[test]
fn only_plain_website_addresses_are_accepted() {
    let (url, site) = parse_site_url("  www.atelier8.com  ", false).unwrap();
    assert_eq!(url.as_str(), "https://www.atelier8.com/");
    assert_eq!(site.base(), "atelier8.com");
    for (inside, expect) in [
        ("https://atelier8.com/menu", true),
        ("https://menu.atelier8.com/", true),
        ("http://www.atelier8.com/x", true),
        ("https://atelier8.com.evil.io/", false),
        ("https://evilatelier8.com/", false),
        ("ftp://atelier8.com/", false),
    ] {
        assert_eq!(site.contains(&inside.parse().unwrap()), expect, "{inside}");
    }

    for bad in [
        "",
        "ftp://atelier8.com",
        "file:///etc/passwd",
        "http://127.0.0.1/",
        "http://[::1]/",
        "http://10.0.0.1/",
        "https://user:pw@atelier8.com/",
        "atelier8",
        "https://atelier8.com:8080/",
    ] {
        assert!(
            matches!(parse_site_url(bad, false), Err(ImportError::InvalidUrl(_))),
            "{bad:?}"
        );
    }
}

#[test]
fn review_delivery_social_and_search_sites_are_refused() {
    for host in [
        "www.yelp.com",
        "yelp.co.uk",
        "www.tripadvisor.com",
        "maps.google.com",
        "www.google.de",
        "goo.gl",
        "maps.app.goo.gl",
        "g.page",
        "www.opentable.com",
        "www.doordash.com",
        "www.instagram.com",
        "m.facebook.com",
        "x.com",
    ] {
        assert!(is_denied_host(host), "{host}");
        assert_eq!(
            parse_site_url(&format!("https://{host}/biz/atelier"), false),
            Err(ImportError::NotOwnSite)
        );
    }
    for host in [
        "atelier8.com",
        "yelpcafe.com",
        "googleplex-diner.com",
        "rosas.kitchen",
    ] {
        assert!(!is_denied_host(host), "{host}");
    }
}
