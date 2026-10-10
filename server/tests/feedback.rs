//! `GET /locations/{id}/feedback`: staff read what guests said.

mod common;

use axum::http::StatusCode;
use capsnap_server::admin;
use common::*;
use serde_json::{Value, json};

/// One captured dish and one guest answer to it.
async fn answered(
    w: &World,
    token: &str,
    n: u8,
    location: &str,
    dish: Option<&str>,
    body: Value,
) -> String {
    let app = app(w);
    let media = upload_ok(&app, token, phone_jpeg(64, 48, n)).await;
    let menu_item = match dish {
        Some(name) => Some(menu_item(&app, token, location, name).await),
        None => None,
    };
    let r = common::json(
        &app,
        axum::http::Method::POST,
        "/api/v1/captures",
        Some(token),
        json!({
            "clientId": format!("client-{n:04}-xxxx"), "mediaId": media, "locationId": location,
            "menuItemId": menu_item, "captureTimeUtc": "2026-09-30T19:24:00.000Z"
        }),
    )
    .await;
    assert!(r.status.is_success(), "{}", r.error());
    let guest_token = token_from(r.json()["guestUrl"].as_str().unwrap());
    let c = cookie(&guest_session(&app, &guest_token).await);
    let f = feedback(&app, &c, body).await;
    assert!(f.status.is_success(), "{}", f.error());
    guest_token
}

fn path(location: &str, query: &str) -> String {
    format!("/api/v1/locations/{location}/feedback{query}")
}

#[tokio::test]
async fn only_admins_and_managers_read_feedback_and_only_for_their_own_restaurant() {
    let w = world().await;
    let app = app(&w);
    let admin_token = sign_in(&app, "ada@atelier").await;
    let chef = sign_in(&app, "chen@atelier").await;
    let server = sign_in(&app, "bo@bistro").await;
    answered(
        &w,
        &admin_token,
        1,
        &w.loc_a1,
        Some("Saffron butter cod"),
        json!({ "rating": 4, "comment": "Lovely" }),
    )
    .await;

    assert_eq!(
        get(&app, &path(&w.loc_a1, ""), None).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        get(&app, &path(&w.loc_a1, ""), Some(&chef)).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        get(&app, &path(&w.loc_b, ""), Some(&server)).await.status,
        StatusCode::FORBIDDEN
    );
    // Another restaurant's location looks like it does not exist.
    assert_eq!(
        get(&app, &path(&w.loc_b, ""), Some(&admin_token))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let ok = get(&app, &path(&w.loc_a1, ""), Some(&admin_token)).await;
    assert_eq!(ok.status, StatusCode::OK);
    assert_eq!(ok.json()["summary"]["count"], 1);
}

#[tokio::test]
async fn the_summary_counts_the_whole_period_and_items_carry_the_dish_and_comment() {
    let w = world().await;
    let app = app(&w);
    let token = sign_in(&app, "ada@atelier").await;
    for (n, rating, comment, dish) in [
        (1u8, 5, Some("Perfect"), Some("Saffron butter cod")),
        (2, 4, None, Some("Pork belly bao")),
        (3, 1, Some("Cold"), Some("Saffron butter cod")),
        (4, 5, None, None),
    ] {
        let body = match comment {
            Some(c) => json!({ "rating": rating, "comment": c }),
            None => json!({ "rating": rating }),
        };
        answered(&w, &token, n, &w.loc_a1, dish, body).await;
    }
    // The other location of the same restaurant is not mixed in.
    answered(&w, &token, 5, &w.loc_a2, None, json!({ "rating": 2 })).await;

    let r = get(&app, &path(&w.loc_a1, ""), Some(&token)).await.json();
    assert_eq!(r["days"], 30);
    assert_eq!(r["summary"]["count"], 4);
    assert_eq!(r["summary"]["distribution"], json!([1, 0, 0, 1, 2]));
    assert_eq!(r["summary"]["average"], 3.75);
    let items = r["items"].as_array().unwrap();
    assert_eq!(items.len(), 4);
    assert!(r["nextBefore"].is_null());
    // Newest first; the last answer was given for the dish with no name.
    assert_eq!(items[0]["rating"], 5);
    assert!(items[0]["dishName"].is_null());
    assert_eq!(items[1]["comment"], "Cold");
    assert_eq!(items[1]["dishName"], "Saffron butter cod");
    // Nothing identifies a guest or a link.
    let keys: Vec<&String> = items[0].as_object().unwrap().keys().collect();
    for k in keys {
        assert!(
            ["id", "rating", "comment", "createdAt", "dishName"].contains(&k.as_str()),
            "unexpected field {k}"
        );
    }

    let none = get(&app, &path(&w.loc_a2, ""), Some(&token)).await.json();
    assert_eq!(none["summary"]["count"], 1);
    assert_eq!(none["summary"]["distribution"], json!([0, 1, 0, 0, 0]));
}

#[tokio::test]
async fn pages_walk_back_without_skipping_or_repeating_and_the_period_is_honoured() {
    let w = world().await;
    let app = app(&w);
    let token = sign_in(&app, "ada@atelier").await;
    for n in 1..=5u8 {
        answered(
            &w,
            &token,
            n,
            &w.loc_a1,
            None,
            json!({ "rating": n % 5 + 1 }),
        )
        .await;
    }
    // Two answers share one timestamp, which a cursor on the time alone would split.
    sqlx::query(
        "UPDATE guest_feedback SET created_at = '2026-01-01T00:00:00.000Z' WHERE rating IN (1, 2)",
    )
    .execute(&w.state.pool)
    .await
    .unwrap();

    let mut seen = Vec::new();
    let mut before: Option<String> = None;
    loop {
        let query = match &before {
            Some(b) => format!("?days=365&limit=2&before={}", b.replace('~', "%7E")),
            None => "?days=365&limit=2".to_owned(),
        };
        let page = get(&app, &path(&w.loc_a1, &query), Some(&token)).await;
        assert_eq!(page.status, StatusCode::OK, "{}", page.error());
        let page = page.json();
        for item in page["items"].as_array().unwrap() {
            seen.push(item["id"].as_str().unwrap().to_owned());
        }
        match page["nextBefore"].as_str() {
            Some(b) => before = Some(b.to_owned()),
            None => break,
        }
    }
    assert_eq!(seen.len(), 5);
    let unique: std::collections::BTreeSet<_> = seen.iter().collect();
    assert_eq!(unique.len(), 5, "no answer is repeated");

    // The default 30 days leave out the two answers moved to January.
    let recent = get(&app, &path(&w.loc_a1, ""), Some(&token)).await.json();
    assert_eq!(recent["summary"]["count"], 3);
}

#[tokio::test]
async fn bad_cursors_and_unknown_parameters_are_refused() {
    let w = world().await;
    let app = app(&w);
    let token = sign_in(&app, "ada@atelier").await;
    for q in ["?before=nonsense", "?before=~", "?surprise=1", "?days=abc"] {
        let r = get(&app, &path(&w.loc_a1, q), Some(&token)).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{q}: {}", r.error());
    }
    // Out-of-range numbers are clamped, not refused.
    let r = get(&app, &path(&w.loc_a1, "?days=99999&limit=0"), Some(&token)).await;
    assert_eq!(r.json()["days"], 365);
}

#[tokio::test]
async fn an_empty_period_has_no_average() {
    let w = world().await;
    let app = app(&w);
    let token = sign_in(&app, "ada@atelier").await;
    let r = get(&app, &path(&w.loc_a1, ""), Some(&token)).await.json();
    assert_eq!(r["summary"]["count"], 0);
    assert!(r["summary"]["average"].is_null());
    assert_eq!(r["items"], json!([]));
    // A second restaurant's admin sees only their own (none).
    admin::create_staff(
        &w.state.pool,
        "bistro",
        "rosa@bistro",
        "Rosa",
        "manager",
        PASSWORD,
        &[],
    )
    .await
    .unwrap();
    let rosa = sign_in(&app, "rosa@bistro").await;
    let r = get(&app, &path(&w.loc_b, ""), Some(&rosa)).await.json();
    assert_eq!(r["summary"]["count"], 0);
}
