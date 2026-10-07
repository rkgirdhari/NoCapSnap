//! Reads what a restaurant publishes about itself. Preference order:
//! schema.org JSON-LD, then microdata, then OpenGraph / `<title>` / `<h1>` /
//! `tel:` links. Images are never read (copyright); raw HTML is dropped as
//! soon as the page has been parsed.

use scraper::{ElementRef, Html, Selector};
use serde_json::Value;
use url::Url;

use crate::{DraftAddress, DraftMenuItem};

pub const MAX_NAME: usize = 120;
pub const MAX_CATEGORY: usize = 40;
pub const MAX_TEXT: usize = 280;
const FALLBACK_CATEGORY: &str = "Menu";

const BUSINESS_TYPES: &[&str] = &[
    "Restaurant",
    "FoodEstablishment",
    "CafeOrCoffeeShop",
    "BarOrPub",
    "Bakery",
    "Brewery",
    "Distillery",
    "Winery",
    "FastFoodRestaurant",
    "IceCreamShop",
    "LocalBusiness",
];

#[derive(Debug, Default)]
pub struct Extracted {
    pub name: Option<String>,
    pub telephone: Option<String>,
    pub website: Option<String>,
    pub address: Option<DraftAddress>,
    pub cuisine: Vec<String>,
    pub opening_hours: Vec<String>,
    pub menu: Vec<DraftMenuItem>,
    /// `hasMenu` URLs first, then links whose text or address mentions a menu.
    pub menu_links: Vec<Url>,
    /// Which source filled the business name, for the draft's notes.
    pub name_source: Option<&'static str>,
    /// The dishes came from the page's plain text, not structured data: more likely to need fixing.
    pub menu_from_text: bool,
}

fn sel(css: &str) -> Selector {
    Selector::parse(css).expect("static selector")
}

/// Collapses whitespace and bounds the length (by characters, not bytes).
pub fn clean(text: &str, max: usize) -> Option<String> {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed: String = collapsed.chars().take(max).collect();
    let trimmed = trimmed.trim().to_owned();
    (!trimmed.is_empty()).then_some(trimmed)
}

pub fn extract(html: &str, page: &Url) -> Extracted {
    let doc = Html::parse_document(html);
    let mut out = Extracted::default();

    // ---- JSON-LD ----
    // Invalid blocks are common in the wild; skip them rather than fail the page.
    let blocks: Vec<Value> = doc
        .select(&sel(r#"script[type="application/ld+json"]"#))
        .filter_map(|script| serde_json::from_str(&script.text().collect::<String>()).ok())
        .collect();
    let mut nodes = Vec::new();
    for value in &blocks {
        flatten_nodes(value, &mut nodes);
    }
    if let Some(business) = nodes.iter().find(|n| has_type(n, BUSINESS_TYPES)) {
        read_business(business, page, &mut out);
    }
    let mut loose_items = Vec::new();
    for node in &nodes {
        if has_type(node, &["Menu"]) {
            read_menu(node, None, &mut out.menu);
        } else if has_type(node, &["MenuSection"]) {
            read_section(node, None, &mut out.menu);
        } else if has_type(node, &["MenuItem"]) {
            loose_items.push(*node);
        }
    }
    // Stand-alone items usually repeat ones inside a menu; use them only when there is no menu.
    if out.menu.is_empty() {
        for item in loose_items {
            push_item(item, FALLBACK_CATEGORY, &mut out.menu);
        }
    }

    // ---- Microdata ----
    if out.menu.is_empty() {
        let item_sel = sel(r#"[itemtype*="schema.org/MenuItem"]"#);
        let name_sel = sel(r#"[itemprop="name"]"#);
        for item in doc.select(&item_sel) {
            let category = item
                .ancestors()
                .filter_map(ElementRef::wrap)
                .find(|a| {
                    a.value()
                        .attr("itemtype")
                        .is_some_and(|t| t.contains("schema.org/MenuSection"))
                })
                .and_then(|section| first_own_prop(section, "name"))
                .unwrap_or_else(|| FALLBACK_CATEGORY.to_owned());
            if let Some(name) = item
                .select(&name_sel)
                .next()
                .and_then(|n| clean(&text_of(n), MAX_NAME))
            {
                out.menu.push(DraftMenuItem {
                    name,
                    category: clean(&category, MAX_CATEGORY)
                        .unwrap_or_else(|| FALLBACK_CATEGORY.into()),
                    description: None,
                    price: None,
                });
            }
        }
    }

    // ---- Plain text ----
    // Many small restaurants publish no structured data. A page that looks like a menu needs 3 priced
    // dishes to be believed; any other page needs 6.
    if out.menu.is_empty() {
        let found = crate::plain::read(
            &doc,
            if looks_like_menu_page(&doc, page) {
                3
            } else {
                6
            },
        );
        if !found.is_empty() {
            out.menu = found;
            out.menu_from_text = true;
        }
    }

    // ---- Fallbacks for the business itself ----
    if out.name.is_none() {
        let meta = |prop: &str| {
            doc.select(&sel(&format!(r#"meta[property="{prop}"]"#)))
                .next()
                .and_then(|m| m.value().attr("content"))
                .and_then(|c| clean(c, MAX_NAME))
        };
        let title = || {
            doc.select(&sel("title"))
                .next()
                .and_then(|t| clean(&text_of(t), MAX_NAME))
        };
        let h1 = || {
            doc.select(&sel("h1"))
                .next()
                .and_then(|t| clean(&text_of(t), MAX_NAME))
        };
        (out.name, out.name_source) = if let Some(n) = meta("og:site_name") {
            (Some(n), Some("the page's site name"))
        } else if let Some(n) = meta("og:title") {
            (Some(n), Some("the page's title"))
        } else if let Some(n) = title() {
            (Some(n), Some("the page's title"))
        } else if let Some(n) = h1() {
            (Some(n), Some("the page's main heading"))
        } else {
            (None, None)
        };
    }
    if out.telephone.is_none() {
        out.telephone = doc
            .select(&sel(r#"a[href^="tel:"]"#))
            .next()
            .and_then(|a| a.value().attr("href"))
            .and_then(|h| clean(&h["tel:".len()..].replace("%20", " "), 40));
    }

    // ---- Links that look like a menu ----
    for a in doc.select(&sel("a[href]")) {
        let href = a.value().attr("href").unwrap_or_default();
        let label = text_of(a).to_lowercase();
        if !(label.contains("menu") || href.to_lowercase().contains("menu")) {
            continue;
        }
        if let Ok(mut url) = page.join(href) {
            url.set_fragment(None);
            if url != *page && !out.menu_links.contains(&url) {
                out.menu_links.push(url);
            }
        }
    }
    out
}

/// The address, title or main heading says "menu".
fn looks_like_menu_page(doc: &Html, page: &Url) -> bool {
    let named = |css: &str| {
        doc.select(&sel(css))
            .next()
            .is_some_and(|e| text_of(e).to_lowercase().contains("menu"))
    };
    page.path().to_lowercase().contains("menu") || named("title") || named("h1")
}

fn text_of(el: ElementRef<'_>) -> String {
    el.text().collect::<Vec<_>>().join(" ")
}

/// The first `itemprop` of `section` that isn't inside a nested item.
fn first_own_prop(section: ElementRef<'_>, prop: &str) -> Option<String> {
    let want = sel(&format!(r#"[itemprop="{prop}"]"#));
    section
        .select(&want)
        .find(|el| {
            el.ancestors()
                .filter_map(ElementRef::wrap)
                .take_while(|a| a.id() != section.id())
                .all(|a| a.value().attr("itemscope").is_none())
        })
        .map(text_of)
}

fn flatten_nodes<'a>(value: &'a Value, out: &mut Vec<&'a Value>) {
    match value {
        Value::Array(items) => items.iter().for_each(|v| flatten_nodes(v, out)),
        Value::Object(map) => {
            out.push(value);
            if let Some(graph) = map.get("@graph") {
                flatten_nodes(graph, out);
            }
        }
        _ => {}
    }
}

fn has_type(node: &Value, types: &[&str]) -> bool {
    let short = |t: &str| t.rsplit(['/', '#', ':']).next().unwrap_or(t).to_owned();
    match node.get("@type") {
        Some(Value::String(t)) => types.contains(&short(t).as_str()),
        Some(Value::Array(ts)) => ts
            .iter()
            .filter_map(Value::as_str)
            .any(|t| types.contains(&short(t).as_str())),
        _ => false,
    }
}

fn string(node: &Value, key: &str) -> Option<String> {
    match node.get(key)? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Object(o) => o.get("name").and_then(Value::as_str).map(str::to_owned),
        Value::Array(a) => a.first().and_then(Value::as_str).map(str::to_owned),
        _ => None,
    }
}

fn strings(node: &Value, key: &str) -> Vec<String> {
    match node.get(key) {
        Some(Value::String(s)) => s
            .split(',')
            .filter_map(|p| clean(p, MAX_CATEGORY))
            .collect(),
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(Value::as_str)
            .filter_map(|s| clean(s, MAX_CATEGORY))
            .collect(),
        _ => Vec::new(),
    }
}

fn read_business(node: &Value, page: &Url, out: &mut Extracted) {
    if let Some(name) = string(node, "name").and_then(|n| clean(&n, MAX_NAME)) {
        out.name = Some(name);
        out.name_source = Some("the site's structured data");
    }
    out.telephone = string(node, "telephone").and_then(|t| clean(&t, 40));
    out.website = string(node, "url")
        .and_then(|u| page.join(&u).ok())
        .map(|u| u.to_string());
    out.cuisine = strings(node, "servesCuisine");
    out.address = match node.get("address") {
        Some(Value::String(s)) => clean(s, MAX_TEXT).map(|line| DraftAddress {
            street: Some(line),
            ..Default::default()
        }),
        Some(addr @ Value::Object(_)) => Some(DraftAddress {
            street: string(addr, "streetAddress").and_then(|s| clean(&s, MAX_TEXT)),
            locality: string(addr, "addressLocality").and_then(|s| clean(&s, MAX_NAME)),
            region: string(addr, "addressRegion").and_then(|s| clean(&s, MAX_NAME)),
            postal_code: string(addr, "postalCode").and_then(|s| clean(&s, 20)),
            country: string(addr, "addressCountry").and_then(|s| clean(&s, MAX_NAME)),
        }),
        _ => None,
    };
    out.opening_hours = match node.get("openingHours") {
        Some(Value::String(s)) => clean(s, MAX_TEXT).into_iter().collect(),
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(Value::as_str)
            .filter_map(|s| clean(s, MAX_TEXT))
            .collect(),
        _ => Vec::new(),
    };
    if out.opening_hours.is_empty() {
        let specs = match node.get("openingHoursSpecification") {
            Some(Value::Array(a)) => a.iter().collect(),
            Some(o @ Value::Object(_)) => vec![o],
            _ => Vec::new(),
        };
        for spec in specs {
            let days = match spec.get("dayOfWeek") {
                Some(Value::String(d)) => vec![short_day(d)],
                Some(Value::Array(ds)) => {
                    ds.iter().filter_map(Value::as_str).map(short_day).collect()
                }
                _ => Vec::new(),
            };
            if let (Some(opens), Some(closes)) = (string(spec, "opens"), string(spec, "closes")) {
                out.opening_hours.push(format!(
                    "{} {}–{}",
                    days.join(","),
                    trim_secs(&opens),
                    trim_secs(&closes)
                ));
            }
        }
    }
    match node.get("hasMenu") {
        Some(Value::String(u)) => push_link(page, u, &mut out.menu_links),
        Some(Value::Array(a)) => {
            for v in a {
                match v {
                    Value::String(u) => push_link(page, u, &mut out.menu_links),
                    m @ Value::Object(_) => read_menu_or_link(m, page, out),
                    _ => {}
                }
            }
        }
        Some(m @ Value::Object(_)) => read_menu_or_link(m, page, out),
        _ => {}
    }
}

fn read_menu_or_link(menu: &Value, page: &Url, out: &mut Extracted) {
    if menu.get("hasMenuSection").is_some() || menu.get("hasMenuItem").is_some() {
        read_menu(menu, None, &mut out.menu);
    } else if let Some(u) = string(menu, "url").or_else(|| string(menu, "@id")) {
        push_link(page, &u, &mut out.menu_links);
    }
}

fn push_link(page: &Url, href: &str, links: &mut Vec<Url>) {
    if let Ok(mut url) = page.join(href) {
        url.set_fragment(None);
        if url != *page && !links.contains(&url) {
            links.insert(0, url); // declared menus go before guessed ones
        }
    }
}

fn read_menu(menu: &Value, category: Option<&str>, items: &mut Vec<DraftMenuItem>) {
    for section in each(menu.get("hasMenuSection")) {
        read_section(section, category, items);
    }
    for item in each(menu.get("hasMenuItem")) {
        push_item(item, category.unwrap_or(FALLBACK_CATEGORY), items);
    }
}

fn read_section(section: &Value, parent: Option<&str>, items: &mut Vec<DraftMenuItem>) {
    let name = string(section, "name").and_then(|n| clean(&n, MAX_CATEGORY));
    let category = name.as_deref().or(parent);
    read_menu(section, category, items);
}

fn push_item(item: &Value, category: &str, items: &mut Vec<DraftMenuItem>) {
    let Some(name) = string(item, "name").and_then(|n| clean(&n, MAX_NAME)) else {
        return;
    };
    let offer = match item.get("offers") {
        Some(Value::Array(a)) => a.first(),
        other => other,
    };
    let price = offer.and_then(|o| {
        let amount = string(o, "price")?;
        Some(match string(o, "priceCurrency") {
            Some(cur) => format!("{amount} {cur}"),
            None => amount,
        })
    });
    let category = clean(category, MAX_CATEGORY).unwrap_or_else(|| FALLBACK_CATEGORY.into());
    if !items
        .iter()
        .any(|i| i.name == name && i.category == category)
    {
        items.push(DraftMenuItem {
            name,
            category,
            description: string(item, "description").and_then(|d| clean(&d, MAX_TEXT)),
            price,
        });
    }
}

fn each(value: Option<&Value>) -> Vec<&Value> {
    match value {
        Some(Value::Array(a)) => a.iter().collect(),
        Some(v @ Value::Object(_)) => vec![v],
        _ => Vec::new(),
    }
}

fn short_day(day: &str) -> String {
    let d = day.rsplit('/').next().unwrap_or(day);
    d.chars().take(2).collect()
}

fn trim_secs(t: &str) -> String {
    match t.len() {
        8 if t.as_bytes()[5] == b':' => t[..5].to_owned(),
        _ => t.to_owned(),
    }
}
