-- Market Data schema: per-asset freshness tiers, price observations, FX
-- readings, refresh runs with their single-flight leases, the inbound
-- service-token registry, and the shared per-source rate buckets.
--
-- Object names are unqualified on purpose: the migration runs with `search_path`
-- set to the service's own schema, and adapter tests point the same migration at
-- a disposable schema. No cross-schema grants, no Row Level Security.
--
-- The service has one role, created idempotently so a disposable test database
-- and production converge on the same shape:
--   market_data_svc       the runtime and the migrate job (the release job
--                         mounts the same DSN for both)
--
-- The role is NOLOGIN: a migration must not mint a login or a password. The
-- platform's Vault bootstrap owns LOGIN and the credential for the role.

-- ---------------------------------------------------------------------------
-- Role
-- ---------------------------------------------------------------------------
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'market_data_svc') THEN
        EXECUTE 'CREATE ROLE market_data_svc NOLOGIN';
    END IF;
END
$$;

-- ---------------------------------------------------------------------------
-- Per-asset freshness tier (WF4, BR2.1)
-- ---------------------------------------------------------------------------
CREATE TABLE asset_price_tiers (
    asset_id      text        PRIMARY KEY CHECK (char_length(asset_id) BETWEEN 1 AND 200),
    rank          integer     NOT NULL CHECK (rank >= 1),
    tier          text        NOT NULL CHECK (tier IN ('top100', 'tail')),
    bound_seconds integer     NOT NULL CHECK (bound_seconds > 0),
    updated_at    timestamptz NOT NULL,
    CONSTRAINT asset_price_tiers_tier_matches_bound CHECK (
        (tier = 'top100' AND bound_seconds = 300)
        OR (tier = 'tail' AND bound_seconds = 900)
    ),
    CONSTRAINT asset_price_tiers_rank_matches_tier CHECK (
        (tier = 'top100' AND rank <= 100) OR (tier = 'tail' AND rank > 100)
    )
);

CREATE INDEX asset_price_tiers_tier_idx ON asset_price_tiers (tier, rank);

-- ---------------------------------------------------------------------------
-- Price observations (accepted and rejected)
-- ---------------------------------------------------------------------------
CREATE TABLE price_observations (
    id               uuid           PRIMARY KEY,
    asset_id         text           NOT NULL CHECK (char_length(asset_id) BETWEEN 1 AND 200),
    source_name      text           NOT NULL CHECK (char_length(source_name) BETWEEN 1 AND 80),
    price            numeric(38,18) NOT NULL,
    currency         text           NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    observed_at      timestamptz    NOT NULL,
    received_at      timestamptz    NOT NULL,
    status           text           NOT NULL DEFAULT 'accepted'
        CHECK (status IN ('accepted', 'rejected')),
    rejection_reason text           CHECK (
        rejection_reason IS NULL OR char_length(rejection_reason) BETWEEN 1 AND 300
    ),
    CONSTRAINT price_observations_accepted_positive CHECK (
        status <> 'accepted' OR price > 0
    ),
    CONSTRAINT price_observations_rejected_reason CHECK (
        status <> 'rejected' OR rejection_reason IS NOT NULL
    )
);

-- NFR2.1: the read path takes the newest accepted observation per source with
-- DISTINCT ON (asset_id) ... ORDER BY asset_id, observed_at DESC.
CREATE INDEX price_observations_asset_source_idx
    ON price_observations (asset_id, source_name, observed_at DESC);
CREATE INDEX price_observations_accepted_idx
    ON price_observations (asset_id, observed_at DESC) WHERE status = 'accepted';
CREATE INDEX price_observations_received_idx
    ON price_observations (received_at);

-- ---------------------------------------------------------------------------
-- FX readings
-- ---------------------------------------------------------------------------
CREATE TABLE fx_rates (
    id          uuid           PRIMARY KEY,
    base        text           NOT NULL CHECK (base ~ '^[A-Z]{3}$'),
    quote       text           NOT NULL CHECK (quote ~ '^[A-Z]{3}$'),
    rate        numeric(38,18) NOT NULL CHECK (rate > 0),
    source      text           NOT NULL CHECK (char_length(source) BETWEEN 1 AND 80),
    observed_at timestamptz    NOT NULL,
    received_at timestamptz    NOT NULL,
    CONSTRAINT fx_rates_distinct_pair CHECK (base <> quote)
);

CREATE INDEX fx_rates_pair_idx ON fx_rates (base, quote, observed_at DESC);
CREATE INDEX fx_rates_received_idx ON fx_rates (received_at);

-- ---------------------------------------------------------------------------
-- Refresh runs and the per-job single-flight lease (BR4.1)
-- ---------------------------------------------------------------------------
CREATE TABLE refresh_runs (
    id              uuid        PRIMARY KEY,
    job             text        NOT NULL CHECK (job IN ('top100', 'tail')),
    started_at      timestamptz NOT NULL,
    lease_until     timestamptz NOT NULL,
    finished_at     timestamptz,
    status          text        NOT NULL DEFAULT 'running'
        CHECK (status IN ('running', 'succeeded', 'failed')),
    requested_count integer     NOT NULL DEFAULT 0 CHECK (requested_count >= 0),
    priced_count    integer     NOT NULL DEFAULT 0 CHECK (priced_count >= 0),
    unpriced_count  integer     NOT NULL DEFAULT 0 CHECK (unpriced_count >= 0),
    error_category  text        CHECK (error_category IS NULL OR error_category IN
        ('rate_limit', 'provider', 'network', 'data')),
    error_message   text        CHECK (error_message IS NULL OR char_length(error_message) <= 500),
    CONSTRAINT refresh_runs_counts_reconcile CHECK (
        status = 'running'
        OR requested_count = priced_count + unpriced_count
    ),
    CONSTRAINT refresh_runs_failed_shape CHECK (
        status <> 'failed' OR error_category IS NOT NULL
    )
);

-- BR4.1: at most one live run per job. The top100 and tail jobs hold separate
-- leases and may run concurrently. The claim path expires an overdue run before
-- inserting, so an expired lease never blocks a later run.
CREATE UNIQUE INDEX refresh_runs_one_live
    ON refresh_runs (job) WHERE status = 'running';
CREATE INDEX refresh_runs_job_status_idx
    ON refresh_runs (job, status, lease_until);
CREATE INDEX refresh_runs_started_idx
    ON refresh_runs (started_at DESC);

-- ---------------------------------------------------------------------------
-- Inbound service-token registry (token hash -> caller -> allow-list)
-- ---------------------------------------------------------------------------
CREATE TABLE service_token_registry (
    id         uuid        PRIMARY KEY,
    token_hash char(64)    NOT NULL UNIQUE CHECK (token_hash ~ '^[0-9a-f]{64}$'),
    caller     text        NOT NULL UNIQUE CHECK (char_length(caller) BETWEEN 1 AND 80),
    operations text[]      NOT NULL CHECK (array_length(operations, 1) >= 1),
    active     boolean     NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL
);

-- ---------------------------------------------------------------------------
-- Shared per-source rate buckets (BR4.3)
-- ---------------------------------------------------------------------------
CREATE TABLE source_rate_buckets (
    source_name text        PRIMARY KEY CHECK (char_length(source_name) BETWEEN 1 AND 80),
    tokens      integer     NOT NULL CHECK (tokens >= 0),
    updated_at  timestamptz NOT NULL
);

-- ---------------------------------------------------------------------------
-- Provider secrets (Supabase Vault, read through a SECURITY DEFINER function)
-- ---------------------------------------------------------------------------
-- The function is created even when Vault is absent (a disposable test
-- database); it returns NULL rather than failing. It uses dynamic SQL so the
-- migration does not depend on the Vault schema at parse time, and it never
-- exposes a cross-schema grant.
CREATE OR REPLACE FUNCTION read_provider_secret(source_name text)
RETURNS text
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = market_data, pg_temp
AS $$
DECLARE
    secret text;
BEGIN
    IF to_regclass('vault.decrypted_secrets') IS NULL THEN
        RETURN NULL;
    END IF;
    EXECUTE
        'SELECT decrypted_secret FROM vault.decrypted_secrets WHERE name = $1 LIMIT 1'
        INTO secret
        USING 'market_data_provider_' || source_name;
    RETURN secret;
END
$$;

-- ---------------------------------------------------------------------------
-- Grants
-- ---------------------------------------------------------------------------
DO $$
DECLARE
    schema_name text := coalesce(current_schema(), 'market_data');
BEGIN
    EXECUTE format('GRANT USAGE, CREATE ON SCHEMA %I TO market_data_svc', schema_name);

    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON asset_price_tiers TO market_data_svc';
    EXECUTE 'GRANT SELECT, INSERT, DELETE ON price_observations TO market_data_svc';
    EXECUTE 'GRANT SELECT, INSERT, DELETE ON fx_rates TO market_data_svc';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON refresh_runs TO market_data_svc';
    -- DELETE lets the deploy job rotate a credential by inserting the new hash
    -- and deleting the old row in one step.
    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON service_token_registry TO market_data_svc';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON source_rate_buckets TO market_data_svc';
    EXECUTE 'GRANT EXECUTE ON FUNCTION read_provider_secret(text) TO market_data_svc';

    IF to_regclass('_sqlx_migrations') IS NOT NULL THEN
        EXECUTE 'GRANT SELECT ON _sqlx_migrations TO market_data_svc';
    END IF;

    -- The SECURITY DEFINER function runs as market_data_svc, the role that owns
    -- it. Without this grant a Vault read fails loudly (insufficient_privilege)
    -- and the run is recorded failed and audited; it is never silenced.
    IF to_regclass('vault.decrypted_secrets') IS NOT NULL THEN
        EXECUTE 'GRANT SELECT ON vault.decrypted_secrets TO market_data_svc';
    END IF;
END
$$;
