-- 0009 down — remove the session-cap anchor.
-- The first-seen value is recoverable only while this column exists; rollback
-- restores the previous session schema without deleting session rows.

ALTER TABLE device_sessions DROP COLUMN IF EXISTS first_seen_at;
