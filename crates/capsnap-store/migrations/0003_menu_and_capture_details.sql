-- W2: local menu cache. Same shape the server's GET /locations/{id}/menu-items
-- will fill in W3 (Spec §5); until then rows come from a labelled demo seed.
CREATE TABLE menu_items (
    id          TEXT    PRIMARY KEY,
    location_id TEXT    NOT NULL,
    name        TEXT    NOT NULL CHECK (length(name) BETWEEN 1 AND 120),
    category    TEXT    NOT NULL CHECK (length(category) BETWEEN 1 AND 40),
    sort_order  INTEGER NOT NULL DEFAULT 0,
    is_active   INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    -- Where the row came from, so demo data is never mistaken for a real menu.
    source      TEXT    NOT NULL CHECK (source IN ('demo', 'server', 'import'))
) STRICT;

CREATE INDEX menu_items_by_location ON menu_items (location_id, is_active, sort_order);

-- Device-only settings (staff display name, current location). Never synced.
CREATE TABLE settings (
    key   TEXT PRIMARY KEY CHECK (length(key) BETWEEN 1 AND 64),
    value TEXT NOT NULL CHECK (length(value) <= 256)
) STRICT;

-- What was photographed. The dish name is copied at capture time so History
-- still reads correctly after the menu cache is refreshed.
ALTER TABLE captures ADD COLUMN location_id TEXT;
ALTER TABLE captures ADD COLUMN menu_item_id TEXT;
ALTER TABLE captures ADD COLUMN dish_name TEXT CHECK (dish_name IS NULL OR length(dish_name) BETWEEN 1 AND 120);
-- Owner default M6: a staff reference kept on this device only; never synced
-- and never shown to guests.
ALTER TABLE captures ADD COLUMN table_label TEXT CHECK (table_label IS NULL OR length(table_label) BETWEEN 1 AND 16);
-- Pixel size after on-device processing (Spec §3: at most 2048 on the long edge).
ALTER TABLE captures ADD COLUMN media_width INTEGER;
ALTER TABLE captures ADD COLUMN media_height INTEGER;
