//! A fallback for menus published as plain text, with no structured data: course headings, then one
//! line per dish ending in a price ("Short rib ........ $32", "Burrata  14"), sometimes with a
//! description line below. It is deliberately cautious. A page is trusted only when it yields several
//! priced dishes, because a wrong guess on a non-menu page is worse than finding nothing: the owner
//! reviews every dish either way, and the draft says these were read from text.

use scraper::{ElementRef, Html, Selector};

use crate::DraftMenuItem;
use crate::extract::{MAX_CATEGORY, MAX_NAME, MAX_TEXT, clean};

const MAX_ITEMS: usize = 300;
const FALLBACK_CATEGORY: &str = "Menu";

/// Elements that start a new line of text in a menu.
const BLOCK: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "dd",
    "details",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "hr",
    "li",
    "main",
    "ol",
    "p",
    "pre",
    "section",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "tr",
    "ul",
];
/// Page furniture and things that never hold dishes.
const SKIP: &[&str] = &[
    "script", "style", "noscript", "template", "svg", "head", "nav", "header", "footer", "form",
    "select", "button", "iframe",
];
/// A "dish" with one of these words is a charge or a notice, not food.
const NOT_FOOD: &[&str] = &[
    "tax",
    "gratuity",
    "tip ",
    "service charge",
    "delivery",
    "fee",
    "gift",
    "minimum",
    "reservation",
    "parking",
    "per person",
    "per guest",
    "happy hour",
    "call ",
    "phone",
    "hours",
    "copyright",
    "©",
    "party of",
    "parties of",
    "corkage",
    "split plate",
    "substitut",
    "add ",
    "extra ",
];
/// Page-title headings that name the page, not a course.
const PAGE_TITLES: &[&str] = &[
    "menu",
    "our menu",
    "the menu",
    "food menu",
    "full menu",
    "menus",
    "food",
];

enum Line {
    Heading(String),
    Text(String),
}

fn is_block(tag: &str) -> bool {
    BLOCK.contains(&tag)
}

fn has_block_descendant(el: ElementRef<'_>) -> bool {
    el.descendants()
        .skip(1)
        .filter_map(ElementRef::wrap)
        .any(|d| is_block(d.value().name()))
}

fn visit(el: ElementRef<'_>, lines: &mut Vec<Line>) {
    let tag = el.value().name();
    if SKIP.contains(&tag) {
        return;
    }
    // A table row is one line, whatever its cells hold ("Short rib" | "$32").
    if is_block(tag) && (tag == "tr" || !has_block_descendant(el)) {
        let text = el.text().collect::<Vec<_>>().join(" ");
        if let Some(text) = clean(&text, 400) {
            let heading = matches!(tag, "h1" | "h2" | "h3" | "h4" | "h5" | "h6");
            lines.push(if heading {
                Line::Heading(text)
            } else {
                Line::Text(text)
            });
        }
        return;
    }
    for child in el.children().filter_map(ElementRef::wrap) {
        visit(child, lines);
    }
}

/// A price token at the end of a line: `$32`, `32.00`, `€14,50`, `12+`. `strong` when a currency sign or
/// decimals make it unmistakable; a bare whole number needs dot leaders to count.
struct Price {
    text: String,
    strong: bool,
}

fn parse_price(token: &str) -> Option<Price> {
    let t = token.trim_end_matches(['*', '+']);
    let (symbol, digits) = match t.chars().next() {
        Some(c @ ('$' | '€' | '£')) => (Some(c), &t[c.len_utf8()..]),
        _ => (None, t),
    };
    let (whole, cents) = match digits.split_once(['.', ',']) {
        Some((w, c)) => (w, Some(c)),
        None => (digits, None),
    };
    let whole_ok = (1..=3).contains(&whole.len()) && whole.chars().all(|c| c.is_ascii_digit());
    let cents_ok = cents.is_none_or(|c| c.len() == 2 && c.chars().all(|d| d.is_ascii_digit()));
    if !whole_ok || !cents_ok || whole.parse::<u32>().ok()? == 0 && cents.is_none() {
        return None;
    }
    Some(Price {
        text: token.trim_end_matches(['*', '+']).to_owned(),
        strong: symbol.is_some() || cents.is_some(),
    })
}

const LEADERS: &[char] = &['.', '·', '…', '-', '–', '—', ':', '_', ' ', '\u{2022}'];

/// "Short rib ........ $32" → ("Short rib", "$32"). `None` when the line doesn't end in a price.
fn split_line(text: &str) -> Option<(String, Price, bool)> {
    let text = text.trim();
    let cut = text.rfind(char::is_whitespace).map(|i| i + 1).unwrap_or(0);
    let price = parse_price(&text[cut..])?;
    let name = text[..cut].trim_end_matches(LEADERS);
    // A run of dots between the name and the price, as printed menus have: "Short rib ........ 32".
    let gap = &text[..cut];
    let leaders =
        gap.contains("...") || gap.contains('…') || gap.contains("···") || gap.contains(". . .");
    if !price.strong && !leaders {
        return None;
    }
    // "Small $9 / Large $14": keep the dish, drop the extra prices.
    let name = name
        .split(['$', '€', '£'])
        .next()
        .unwrap_or("")
        .trim_end_matches(['/', '-', '–', '—', '(', ' ']);
    let name = name.trim_start_matches(['•', '-', '–', '—', '*', ' ']);
    if name.chars().filter(|c| c.is_alphabetic()).count() < 2 || name.chars().count() > 120 {
        return None;
    }
    Some((name.to_owned(), price, leaders))
}

fn is_price_only(text: &str) -> bool {
    parse_price(text.trim()).is_some_and(|p| p.strong)
}

fn looks_like_not_food(name: &str) -> bool {
    let n = format!("{} ", name.to_lowercase());
    NOT_FOOD.iter().any(|w| n.contains(w))
}

/// "BRAISED SHORT RIB 8 HRS" style headings: short, capitals only, no price.
fn shouting_heading(text: &str) -> bool {
    let letters: Vec<char> = text.chars().filter(|c| c.is_alphabetic()).collect();
    letters.len() >= 3
        && text.chars().count() <= 30
        && letters.iter().all(|c| c.is_uppercase())
        && split_line(text).is_none()
}

/// Splits "Short rib — braised for eight hours with…" into a name and a description when the line is long.
fn name_and_description(name: &str) -> (String, Option<String>) {
    if name.chars().count() > 45 {
        for sep in [" – ", " — ", " - ", ". ", ", "] {
            if let Some((head, tail)) = name.split_once(sep)
                && (3..=45).contains(&head.chars().count())
            {
                return (head.trim().to_owned(), clean(tail, MAX_TEXT));
            }
        }
    }
    (name.to_owned(), None)
}

/// Reads the dishes from the page's text. Returns nothing unless at least `min_items` priced dishes are
/// found (3 on a page that looks like a menu, more elsewhere).
pub fn read(doc: &Html, min_items: usize) -> Vec<DraftMenuItem> {
    let body = Selector::parse("body").expect("static selector");
    let root = doc
        .select(&body)
        .next()
        .unwrap_or_else(|| doc.root_element());
    let mut lines = Vec::new();
    visit(root, &mut lines);

    let mut items: Vec<DraftMenuItem> = Vec::new();
    let mut category = FALLBACK_CATEGORY.to_owned();
    let mut pending: Option<String> = None;
    for line in &lines {
        let (text, heading) = match line {
            Line::Heading(t) => (t.as_str(), true),
            Line::Text(t) => (t.as_str(), false),
        };
        if let Some((name, price, _)) = split_line(text) {
            if looks_like_not_food(&name) {
                pending = None;
                continue;
            }
            let (name, description) = name_and_description(&name);
            push(&mut items, &name, &category, description, Some(price.text));
            pending = None;
            continue;
        }
        if is_price_only(text) {
            // The name sat in the line above: "Short rib" then "$32".
            if let Some(name) = pending.take()
                && !looks_like_not_food(&name)
            {
                push(
                    &mut items,
                    &name,
                    &category,
                    None,
                    Some(text.trim().to_owned()),
                );
            }
            continue;
        }
        if heading || shouting_heading(text) {
            let key = text.to_lowercase();
            category = if PAGE_TITLES.contains(&key.trim()) {
                FALLBACK_CATEGORY.to_owned()
            } else {
                clean(text.trim_end_matches([':', ' ']), MAX_CATEGORY)
                    .unwrap_or_else(|| FALLBACK_CATEGORY.to_owned())
            };
            pending = None;
            continue;
        }
        // Plain words: a description of the dish just read, or the name of the next dish.
        match items.last_mut() {
            Some(last)
                if last.description.is_none()
                    && text.chars().count() >= 20
                    && pending.is_none() =>
            {
                last.description = clean(text, MAX_TEXT);
            }
            _ if text.chars().count() <= 80 => pending = clean(text, MAX_NAME),
            _ => pending = None,
        }
    }
    if items.len() < min_items {
        return Vec::new();
    }
    items.truncate(MAX_ITEMS);
    items
}

fn push(
    items: &mut Vec<DraftMenuItem>,
    name: &str,
    category: &str,
    description: Option<String>,
    price: Option<String>,
) {
    let Some(name) = clean(name, MAX_NAME) else {
        return;
    };
    let category = clean(category, MAX_CATEGORY).unwrap_or_else(|| FALLBACK_CATEGORY.to_owned());
    if items
        .iter()
        .any(|i| i.name.eq_ignore_ascii_case(&name) && i.category == category)
    {
        return;
    }
    items.push(DraftMenuItem {
        name,
        category,
        description,
        price,
    });
}
