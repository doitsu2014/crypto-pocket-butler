-- 0009 up — anchor the absolute lifetime to each provider session (BR2.1, FR2.1).
-- Existing rows have no recorded sign-in timestamp. Expire them at migration time
-- rather than extending their cap from last_activity_at. The default lets old
-- Identity writers insert sessions during a start-first rollout; new code writes
-- the actual first-seen time and subsequent updates preserve it.

ALTER TABLE device_sessions
    ADD COLUMN first_seen_at timestamptz NOT NULL DEFAULT clock_timestamp();

-- The original sign-in time is unknown for rows that predate this migration.
-- Expire those rows at the migration boundary instead of extending their cap
-- from last_activity_at. Inserts from the old writer use the DEFAULT above and
-- remain valid during a start-first rollout.
UPDATE device_sessions
SET first_seen_at = transaction_timestamp() - INTERVAL '30 days';

-- No expiry index is created: current Identity queries do not filter on the
-- derived expiry, and timestamptz-plus-interval is not a safe immutable index key.
