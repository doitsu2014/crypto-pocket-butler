-- Reverse of 0002_alert_channel.up.sql. Dropping the column loses only the
-- emitter's channel request; delivery falls back to the pre-C9 behaviour.

ALTER TABLE alerts DROP COLUMN IF EXISTS requested_channel;
