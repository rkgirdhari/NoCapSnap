//! `PUT /locations/{id}/menu-items`: saving a reviewed menu (website import or by hand).

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::{Value, json};

async fn put(app: &axum::Router, token: Option<&str>, location: &str, body: Value) -> Resp {
    json(
        app,
        Method::PUT,
        &format!("/api/v1/locations/{location}/menu-items"),
        token,
        body,
    )
    .await
}

fn names(r: &Resp) -> Vec<String> {
    r.json()
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            format!(
                "{}/{}",
                i["category"].as_str().unwrap(),
                i["name"].as_str().unwrap()
            )
        })
        .collect()
}

#[tokio::test]
async fn only_managers_may_save_a_menu_and_only_for_their_own_restaurant() {
    let w = world().await;
    let app = app(&w);
    let admin = sign_in(&app, "ada@atelier").await;
    let chef = sign_in(&app, "chen@atelier").await;
    let one = json!({ "items": [{ "name": "Burrata", "category": "Starters" }] });

    assert_eq!(
        put(&app, None, &w.loc_a1, one.clone()).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        put(&app, Some(&chef), &w.loc_a1, one.clone()).await.status,
        StatusCode::FORBIDDEN
    );
    // Another restaurant's location looks like it doesn't exist.
    assert_eq!(
        put(&app, Some(&admin), &w.loc_b, one.clone()).await.status,
        StatusCode::NOT_FOUND
    );
    let (rosa,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM menu_items WHERE location_id = ? AND is_active = 1")
            .bind(&w.loc_b)
            .fetch_one(&w.state.pool)
            .await
            .unwrap();
    assert_eq!(rosa, 1, "the other restaurant's menu is untouched");

    let ok = put(&app, Some(&admin), &w.loc_a1, one).await;
    assert_eq!(ok.status, StatusCode::OK);
    assert_eq!(names(&ok), ["Starters/Burrata"]);
}

#[tokio::test]
async fn the_menu_is_checked_tidied_and_kept_in_the_order_sent() {
    let w = world().await;
    let app = app(&w);
    let admin = sign_in(&app, "ada@atelier").await;

    // An empty list would wipe the menu: refused, and the old menu stays.
    let before = get(
        &app,
        &format!("/api/v1/locations/{}/menu-items", w.loc_a1),
        Some(&admin),
    )
    .await;
    let empty = put(&app, Some(&admin), &w.loc_a1, json!({ "items": [] })).await;
    assert_eq!(
        (empty.status, empty.error().as_str()),
        (StatusCode::BAD_REQUEST, "empty_menu")
    );
    let still = get(
        &app,
        &format!("/api/v1/locations/{}/menu-items", w.loc_a1),
        Some(&admin),
    )
    .await;
    assert_eq!(names(&still), names(&before));

    let blank = put(
        &app,
        Some(&admin),
        &w.loc_a1,
        json!({ "items": [{ "name": "  ", "category": "Mains" }] }),
    )
    .await;
    assert_eq!(
        (blank.status, blank.error().as_str()),
        (StatusCode::BAD_REQUEST, "bad_item")
    );
    let many: Vec<Value> = (0..501)
        .map(|i| json!({ "name": format!("Dish {i}"), "category": "Mains" }))
        .collect();
    let big = put(&app, Some(&admin), &w.loc_a1, json!({ "items": many })).await;
    assert_eq!(big.error(), "too_many_items");
    let extra = put(
        &app,
        Some(&admin),
        &w.loc_a1,
        json!({ "items": [], "price": 1 }),
    )
    .await;
    assert_eq!(
        extra.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "unknown fields are refused"
    );

    let r = put(
        &app,
        Some(&admin),
        &w.loc_a1,
        json!({ "items": [
            { "name": "  Short   rib\u{0007} ", "category": "Mains" },
            { "name": "Burrata", "category": "Starters" },
            { "name": "short rib", "category": "mains" },
            { "name": "Short rib", "category": "Sharing" },
        ] }),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    // Whitespace and control characters cleaned; the repeat under the same category dropped;
    // the same dish under another category kept; the order is the order sent.
    assert_eq!(
        names(&r),
        ["Mains/Short rib", "Starters/Burrata", "Sharing/Short rib"]
    );
}

#[tokio::test]
async fn replacing_the_menu_keeps_what_earlier_captures_recorded() {
    let w = world().await;
    let app = app(&w);
    let admin = sign_in(&app, "ada@atelier").await;
    let media = upload_ok(&app, &admin, phone_jpeg(320, 240, 5)).await;
    let cod = menu_item(&app, &admin, &w.loc_a1, "Saffron butter cod").await;
    let first = capture(
        &app,
        &admin,
        "menu-replace-0001",
        &media,
        &w.loc_a1,
        Some(&cod),
    )
    .await;
    assert_eq!(first.status, StatusCode::CREATED);

    let r = put(
        &app,
        Some(&admin),
        &w.loc_a1,
        json!({ "items": [{ "name": "Burrata", "category": "Starters" }] }),
    )
    .await;
    assert_eq!(names(&r), ["Starters/Burrata"]);

    // A phone that was offline sends the same capture again, naming the retired dish.
    let retry = capture(
        &app,
        &admin,
        "menu-replace-0001",
        &media,
        &w.loc_a1,
        Some(&cod),
    )
    .await;
    assert_eq!(retry.status, StatusCode::OK);
    let (dish,): (String,) =
        sqlx::query_as("SELECT dish_name FROM captures WHERE client_id = 'menu-replace-0001'")
            .fetch_one(&w.state.pool)
            .await
            .unwrap();
    assert_eq!(dish, "Saffron butter cod");
}
