-- W3a: what the sync client needs to resume after any interruption.
-- The server's media id, so a retry doesn't upload the photo again.
ALTER TABLE captures ADD COLUMN remote_media_id TEXT;
-- The acknowledgement (Spec §3): the server's capture id and the guest link
-- it issued. The link carries a capability token; it stays in this app's
-- private storage (backups are disabled) and is shown only as the QR.
ALTER TABLE captures ADD COLUMN server_capture_id TEXT;
ALTER TABLE captures ADD COLUMN guest_url TEXT;
ALTER TABLE captures ADD COLUMN guest_expires_at TEXT;
ALTER TABLE captures ADD COLUMN sync_attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE captures ADD COLUMN last_sync_error TEXT;
ALTER TABLE captures ADD COLUMN last_attempt_utc TEXT;
