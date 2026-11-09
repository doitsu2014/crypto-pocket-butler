-- Reverse of 0001_init.up.sql.
--
-- The guard is the important part: a rollback must never delete six months of
-- trail as a side effect. If any record is still inside the retention window,
-- this migration refuses and an operator decides. It succeeds on a fresh schema,
-- which is what the pipeline's up/down/re-up check exercises.

DO $$
BEGIN
    IF to_regclass('audit_events') IS NOT NULL THEN
        IF EXISTS (
            SELECT 1 FROM audit_events WHERE recorded_at > now() - interval '6 months'
        ) THEN
            RAISE EXCEPTION
                'refusing to drop the audit trail: it holds records inside the six-month retention window';
        END IF;
        -- A legally held record must survive a rollback too, even once it is
        -- past the ordinary window.
        IF EXISTS (
            SELECT 1 FROM audit_events WHERE legal_basis IS NOT NULL
        ) THEN
            RAISE EXCEPTION
                'refusing to drop the audit trail: it holds legally held records';
        END IF;
    END IF;
END
$$;

DROP TABLE IF EXISTS privacy_deletions;
DROP TABLE IF EXISTS audit_exports;
DROP TABLE IF EXISTS retention_runs;
DROP TABLE IF EXISTS support_access_records;
DROP TABLE IF EXISTS append_idempotency;
DROP TABLE IF EXISTS audit_events CASCADE;

DROP FUNCTION IF EXISTS drop_audit_partition(text, timestamptz);
DROP FUNCTION IF EXISTS ensure_audit_partition(date);
DROP FUNCTION IF EXISTS create_audit_partition(date);
DROP FUNCTION IF EXISTS enforce_app_insert_shape();
DROP FUNCTION IF EXISTS refuse_terminal_update();

-- The roles are cluster bootstrap objects, not this schema's objects: the up
-- migration creates them idempotently so a disposable database converges, and a
-- rollback leaves them in place. Dropping a role that another schema has since
-- granted would fail the rollback for the wrong reason; revoking this schema's
-- grants happens implicitly when the tables above are dropped.
