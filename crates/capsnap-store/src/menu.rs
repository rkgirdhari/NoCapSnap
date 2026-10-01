//! The menu cache and device-only settings (W2).
//!
//! Menu rows have the shape the server's `GET /locations/{id}/menu-items`
//! returns (Spec §5), so W3 only swaps where they come from. Until then the
//! only source is a demo seed, and every demo row says so in `source`.

use crate::LocalStore;

/// Demo location used until staff sign in against a server (W3).
pub const DEMO_LOCATION_ID: &str = "demo-atelier-no-8";
pub const DEMO_LOCATION_NAME: &str = "Atelier No. 8";

/// (id, name, category) — the first three match mockup 02 "Which dish?".
const DEMO_MENU: &[(&str, &str, &str)] = &[
    ("demo-saffron-butter-cod", "Saffron butter cod", "Mains"),
    (
        "demo-wild-mushroom-risotto",
        "Wild mushroom risotto",
        "Mains",
    ),
    (
        "demo-charred-heritage-carrots",
        "Charred heritage carrots",
        "Mains",
    ),
    ("demo-five-spice-duck", "Five-spice duck breast", "Mains"),
    ("demo-pork-belly-bao", "Pork belly bao", "Starters"),
    (
        "demo-sweet-corn-miso",
        "Sweet corn & white miso soup",
        "Starters",
    ),
    ("demo-tuna-crudo", "Tuna crudo, yuzu kosho", "Starters"),
];

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MenuItem {
    pub id: String,
    pub location_id: String,
    pub name: String,
    pub category: String,
    pub sort_order: i64,
    pub is_active: bool,
    pub source: String,
}

pub struct NewMenuItem<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub category: &'a str,
    pub is_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuSource {
    Demo,
    Server,
    Import,
}

impl MenuSource {
    fn as_str(self) -> &'static str {
        match self {
            MenuSource::Demo => "demo",
            MenuSource::Server => "server",
            MenuSource::Import => "import",
        }
    }
}

/// The only keys the app may write. A closed set keeps the table from turning
/// into a place where guest data could be parked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    StaffDisplayName,
    LocationId,
    LocationName,
    // W3a: the signed-in server session.
    ServerUrl,
    SessionToken,
    SessionExpiresAt,
    StaffId,
    StaffRole,
    OrganizationName,
}

impl Setting {
    pub fn key(self) -> &'static str {
        match self {
            Setting::StaffDisplayName => "staff_display_name",
            Setting::LocationId => "location_id",
            Setting::LocationName => "location_name",
            Setting::ServerUrl => "server_url",
            Setting::SessionToken => "session_token",
            Setting::SessionExpiresAt => "session_expires_at",
            Setting::StaffId => "staff_id",
            Setting::StaffRole => "staff_role",
            Setting::OrganizationName => "organization_name",
        }
    }
}

impl LocalStore {
    /// Active dishes for one location, in menu order.
    pub async fn menu_items(&self, location_id: &str) -> Result<Vec<MenuItem>, sqlx::Error> {
        sqlx::query_as(
            "SELECT id, location_id, name, category, sort_order, is_active, source
             FROM menu_items
             WHERE location_id = ? AND is_active = 1
             ORDER BY sort_order, name",
        )
        .bind(location_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn menu_item(&self, id: &str) -> Result<Option<MenuItem>, sqlx::Error> {
        sqlx::query_as(
            "SELECT id, location_id, name, category, sort_order, is_active, source
             FROM menu_items WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Replaces one location's cached menu in a single transaction, keeping the
    /// given order. Past captures keep their copied dish name.
    pub async fn replace_menu(
        &self,
        location_id: &str,
        source: MenuSource,
        items: &[NewMenuItem<'_>],
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM menu_items WHERE location_id = ?")
            .bind(location_id)
            .execute(&mut *tx)
            .await?;
        for (order, item) in items.iter().enumerate() {
            sqlx::query(
                "INSERT INTO menu_items (id, location_id, name, category, sort_order, is_active, source)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(item.id)
            .bind(location_id)
            .bind(item.name)
            .bind(item.category)
            .bind(order as i64)
            .bind(item.is_active)
            .bind(source.as_str())
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await
    }

    /// First run only: seeds the demo location and its labelled demo menu so the
    /// capture flow works offline before W3. Returns true if it seeded.
    pub async fn seed_demo_if_empty(&self) -> Result<bool, sqlx::Error> {
        if self.setting(Setting::LocationId).await?.is_some() {
            return Ok(false);
        }
        let items: Vec<NewMenuItem<'_>> = DEMO_MENU
            .iter()
            .map(|&(id, name, category)| NewMenuItem {
                id,
                name,
                category,
                is_active: true,
            })
            .collect();
        self.replace_menu(DEMO_LOCATION_ID, MenuSource::Demo, &items)
            .await?;
        self.set_setting(Setting::LocationName, DEMO_LOCATION_NAME)
            .await?;
        // Written last: its presence is what marks the seed as done.
        self.set_setting(Setting::LocationId, DEMO_LOCATION_ID)
            .await?;
        Ok(true)
    }

    pub async fn setting(&self, key: Setting) -> Result<Option<String>, sqlx::Error> {
        let row: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
            .bind(key.key())
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.0))
    }

    pub async fn set_setting(&self, key: Setting, value: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?, ?)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        )
        .bind(key.key())
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn clear_setting(&self, key: Setting) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM settings WHERE key = ?")
            .bind(key.key())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
