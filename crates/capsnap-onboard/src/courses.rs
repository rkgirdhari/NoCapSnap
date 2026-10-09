//! Puts a menu's courses in the order a diner reads them: breakfast, starters, soups and salads, mains,
//! sides, desserts, then drinks. Within a course, and among courses we don't recognise, the restaurant's
//! own order is kept (the sort is stable). Names are matched by keyword, in English.

use crate::DraftMenuItem;

/// Where an unrecognised course sits: with the mains, so "Pizzas" or "From the grill" stay in the middle.
const OTHER: u8 = 35;

const COURSES: &[(u8, &[&str])] = &[
    (5, &["breakfast", "brunch"]),
    (
        10,
        &[
            "starter",
            "appetizer",
            "appetiser",
            "small plate",
            "to start",
            "to share",
            "snack",
            "tapas",
            "bites",
            "antipasti",
            "raw bar",
            "sharing",
        ],
    ),
    (20, &["soup", "salad"]),
    (
        30,
        &[
            "main",
            "entree",
            "entrée",
            "dinner",
            "pasta",
            "pizza",
            "burger",
            "sandwich",
            "grill",
            "large plate",
        ],
    ),
    (40, &["side"]),
    (50, &["dessert", "sweet", "pastry", "pastries"]),
    (55, &["kid", "children"]),
    (
        60,
        &[
            "drink", "beverage", "cocktail", "mocktail", "beer", "wine", "coffee", "tea", "soda",
            "juice", "spirit", "whiskey", "liquor",
        ],
    ),
];

/// Whole words only ("steaks" must not match "tea"), singular or plural. Drinks are checked first, so
/// "Dessert Wines" sits with the drinks, then the other courses from the narrow to the broad.
fn rank(category: &str) -> u8 {
    let spaced: String = category
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    let padded = format!(
        " {} ",
        spaced.split_whitespace().collect::<Vec<_>>().join(" ")
    );
    let has = |words: &[&str]| {
        words.iter().any(|w| {
            [format!(" {w} "), format!(" {w}s "), format!(" {w}es ")]
                .iter()
                .any(|form| padded.contains(form))
        })
    };
    for want in [60, 50, 55, 40, 20, 10, 5, 30] {
        if let Some((rank, words)) = COURSES.iter().find(|(r, _)| *r == want)
            && has(words)
        {
            return *rank;
        }
    }
    OTHER
}

pub fn sort(menu: &mut [DraftMenuItem]) {
    menu.sort_by_key(|item| rank(&item.category));
}
