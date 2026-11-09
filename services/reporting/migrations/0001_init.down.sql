-- Reverse of 0001_init.up.sql.
--
-- The guard is the important part: a rollback must never silently delete real
-- briefs. If the schema holds any brief, this migration refuses and an operator
-- decides. It succeeds on a fresh schema, which is what the pipeline's
-- up/down/re-up check exercises.

DO $$
BEGIN
    IF to_regclass('briefs') IS NOT NULL AND EXISTS (SELECT 1 FROM briefs) THEN
        RAISE EXCEPTION
            'refusing to drop the reporting schema: it holds brief records';
    END IF;
END
$$;

DROP TABLE IF EXISTS privacy_anonymisations;
DROP TABLE IF EXISTS service_token_registry;
DROP TABLE IF EXISTS brief_runs;
DROP TABLE IF EXISTS brief_deliveries;
DROP TABLE IF EXISTS briefs;

-- The roles are cluster bootstrap objects, not this schema's objects: the up
-- migration creates them idempotently so a disposable database converges, and a
-- rollback leaves them in place.
