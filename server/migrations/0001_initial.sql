-- CapSnap server schema (Spec §5). Every tenant-owned row carries org_id, and
-- every query filters on the org taken from the authenticated session.
-- Timestamps are RFC 3339 UTC strings with milliseconds, so text order is time order.

CREATE TABLE organizations (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 120),
    slug       TEXT NOT NULL UNIQUE CHECK (slug GLOB '[a-z0-9]*' AND slug NOT GLOB '*[^a-z0-9-]*' AND length(slug) BETWEEN 2 AND 40),
    status     TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'suspended')),
    created_at TEXT NOT NULL
) STRICT;

CREATE TABLE locations (
    id         TEXT PRIMARY KEY,
    org_id     TEXT NOT NULL REFERENCES organizations (id),
    name       TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 120),
    timezone   TEXT NOT NULL CHECK (length(timezone) BETWEEN 1 AND 64),
    created_at TEXT NOT NULL
) STRICT;
CREATE INDEX locations_by_org ON locations (org_id);

CREATE TABLE staff (
    id            TEXT PRIMARY KEY,
    org_id        TEXT NOT NULL REFERENCES organizations (id),
    -- Sign-in name, unique across the server (compared lower-case).
    login         TEXT NOT NULL UNIQUE CHECK (login = lower(login) AND length(login) BETWEEN 3 AND 64),
    display_name  TEXT NOT NULL CHECK (length(display_name) BETWEEN 1 AND 60),
    -- Argon2id PHC string; the password itself is never stored.
    password_hash TEXT NOT NULL CHECK (password_hash LIKE '$argon2id$%'),
    role          TEXT NOT NULL CHECK (role IN ('admin', 'manager', 'chef', 'server')),
    active        INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    created_at    TEXT NOT NULL
) STRICT;

-- Chefs and servers work at the locations listed here; admins and managers at all of their organization's.
CREATE TABLE staff_locations (
    staff_id    TEXT NOT NULL REFERENCES staff (id),
    location_id TEXT NOT NULL REFERENCES locations (id),
    PRIMARY KEY (staff_id, location_id)
) STRICT;

-- One per signed-in phone. Only a SHA-256 of the bearer token is stored.
CREATE TABLE device_sessions (
    id           TEXT PRIMARY KEY,
    staff_id     TEXT NOT NULL REFERENCES staff (id),
    token_hash   TEXT NOT NULL UNIQUE CHECK (length(token_hash) = 64),
    device_label TEXT NOT NULL CHECK (length(device_label) BETWEEN 1 AND 60),
    created_at   TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    expires_at   TEXT NOT NULL,
    revoked_at   TEXT
) STRICT;
CREATE INDEX device_sessions_by_staff ON device_sessions (staff_id);

CREATE TABLE menu_items (
    id          TEXT PRIMARY KEY,
    org_id      TEXT NOT NULL REFERENCES organizations (id),
    location_id TEXT NOT NULL REFERENCES locations (id),
    name        TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 120),
    category    TEXT NOT NULL CHECK (length(category) BETWEEN 1 AND 40),
    sort_order  INTEGER NOT NULL DEFAULT 0,
    is_active   INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1))
) STRICT;
CREATE INDEX menu_items_by_location ON menu_items (location_id, is_active, sort_order);

-- Spec §5 "Media Assets": opaque id, digest, MIME, private storage key.
CREATE TABLE media_assets (
    id          TEXT PRIMARY KEY,
    org_id      TEXT NOT NULL REFERENCES organizations (id),
    sha256      TEXT NOT NULL CHECK (length(sha256) = 64),
    mime        TEXT NOT NULL CHECK (mime = 'image/jpeg'),
    bytes       INTEGER NOT NULL,
    width       INTEGER NOT NULL,
    height      INTEGER NOT NULL,
    storage_key TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    -- Set by the retention job when the file is deleted (Spec §6: 30 days after sync).
    deleted_at  TEXT,
    UNIQUE (org_id, sha256)
) STRICT;

CREATE TABLE captures (
    id               TEXT PRIMARY KEY,
    org_id           TEXT NOT NULL REFERENCES organizations (id),
    location_id      TEXT NOT NULL REFERENCES locations (id),
    staff_id         TEXT NOT NULL REFERENCES staff (id),
    -- The phone's id for this capture: the idempotency key (Spec §5).
    client_id        TEXT NOT NULL CHECK (length(client_id) BETWEEN 8 AND 64),
    media_id         TEXT NOT NULL REFERENCES media_assets (id),
    menu_item_id     TEXT REFERENCES menu_items (id),
    dish_name        TEXT,
    capture_time_utc TEXT NOT NULL,
    received_at      TEXT NOT NULL,
    UNIQUE (org_id, client_id)
) STRICT;

-- Spec §4: 256-bit capability tokens; only their SHA-256 is stored, one use, 30 days.
CREATE TABLE guest_links (
    id         TEXT PRIMARY KEY,
    capture_id TEXT NOT NULL UNIQUE REFERENCES captures (id),
    token_hash TEXT NOT NULL UNIQUE CHECK (length(token_hash) = 64),
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    used_at    TEXT
) STRICT;

-- Short-lived session a guest page gets in exchange for the fragment token.
CREATE TABLE guest_sessions (
    id         TEXT PRIMARY KEY,
    link_id    TEXT NOT NULL REFERENCES guest_links (id),
    token_hash TEXT NOT NULL UNIQUE CHECK (length(token_hash) = 64),
    expires_at TEXT NOT NULL
) STRICT;

-- Spec §5 "Guest Feedback": integer rating 1–5, optional comment up to 2,000 characters.
-- No guest identity of any kind (Spec §6).
CREATE TABLE guest_feedback (
    id         TEXT PRIMARY KEY,
    link_id    TEXT NOT NULL UNIQUE REFERENCES guest_links (id),
    rating     INTEGER NOT NULL CHECK (rating BETWEEN 1 AND 5),
    comment    TEXT CHECK (comment IS NULL OR length(comment) BETWEEN 1 AND 2000),
    created_at TEXT NOT NULL
) STRICT;

-- ONB-1: who agreed to a website import, for which site, and when.
CREATE TABLE onboarding_consents (
    id          TEXT PRIMARY KEY,
    org_id      TEXT NOT NULL REFERENCES organizations (id),
    staff_id    TEXT NOT NULL REFERENCES staff (id),
    site_url    TEXT NOT NULL,
    statement   TEXT NOT NULL,
    accepted_at TEXT NOT NULL
) STRICT;
