-- Reverse of 0001_init.up.sql.
--
-- The guard is the important part: a rollback must never silently delete real
-- portfolios, snapshots, or trades. If the schema holds any portfolio, this
-- migration refuses and an operator decides. It succeeds on a fresh schema,
-- which is what the pipeline's up/down/re-up check exercises.

DO $$
BEGIN
    IF to_regclass('portfolios') IS NOT NULL AND EXISTS (SELECT 1 FROM portfolios) THEN
        RAISE EXCEPTION
            'refusing to drop the portfolio schema: it holds portfolio records';
    END IF;
END
$$;

DROP TABLE IF EXISTS privacy_anonymisations;
DROP TABLE IF EXISTS service_token_registry;
DROP TABLE IF EXISTS valuation_runs;
DROP TABLE IF EXISTS recorded_trades;
DROP TABLE IF EXISTS order_intents;
DROP TABLE IF EXISTS trade_overlays;
DROP TABLE IF EXISTS valuation_snapshots;
DROP TABLE IF EXISTS guardrails;
DROP TABLE IF EXISTS target_allocations;
DROP TABLE IF EXISTS portfolio_accounts;
DROP TABLE IF EXISTS portfolios;

-- The roles are cluster bootstrap objects, not this schema's objects: the up
-- migration creates them idempotently so a disposable database converges, and a
-- rollback leaves them in place.
