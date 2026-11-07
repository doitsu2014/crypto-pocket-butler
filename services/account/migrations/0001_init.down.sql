-- Reverse of 0001_init.up.sql.
--
-- The guard is the important part: a rollback must never silently delete real
-- account, credential, or holding records. If the schema holds any account, this
-- migration refuses and an operator decides. It succeeds on a fresh schema, which
-- is what the pipeline's up/down/re-up check exercises.

DO $$
BEGIN
    IF to_regclass('accounts') IS NOT NULL AND EXISTS (SELECT 1 FROM accounts) THEN
        RAISE EXCEPTION
            'refusing to drop the account schema: it holds account records';
    END IF;
END
$$;

DROP TABLE IF EXISTS privacy_anonymisations;
DROP TABLE IF EXISTS service_token_registry;
DROP TABLE IF EXISTS import_row_errors;
DROP TABLE IF EXISTS imported_transactions;
DROP TABLE IF EXISTS import_upload;
DROP TABLE IF EXISTS import_batches;
DROP TABLE IF EXISTS holding_snapshots;
DROP TABLE IF EXISTS sync_runs;
DROP TABLE IF EXISTS wallet_connections;
DROP TABLE IF EXISTS credentials;
DROP TABLE IF EXISTS accounts;

-- The roles are cluster bootstrap objects, not this schema's objects: the up
-- migration creates them idempotently so a disposable database converges, and a
-- rollback leaves them in place.
