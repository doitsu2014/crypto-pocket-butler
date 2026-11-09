-- Reverse of 0001_init.up.sql.
--
-- The guard is the important part: a rollback must never silently delete real
-- market observations. If the schema holds any observation, this migration
-- refuses and an operator decides. It succeeds on a fresh schema, which is what
-- the pipeline's up/down/re-up check exercises.

DO $$
BEGIN
    IF to_regclass('price_observations') IS NOT NULL
        AND EXISTS (SELECT 1 FROM price_observations) THEN
        RAISE EXCEPTION
            'refusing to drop the market_data schema: it holds price observations';
    END IF;
END
$$;

DROP FUNCTION IF EXISTS read_provider_secret(text);
DROP TABLE IF EXISTS source_rate_buckets;
DROP TABLE IF EXISTS service_token_registry;
DROP TABLE IF EXISTS refresh_runs;
DROP TABLE IF EXISTS fx_rates;
DROP TABLE IF EXISTS price_observations;
DROP TABLE IF EXISTS asset_price_tiers;

-- The role is a cluster bootstrap object, not this schema's object: the up
-- migration creates it idempotently so a disposable database converges, and a
-- rollback leaves it in place.
