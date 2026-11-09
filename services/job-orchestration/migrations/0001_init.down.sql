-- Reverse of 0001_init.up.sql. It refuses to drop a schema that holds runs.

DO $$
BEGIN
    IF to_regclass('job_runs') IS NOT NULL AND EXISTS (SELECT 1 FROM job_runs) THEN
        RAISE EXCEPTION 'refusing to drop the job-orchestration schema: it holds run records';
    END IF;
END
$$;

DROP TABLE IF EXISTS service_token_registry;
DROP TABLE IF EXISTS prune_state;
DROP TABLE IF EXISTS backfill_requests;
DROP TABLE IF EXISTS heartbeat;
DROP TABLE IF EXISTS orchestrator_lease;
DROP TABLE IF EXISTS job_runs;
DROP TABLE IF EXISTS job_definitions;
