-- Portfolio schema: portfolios and account membership, targets and guardrails,
-- immutable valuation snapshots, trade overlays, advisory order intents,
-- recorded trades, the single-flight valuation run lease, the service-token
-- registry, and the privacy ledger.
--
-- Object names are unqualified on purpose: the migration runs with `search_path`
-- set to the service's own schema, and adapter tests point the same migration at
-- a disposable schema. No cross-schema grants, no Row Level Security.
--
-- Roles are created idempotently so a disposable test database and production
-- converge on the same shape:
--   portfolio_svc       read/write the service's own records, insert-only snapshots
--   portfolio_readonly  select only, for operator tooling
--   portfolio_migrator  DDL
--
-- The roles are NOLOGIN: a migration must not mint a login or a password. The
-- platform's Vault bootstrap owns LOGIN and the credential for each role.

-- ---------------------------------------------------------------------------
-- Roles
-- ---------------------------------------------------------------------------
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'portfolio_svc') THEN
        EXECUTE 'CREATE ROLE portfolio_svc NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'portfolio_readonly') THEN
        EXECUTE 'CREATE ROLE portfolio_readonly NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'portfolio_migrator') THEN
        EXECUTE 'CREATE ROLE portfolio_migrator NOLOGIN';
    END IF;
END
$$;

-- ---------------------------------------------------------------------------
-- Portfolios
-- ---------------------------------------------------------------------------
CREATE TABLE portfolios (
    id             uuid        PRIMARY KEY,
    user_id        uuid        NOT NULL,
    name           text        NOT NULL CHECK (char_length(name) BETWEEN 1 AND 60),
    base_currency  char(3)     NOT NULL CHECK (base_currency ~ '^[A-Z]{3}$'),
    created_at     timestamptz NOT NULL,
    updated_at     timestamptz NOT NULL,
    anonymised_at  timestamptz
);

-- BR1.2: a name is unique per user ignoring case. An anonymised portfolio leaves
-- the unique set so several anonymised rows can coexist.
CREATE UNIQUE INDEX portfolios_name_unique
    ON portfolios (user_id, lower(name)) WHERE anonymised_at IS NULL;
CREATE INDEX portfolios_user_idx ON portfolios (user_id, created_at);

-- ---------------------------------------------------------------------------
-- Account membership (BR1.3: an account belongs to at most one portfolio)
-- ---------------------------------------------------------------------------
CREATE TABLE portfolio_accounts (
    id           uuid        PRIMARY KEY,
    portfolio_id uuid        NOT NULL REFERENCES portfolios (id) ON DELETE CASCADE,
    account_id   uuid        NOT NULL UNIQUE,
    added_at     timestamptz NOT NULL
);

CREATE INDEX portfolio_accounts_portfolio_idx ON portfolio_accounts (portfolio_id);
CREATE INDEX portfolio_accounts_account_idx ON portfolio_accounts (account_id);

-- ---------------------------------------------------------------------------
-- Targets and guardrails
-- ---------------------------------------------------------------------------
CREATE TABLE target_allocations (
    id             uuid          PRIMARY KEY,
    portfolio_id   uuid          NOT NULL REFERENCES portfolios (id) ON DELETE CASCADE,
    asset_id       text          NOT NULL CHECK (char_length(asset_id) BETWEEN 1 AND 200),
    target_percent numeric(5,2)  NOT NULL CHECK (target_percent >= 0 AND target_percent <= 100),
    updated_at     timestamptz   NOT NULL,
    UNIQUE (portfolio_id, asset_id)
);

CREATE INDEX target_allocations_portfolio_idx ON target_allocations (portfolio_id);

-- BR7.1: the crossing state lives with the guardrail so one crossing notifies
-- once. A guardrail without a target is refused by the use case (BR3.4).
CREATE TABLE guardrails (
    id                uuid          PRIMARY KEY,
    portfolio_id      uuid          NOT NULL REFERENCES portfolios (id) ON DELETE CASCADE,
    asset_id          text          NOT NULL CHECK (char_length(asset_id) BETWEEN 1 AND 200),
    threshold_points  numeric(6,2)  NOT NULL CHECK (threshold_points > 0),
    is_outside        boolean       NOT NULL DEFAULT false,
    last_crossing_key text          CHECK (last_crossing_key IS NULL OR char_length(last_crossing_key) BETWEEN 1 AND 400),
    crossed_at        timestamptz,
    updated_at        timestamptz   NOT NULL,
    UNIQUE (portfolio_id, asset_id),
    CONSTRAINT guardrails_crossing_shape CHECK (
        (is_outside AND last_crossing_key IS NOT NULL AND crossed_at IS NOT NULL)
        OR (NOT is_outside AND last_crossing_key IS NULL AND crossed_at IS NULL)
    )
);

CREATE INDEX guardrails_portfolio_idx ON guardrails (portfolio_id);

-- ---------------------------------------------------------------------------
-- Immutable valuation snapshots (BR4.1–BR4.2)
-- ---------------------------------------------------------------------------
CREATE TABLE valuation_snapshots (
    id            uuid        PRIMARY KEY,
    portfolio_id  uuid        NOT NULL REFERENCES portfolios (id) ON DELETE CASCADE,
    as_of         timestamptz NOT NULL,
    hour_bucket   timestamptz NOT NULL,
    total_value   numeric(38,18) NOT NULL,
    currency      char(3)     NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    inputs_digest char(64)    NOT NULL CHECK (inputs_digest ~ '^[0-9a-f]{64}$'),
    inputs        jsonb       NOT NULL CHECK (jsonb_typeof(inputs) = 'object'),
    lines         jsonb       NOT NULL CHECK (jsonb_typeof(lines) = 'object'),
    CONSTRAINT valuation_snapshots_hour_bucket_minute CHECK (
        date_trunc('hour', hour_bucket) = hour_bucket
    ),
    UNIQUE (portfolio_id, hour_bucket)
);

CREATE INDEX valuation_snapshots_portfolio_idx
    ON valuation_snapshots (portfolio_id, as_of DESC);

-- ---------------------------------------------------------------------------
-- Trade overlays (BR6.1)
-- ---------------------------------------------------------------------------
CREATE TABLE trade_overlays (
    id                uuid           PRIMARY KEY,
    recorded_trade_id uuid           NOT NULL UNIQUE,
    portfolio_id      uuid           NOT NULL REFERENCES portfolios (id) ON DELETE CASCADE,
    account_id        uuid,
    asset_id          text           NOT NULL CHECK (char_length(asset_id) BETWEEN 1 AND 200),
    quantity_delta    numeric(38,18) NOT NULL CHECK (quantity_delta <> 0),
    applied_at        timestamptz    NOT NULL
);

CREATE INDEX trade_overlays_portfolio_idx
    ON trade_overlays (portfolio_id, asset_id);

-- ---------------------------------------------------------------------------
-- Advisory order intents (BR5.1–BR5.3)
-- ---------------------------------------------------------------------------
CREATE TABLE order_intents (
    id                      uuid          PRIMARY KEY,
    portfolio_id            uuid          NOT NULL REFERENCES portfolios (id) ON DELETE CASCADE,
    asset_id                text          NOT NULL CHECK (char_length(asset_id) BETWEEN 1 AND 200),
    side                    text          NOT NULL CHECK (side IN ('buy', 'sell')),
    quantity                numeric(38,18) NOT NULL CHECK (quantity > 0),
    projected_allocation    numeric(6,2)  NOT NULL CHECK (projected_allocation >= 0),
    projected_drift_points  numeric(7,2),
    shortfall               numeric(38,18) CHECK (shortfall IS NULL OR shortfall > 0),
    status                  text          NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'cancelled')),
    created_at              timestamptz   NOT NULL,
    cancelled_at            timestamptz,
    CONSTRAINT order_intents_cancelled_shape CHECK (
        (status = 'cancelled' AND cancelled_at IS NOT NULL)
        OR (status = 'pending' AND cancelled_at IS NULL)
    )
);

CREATE INDEX order_intents_portfolio_idx ON order_intents (portfolio_id, created_at DESC);

-- ---------------------------------------------------------------------------
-- Recorded trades (BR6.1–BR6.2)
-- ---------------------------------------------------------------------------
CREATE TABLE recorded_trades (
    id                    uuid           PRIMARY KEY,
    portfolio_id          uuid           NOT NULL REFERENCES portfolios (id) ON DELETE CASCADE,
    user_id               uuid           NOT NULL,
    asset_id              text           NOT NULL CHECK (char_length(asset_id) BETWEEN 1 AND 200),
    side                  text           NOT NULL CHECK (side IN ('buy', 'sell')),
    quantity              numeric(38,18) NOT NULL CHECK (quantity > 0),
    price                 numeric(38,18) NOT NULL CHECK (price >= 0),
    price_source          text           NOT NULL CHECK (price_source IN ('venue', 'user')),
    executed_at           timestamptz    NOT NULL,
    created_at            timestamptz    NOT NULL,
    idempotency_key       text           NOT NULL CHECK (char_length(idempotency_key) BETWEEN 1 AND 200),
    ledger_transaction_id text           CHECK (ledger_transaction_id IS NULL OR char_length(ledger_transaction_id) BETWEEN 1 AND 200),
    UNIQUE (portfolio_id, idempotency_key)
);

CREATE INDEX recorded_trades_user_idx ON recorded_trades (user_id, created_at DESC);

-- ---------------------------------------------------------------------------
-- Single-flight valuation run lease (NFR1.4)
-- ---------------------------------------------------------------------------
CREATE TABLE valuation_runs (
    id          uuid        PRIMARY KEY,
    job         text        NOT NULL CHECK (char_length(job) BETWEEN 1 AND 80),
    owner       text        NOT NULL CHECK (char_length(owner) BETWEEN 1 AND 120),
    started_at  timestamptz NOT NULL,
    lease_until timestamptz NOT NULL,
    finished_at timestamptz,
    status      text        NOT NULL DEFAULT 'running'
        CHECK (status IN ('running', 'succeeded', 'failed')),
    detail      text        CHECK (detail IS NULL OR char_length(detail) <= 500)
);

CREATE UNIQUE INDEX valuation_runs_one_live
    ON valuation_runs (job) WHERE status = 'running';
CREATE INDEX valuation_runs_running_idx
    ON valuation_runs (lease_until) WHERE status = 'running';

-- ---------------------------------------------------------------------------
-- Internal service-token registry (token hash -> caller -> allow-list)
-- ---------------------------------------------------------------------------
CREATE TABLE service_token_registry (
    id         uuid        PRIMARY KEY,
    token_hash char(64)    NOT NULL UNIQUE CHECK (token_hash ~ '^[0-9a-f]{64}$'),
    -- A caller may hold several active rows during a rotation; the unique key is
    -- the token hash, so insert-then-delete keeps both tokens resolving while
    -- consumers roll.
    caller     text        NOT NULL CHECK (char_length(caller) BETWEEN 1 AND 80),
    operations text[]      NOT NULL CHECK (array_length(operations, 1) >= 1),
    active     boolean     NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL
);

CREATE INDEX service_token_registry_caller_idx
    ON service_token_registry (caller) WHERE active;

-- ---------------------------------------------------------------------------
-- Privacy anonymisation ledger (a retry applies once)
--
-- The row holds only the random opaque reference that replaced the subject; the
-- original user id is never stored, so a completed erasure leaves no subject in
-- this schema.
-- ---------------------------------------------------------------------------
CREATE TABLE privacy_anonymisations (
    anonymised_ref      uuid        PRIMARY KEY,
    idempotency_key     text        NOT NULL UNIQUE CHECK (char_length(idempotency_key) BETWEEN 1 AND 200),
    retained_categories jsonb       NOT NULL DEFAULT '[]'::jsonb
        CHECK (jsonb_typeof(retained_categories) = 'array'),
    completed_at        timestamptz NOT NULL
);

-- ---------------------------------------------------------------------------
-- Grants
-- ---------------------------------------------------------------------------
DO $$
DECLARE
    schema_name text := coalesce(current_schema(), 'portfolio');
BEGIN
    EXECUTE format('GRANT USAGE ON SCHEMA %I TO portfolio_svc, portfolio_readonly, portfolio_migrator', schema_name);
    EXECUTE format('GRANT CREATE ON SCHEMA %I TO portfolio_migrator', schema_name);

    EXECUTE 'GRANT SELECT, INSERT ON portfolios TO portfolio_svc';
    -- Column-scoped UPDATE: the erasure path replaces the subject column, and the
    -- name/currency/timestamp path stays writable. Nothing else on the row is.
    EXECUTE 'GRANT UPDATE (name, base_currency, updated_at, user_id, anonymised_at) ON portfolios TO portfolio_svc';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON portfolio_accounts TO portfolio_svc';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON target_allocations TO portfolio_svc';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON guardrails TO portfolio_svc';
    -- BR4.1: insert-only. No UPDATE, no DELETE, no TRUNCATE.
    EXECUTE 'GRANT SELECT, INSERT ON valuation_snapshots TO portfolio_svc';
    EXECUTE 'GRANT SELECT, INSERT, DELETE ON trade_overlays TO portfolio_svc';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON order_intents TO portfolio_svc';
    EXECUTE 'GRANT SELECT, INSERT ON recorded_trades TO portfolio_svc';
    -- Erasure replaces only the subject column on recorded trades.
    EXECUTE 'GRANT UPDATE (user_id) ON recorded_trades TO portfolio_svc';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON valuation_runs TO portfolio_svc';
    EXECUTE 'GRANT SELECT ON service_token_registry TO portfolio_svc';
    EXECUTE 'GRANT SELECT, INSERT ON privacy_anonymisations TO portfolio_svc';

    IF to_regclass('_sqlx_migrations') IS NOT NULL THEN
        EXECUTE 'GRANT SELECT ON _sqlx_migrations TO portfolio_svc';
    END IF;

    EXECUTE 'GRANT SELECT ON portfolios TO portfolio_readonly';
    EXECUTE 'GRANT SELECT ON portfolio_accounts TO portfolio_readonly';
    EXECUTE 'GRANT SELECT ON target_allocations TO portfolio_readonly';
    EXECUTE 'GRANT SELECT ON guardrails TO portfolio_readonly';
    EXECUTE 'GRANT SELECT ON valuation_snapshots TO portfolio_readonly';
    EXECUTE 'GRANT SELECT ON trade_overlays TO portfolio_readonly';
    EXECUTE 'GRANT SELECT ON order_intents TO portfolio_readonly';
    EXECUTE 'GRANT SELECT ON recorded_trades TO portfolio_readonly';
    EXECUTE 'GRANT SELECT ON valuation_runs TO portfolio_readonly';
    EXECUTE 'GRANT SELECT ON service_token_registry TO portfolio_readonly';
    EXECUTE 'GRANT SELECT ON privacy_anonymisations TO portfolio_readonly';

    EXECUTE 'GRANT ALL ON portfolios TO portfolio_migrator';
    EXECUTE 'GRANT ALL ON portfolio_accounts TO portfolio_migrator';
    EXECUTE 'GRANT ALL ON target_allocations TO portfolio_migrator';
    EXECUTE 'GRANT ALL ON guardrails TO portfolio_migrator';
    EXECUTE 'GRANT ALL ON valuation_snapshots TO portfolio_migrator';
    EXECUTE 'GRANT ALL ON trade_overlays TO portfolio_migrator';
    EXECUTE 'GRANT ALL ON order_intents TO portfolio_migrator';
    EXECUTE 'GRANT ALL ON recorded_trades TO portfolio_migrator';
    EXECUTE 'GRANT ALL ON valuation_runs TO portfolio_migrator';
    EXECUTE 'GRANT ALL ON service_token_registry TO portfolio_migrator';
    EXECUTE 'GRANT ALL ON privacy_anonymisations TO portfolio_migrator';
END
$$;
