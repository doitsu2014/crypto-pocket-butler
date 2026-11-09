-- Ledger schema: append-only transactions and corrections, mutable subjects,
-- transfer pairings, derived FIFO lots and P&L snapshots, the service-token
-- registry, the anonymisation idempotency records, and the reconcile lease.
--
-- Object names are unqualified on purpose: the migration runs with `search_path`
-- set to the service's own schema, and adapter tests point the same migration at
-- a disposable schema. No cross-schema grants and no Row Level Security.
--
-- Roles are created idempotently so a disposable database and production
-- converge on the same shape:
--   ledger_svc         select/insert transactions and corrections (no update,
--                      no delete); select/insert/update transfer pairs, lots and
--                      snapshots (delete lots only for prune); select/insert
--                      anonymisation records; select/update the reconcile lease;
--                      select the token registry; insert/select but never update
--                      ledger_subjects
--   ledger_anonymiser  select/update ledger_subjects only
--   ledger_readonly    select only
--   ledger_migrator    DDL

-- ---------------------------------------------------------------------------
-- Roles
-- ---------------------------------------------------------------------------
DO $$
BEGIN
    BEGIN
        IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ledger_svc') THEN
            EXECUTE 'CREATE ROLE ledger_svc NOLOGIN';
        END IF;
        IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ledger_anonymiser') THEN
            EXECUTE 'CREATE ROLE ledger_anonymiser NOLOGIN';
        END IF;
        IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ledger_readonly') THEN
            EXECUTE 'CREATE ROLE ledger_readonly NOLOGIN';
        END IF;
        IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ledger_migrator') THEN
            EXECUTE 'CREATE ROLE ledger_migrator NOLOGIN';
        END IF;
    EXCEPTION WHEN insufficient_privilege THEN
        -- A disposable test database may not grant CREATEROLE. Production's
        -- Vault bootstrap owns the roles; the tables still converge here.
        RAISE NOTICE 'skipping ledger role creation: %', SQLERRM;
    END;
END
$$;

-- The roles are created NOLOGIN on purpose: a migration must not mint a login or
-- a password. The platform's Supabase Vault bootstrap owns the LOGIN attribute
-- and the credential for each role.

-- ---------------------------------------------------------------------------
-- Mutable subjects
-- ---------------------------------------------------------------------------
-- Transactions reference this row, not the user id, so anonymisation never
-- rewrites an append-only row (NFR9.2).
CREATE TABLE ledger_subjects (
    id             uuid        PRIMARY KEY,
    user_id        uuid        UNIQUE,
    anonymised_ref text        UNIQUE,
    created_at     timestamptz NOT NULL,
    CONSTRAINT ledger_subjects_identity CHECK (user_id IS NOT NULL OR anonymised_ref IS NOT NULL)
);

-- ---------------------------------------------------------------------------
-- Append-only transactions
-- ---------------------------------------------------------------------------
CREATE TABLE transactions (
    id             uuid        PRIMARY KEY,
    subject_id     uuid        NOT NULL REFERENCES ledger_subjects (id),
    account_id     uuid,
    asset_id       text        NOT NULL CHECK (char_length(asset_id) BETWEEN 1 AND 128),
    kind           text        NOT NULL
        CHECK (kind IN ('buy', 'sell', 'transfer_in', 'transfer_out', 'fee')),
    quantity       numeric     NOT NULL CHECK (quantity > 0),
    price          numeric     CHECK (price IS NULL OR price >= 0),
    price_source   text        CHECK (price_source IS NULL OR price_source IN ('venue', 'csv', 'user')),
    currency       text        NOT NULL CHECK (char_length(currency) BETWEEN 1 AND 8),
    source         text        NOT NULL CHECK (source IN ('venue', 'csv', 'recorded_trade')),
    external_ref   text,
    canonical_key  text        NOT NULL UNIQUE,
    external_key   text        UNIQUE,
    executed_at    timestamptz NOT NULL,
    recorded_at    timestamptz NOT NULL
);

CREATE INDEX transactions_list_idx ON transactions (subject_id, executed_at DESC, id DESC);
CREATE INDEX transactions_subject_asset_idx ON transactions (subject_id, asset_id, executed_at);

-- ---------------------------------------------------------------------------
-- Append-only corrections
-- ---------------------------------------------------------------------------
CREATE TABLE corrections (
    id             uuid        PRIMARY KEY,
    transaction_id uuid        NOT NULL REFERENCES transactions (id),
    field          text        NOT NULL
        CHECK (field IN ('quantity', 'price', 'kind', 'executedAt', 'assetId')),
    old_value      text        NOT NULL,
    new_value      text        NOT NULL,
    reason         text        NOT NULL CHECK (char_length(reason) BETWEEN 1 AND 500),
    actor_id       text        NOT NULL CHECK (char_length(actor_id) BETWEEN 1 AND 128),
    corrected_at   timestamptz NOT NULL
);

CREATE INDEX corrections_transaction_idx ON corrections (transaction_id, corrected_at, id);

-- ---------------------------------------------------------------------------
-- Transfer pairings
-- ---------------------------------------------------------------------------
CREATE TABLE transfer_pairs (
    id                 uuid        PRIMARY KEY,
    subject_id         uuid        NOT NULL REFERENCES ledger_subjects (id),
    out_transaction_id uuid        UNIQUE REFERENCES transactions (id),
    in_transaction_id  uuid        UNIQUE REFERENCES transactions (id),
    asset_id           text        NOT NULL,
    status             text        NOT NULL CHECK (status IN ('pending', 'matched', 'rejected')),
    matched_at         timestamptz,
    CONSTRAINT transfer_pairs_leg CHECK (
        out_transaction_id IS NOT NULL OR in_transaction_id IS NOT NULL
    ),
    CONSTRAINT transfer_pairs_matched CHECK (
        status <> 'matched'
        OR (out_transaction_id IS NOT NULL AND in_transaction_id IS NOT NULL AND matched_at IS NOT NULL)
    )
);

CREATE INDEX transfer_pairs_subject_idx ON transfer_pairs (subject_id, status);

-- ---------------------------------------------------------------------------
-- Derived FIFO lots
-- ---------------------------------------------------------------------------
CREATE TABLE cost_basis_lots (
    id                    uuid        PRIMARY KEY,
    subject_id            uuid        NOT NULL REFERENCES ledger_subjects (id),
    asset_id              text        NOT NULL,
    acquired_at           timestamptz NOT NULL,
    quantity              numeric     NOT NULL CHECK (quantity > 0),
    remaining             numeric     NOT NULL CHECK (remaining >= 0 AND remaining <= quantity),
    unit_cost             numeric     NOT NULL CHECK (unit_cost >= 0),
    currency              text        NOT NULL CHECK (char_length(currency) BETWEEN 1 AND 8),
    source_transaction_id uuid        NOT NULL REFERENCES transactions (id)
);

CREATE INDEX cost_basis_lots_fifo_idx ON cost_basis_lots (subject_id, asset_id, acquired_at, id);

-- ---------------------------------------------------------------------------
-- Derived P&L snapshots
-- ---------------------------------------------------------------------------
CREATE TABLE pnl_snapshots (
    id                     uuid        PRIMARY KEY,
    subject_id             uuid        NOT NULL REFERENCES ledger_subjects (id),
    portfolio_id           uuid,
    as_of                  timestamptz NOT NULL,
    currency               text        NOT NULL CHECK (char_length(currency) BETWEEN 1 AND 8),
    fx_as_of               timestamptz,
    fx_estimated           boolean     NOT NULL DEFAULT false,
    net_invested           numeric,
    realized               numeric,
    unrealized             numeric,
    total_return           numeric,
    total_acquisition_cost numeric,
    coverage               numeric     NOT NULL CHECK (coverage >= 0 AND coverage <= 1),
    coverage_gap           text[]      NOT NULL DEFAULT '{}',
    computed_at            timestamptz NOT NULL,
    CONSTRAINT pnl_snapshots_figures CHECK (
        (coverage > 0 AND net_invested IS NOT NULL)
        OR (coverage = 0 AND net_invested IS NULL)
    )
);

CREATE UNIQUE INDEX pnl_snapshots_user_idx
    ON pnl_snapshots (subject_id) WHERE portfolio_id IS NULL;
CREATE UNIQUE INDEX pnl_snapshots_portfolio_idx
    ON pnl_snapshots (subject_id, portfolio_id) WHERE portfolio_id IS NOT NULL;

-- ---------------------------------------------------------------------------
-- Anonymisation idempotency and the reconcile lease
-- ---------------------------------------------------------------------------
CREATE TABLE anonymisation_requests (
    idempotency_key text        PRIMARY KEY CHECK (char_length(idempotency_key) BETWEEN 1 AND 128),
    anonymised_ref  text        NOT NULL,
    completed_at    timestamptz NOT NULL
);

CREATE TABLE reconcile_leases (
    id                text        PRIMARY KEY,
    leased_until      timestamptz,
    last_completed_at timestamptz
);

INSERT INTO reconcile_leases (id) VALUES ('ledger-reconcile');

-- ---------------------------------------------------------------------------
-- Service-token registry
-- ---------------------------------------------------------------------------
-- Only hashes are stored. The deploy job holds write rights for rotation; the
-- runtime role reads only.
CREATE TABLE service_token_registry (
    service            text        PRIMARY KEY,
    token_sha256       text        NOT NULL UNIQUE CHECK (char_length(token_sha256) = 64),
    allowed_operations text[]      NOT NULL,
    rotated_at         timestamptz NOT NULL DEFAULT now()
);

-- ---------------------------------------------------------------------------
-- Grants
-- ---------------------------------------------------------------------------
DO $$
DECLARE
    schema_name text := coalesce(current_schema(), 'ledger');
BEGIN
    BEGIN
        EXECUTE format('GRANT USAGE ON SCHEMA %I TO ledger_svc, ledger_anonymiser, ledger_readonly, ledger_migrator', schema_name);
        EXECUTE format('GRANT CREATE ON SCHEMA %I TO ledger_migrator', schema_name);

        -- Append-only tables: never UPDATE or DELETE.
        EXECUTE 'GRANT SELECT, INSERT ON transactions TO ledger_svc';
        EXECUTE 'GRANT SELECT, INSERT ON corrections TO ledger_svc';
        -- The subject is INSERT/SELECT-only for the runtime role; the anonymiser
        -- role has the one UPDATE.
        EXECUTE 'GRANT SELECT, INSERT ON ledger_subjects TO ledger_svc';
        -- Derived state is replaced by the rebuild.
        EXECUTE 'GRANT SELECT, INSERT, UPDATE ON transfer_pairs TO ledger_svc';
        EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON cost_basis_lots TO ledger_svc';
        EXECUTE 'GRANT SELECT, INSERT, UPDATE ON pnl_snapshots TO ledger_svc';
        EXECUTE 'GRANT SELECT, INSERT ON anonymisation_requests TO ledger_svc';
        EXECUTE 'GRANT SELECT, UPDATE ON reconcile_leases TO ledger_svc';
        EXECUTE 'GRANT SELECT ON service_token_registry TO ledger_svc';

        EXECUTE 'GRANT SELECT, UPDATE ON ledger_subjects TO ledger_anonymiser';

        IF to_regclass('_sqlx_migrations') IS NOT NULL THEN
            EXECUTE 'GRANT SELECT ON _sqlx_migrations TO ledger_svc';
        END IF;

        EXECUTE 'GRANT SELECT ON ledger_subjects TO ledger_readonly';
        EXECUTE 'GRANT SELECT ON transactions TO ledger_readonly';
        EXECUTE 'GRANT SELECT ON corrections TO ledger_readonly';
        EXECUTE 'GRANT SELECT ON transfer_pairs TO ledger_readonly';
        EXECUTE 'GRANT SELECT ON cost_basis_lots TO ledger_readonly';
        EXECUTE 'GRANT SELECT ON pnl_snapshots TO ledger_readonly';
        EXECUTE 'GRANT SELECT ON anonymisation_requests TO ledger_readonly';
        EXECUTE 'GRANT SELECT ON reconcile_leases TO ledger_readonly';
        EXECUTE 'GRANT SELECT ON service_token_registry TO ledger_readonly';

        EXECUTE 'GRANT ALL ON ledger_subjects TO ledger_migrator';
        EXECUTE 'GRANT ALL ON transactions TO ledger_migrator';
        EXECUTE 'GRANT ALL ON corrections TO ledger_migrator';
        EXECUTE 'GRANT ALL ON transfer_pairs TO ledger_migrator';
        EXECUTE 'GRANT ALL ON cost_basis_lots TO ledger_migrator';
        EXECUTE 'GRANT ALL ON pnl_snapshots TO ledger_migrator';
        EXECUTE 'GRANT ALL ON anonymisation_requests TO ledger_migrator';
        EXECUTE 'GRANT ALL ON reconcile_leases TO ledger_migrator';
        EXECUTE 'GRANT ALL ON service_token_registry TO ledger_migrator';
    EXCEPTION WHEN insufficient_privilege OR undefined_object THEN
        RAISE NOTICE 'skipping ledger role grants: %', SQLERRM;
    END;
END
$$;
