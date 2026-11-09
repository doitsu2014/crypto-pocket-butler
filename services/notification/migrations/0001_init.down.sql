-- Reverse of 0001_init.up.sql.
--
-- The guard refuses to drop alerts that are still queued: a rollback would
-- silently lose a notification nobody has delivered yet. It succeeds on a fresh
-- schema, which is what the pipeline's up/down/re-up check exercises.

DO $$
BEGIN
    IF to_regclass('alerts') IS NOT NULL THEN
        IF EXISTS (SELECT 1 FROM alerts WHERE status = 'queued') THEN
            RAISE EXCEPTION
                'refusing to drop notification alerts: undelivered alerts are still queued';
        END IF;
    END IF;
END
$$;

DROP TABLE IF EXISTS service_token_registry;
DROP TABLE IF EXISTS delivery_control;
DROP TABLE IF EXISTS delivery_attempts;
DROP TABLE IF EXISTS alerts;
DROP TABLE IF EXISTS notification_preferences;

-- The roles are cluster bootstrap objects: the up migration creates them
-- idempotently so a disposable database converges, and a rollback leaves them in
-- place.
