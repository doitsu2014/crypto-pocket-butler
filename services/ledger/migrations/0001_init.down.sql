-- Reverse of 0001_init.up.sql.
--
-- The guard refuses to drop a ledger that still holds financial rows. It
-- succeeds on a fresh schema, which is what the pipeline's up/down/re-up check
-- exercises.
DO $$
BEGIN
    IF to_regclass('transactions') IS NOT NULL THEN
        IF EXISTS (SELECT 1 FROM transactions LIMIT 1) THEN
            RAISE EXCEPTION
                'refusing to drop the ledger: append-only financial rows are present';
        END IF;
    END IF;
END
$$;

DROP TABLE IF EXISTS service_token_registry;
DROP TABLE IF EXISTS reconcile_leases;
DROP TABLE IF EXISTS anonymisation_requests;
DROP TABLE IF EXISTS pnl_snapshots;
DROP TABLE IF EXISTS cost_basis_lots;
DROP TABLE IF EXISTS transfer_pairs;
DROP TABLE IF EXISTS corrections;
DROP TABLE IF EXISTS transactions;
DROP TABLE IF EXISTS ledger_subjects;

-- The roles are cluster bootstrap objects: the up migration creates them
-- idempotently so a disposable database converges, and a rollback leaves them in
-- place.
