-- Reverse of 0001_init.up.sql.
--
-- The guard refuses to delete six months of raw events as a rollback side
-- effect. It succeeds on a fresh schema, which is what the pipeline's
-- up/down/re-up check exercises.

DO $$
BEGIN
    IF to_regclass('events') IS NOT NULL THEN
        IF EXISTS (SELECT 1 FROM events WHERE ingested_at > now() - interval '6 months') THEN
            RAISE EXCEPTION
                'refusing to drop analytics events: rows are inside the six-month retention window';
        END IF;
    END IF;
END
$$;

DROP FUNCTION IF EXISTS read_analytics_salt(text);

DROP TABLE IF EXISTS salt_versions;
DROP TABLE IF EXISTS service_token_registry;
DROP TABLE IF EXISTS feature_flags;
DROP TABLE IF EXISTS metric_aggregates;
DROP TABLE IF EXISTS consent_checks;
DROP TABLE IF EXISTS events;

-- The roles are cluster bootstrap objects: the up migration creates them
-- idempotently so a disposable database converges, and a rollback leaves them in
-- place. Dropping a role another schema has since granted would fail the
-- rollback for the wrong reason.
