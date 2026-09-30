-- Local-first capture outbox (spec §3, §5 "Captures").
CREATE TABLE captures (
    id               INTEGER PRIMARY KEY,
    -- Client-generated id; the idempotency key for POST /captures.
    client_id        TEXT    NOT NULL UNIQUE,
    staff_id         TEXT    NOT NULL,
    media_sha256     TEXT    NOT NULL CHECK (length(media_sha256) = 64 AND media_sha256 NOT GLOB '*[^0-9a-f]*'),
    capture_time_utc TEXT    NOT NULL,
    sync_state       TEXT    NOT NULL DEFAULT 'pending' CHECK (sync_state IN ('pending', 'synced')),
    synced_at_utc    TEXT
) STRICT;

CREATE INDEX captures_pending ON captures (capture_time_utc) WHERE sync_state = 'pending';
