-- W1: keep the media type and size with each capture (Spec §5 "Media Assets").
ALTER TABLE captures ADD COLUMN media_mime TEXT;
ALTER TABLE captures ADD COLUMN media_bytes INTEGER;
