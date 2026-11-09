-- Account schema: read-only holding sources, credentials as Vault references,
-- wallet connections, sync runs with a lease, per-sync snapshots, CSV imports,
-- the staged upload buffer, the service-token registry, and the privacy ledger.
--
-- Object names are unqualified on purpose: the migration runs with `search_path`
-- set to the service's own schema, and adapter tests point the same migration at a
-- disposable schema. No cross-schema grants, no Row Level Security.
--
-- Roles are created idempotently so a disposable test database and production
-- converge on the same shape:
--   account_svc       read/write the service's own records
--   account_readonly  select only, for operator tooling
--   account_migrator  DDL
--
-- The roles are NOLOGIN: a migration must not mint a login or a password. The
-- platform's Vault bootstrap owns LOGIN and the credential for each role.

-- ---------------------------------------------------------------------------
-- Roles
-- ---------------------------------------------------------------------------
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'account_svc') THEN
        EXECUTE 'CREATE ROLE account_svc NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'account_readonly') THEN
        EXECUTE 'CREATE ROLE account_readonly NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'account_migrator') THEN
        EXECUTE 'CREATE ROLE account_migrator NOLOGIN';
    END IF;
END
$$;

-- ---------------------------------------------------------------------------
-- Accounts
-- ---------------------------------------------------------------------------
CREATE TABLE accounts (
    id             uuid        PRIMARY KEY,
    user_id        uuid        NOT NULL,
    kind           text        NOT NULL CHECK (kind IN ('exchange', 'wallet', 'walletconnect', 'csv')),
    venue          text        NOT NULL CHECK (char_length(venue) BETWEEN 1 AND 80),
    label          text        NOT NULL CHECK (char_length(label) BETWEEN 1 AND 64),
    status         text        NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'syncing', 'error', 'disconnected')),
    last_synced_at timestamptz,
    created_at     timestamptz NOT NULL,
    updated_at     timestamptz NOT NULL,
    anonymised_at  timestamptz
);

-- BR1.2: labels are unique per user ignoring case. An anonymised account leaves
-- the unique set so several anonymised rows can coexist.
CREATE UNIQUE INDEX accounts_label_unique
    ON accounts (user_id, lower(label)) WHERE anonymised_at IS NULL;
CREATE INDEX accounts_user_idx ON accounts (user_id, created_at);
CREATE INDEX accounts_syncable_idx
    ON accounts (status, kind) WHERE anonymised_at IS NULL;

-- ---------------------------------------------------------------------------
-- Credentials (Vault references only; never key material)
-- ---------------------------------------------------------------------------
CREATE TABLE credentials (
    id                     uuid        PRIMARY KEY,
    account_id             uuid        NOT NULL UNIQUE REFERENCES accounts (id) ON DELETE CASCADE,
    vault_secret_ref       text        NOT NULL CHECK (char_length(vault_secret_ref) BETWEEN 1 AND 240),
    read_only              boolean     NOT NULL DEFAULT false,
    verification_status    text        NOT NULL DEFAULT 'pending'
        CHECK (verification_status IN ('pending', 'verified', 'rejected', 'stale')),
    permissions_verified_at timestamptz,
    created_at             timestamptz NOT NULL
);

CREATE INDEX credentials_status_idx ON credentials (verification_status);

-- ---------------------------------------------------------------------------
-- Wallet connections
-- ---------------------------------------------------------------------------
CREATE TABLE wallet_connections (
    id             uuid        PRIMARY KEY,
    account_id     uuid        NOT NULL UNIQUE REFERENCES accounts (id) ON DELETE CASCADE,
    address        text        NOT NULL CHECK (char_length(address) BETWEEN 1 AND 200),
    observed_chain text        NOT NULL CHECK (char_length(observed_chain) BETWEEN 1 AND 80),
    session_ref    text        CHECK (session_ref IS NULL OR char_length(session_ref) BETWEEN 1 AND 200),
    connected_at   timestamptz NOT NULL
);

-- ---------------------------------------------------------------------------
-- Sync runs and the single-flight lease
-- ---------------------------------------------------------------------------
CREATE TABLE sync_runs (
    id             uuid        PRIMARY KEY,
    account_id     uuid        NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    started_at     timestamptz NOT NULL,
    lease_until    timestamptz NOT NULL,
    finished_at    timestamptz,
    status         text        NOT NULL DEFAULT 'running'
        CHECK (status IN ('running', 'succeeded', 'failed')),
    error_category text        CHECK (error_category IS NULL OR error_category IN
        ('auth', 'rate_limit', 'provider', 'network', 'data')),
    error_message  text        CHECK (error_message IS NULL OR char_length(error_message) <= 500),
    retries        integer     NOT NULL DEFAULT 0 CHECK (retries >= 0),
    CONSTRAINT sync_runs_failed_shape CHECK (
        status <> 'failed' OR error_category IS NOT NULL
    )
);

-- BR4.1: at most one running run per account. The claim path expires an overdue
-- run before inserting, so an expired lease never blocks a later run.
CREATE UNIQUE INDEX sync_runs_one_live
    ON sync_runs (account_id) WHERE status = 'running';
CREATE INDEX sync_runs_account_idx ON sync_runs (account_id, started_at DESC);
CREATE INDEX sync_runs_running_idx ON sync_runs (lease_until) WHERE status = 'running';

-- ---------------------------------------------------------------------------
-- Holding snapshots
-- ---------------------------------------------------------------------------
CREATE TABLE holding_snapshots (
    id             uuid           PRIMARY KEY,
    sync_run_id    uuid           NOT NULL REFERENCES sync_runs (id) ON DELETE CASCADE,
    account_id     uuid           NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    observed_token text           NOT NULL CHECK (char_length(observed_token) BETWEEN 1 AND 200),
    asset_id       text           CHECK (asset_id IS NULL OR char_length(asset_id) BETWEEN 1 AND 200),
    quantity       numeric(38,18) NOT NULL CHECK (quantity >= 0),
    observed_at    timestamptz    NOT NULL,
    spam           boolean        NOT NULL DEFAULT false,
    CONSTRAINT holding_snapshots_spam_shape CHECK (
        (spam AND asset_id IS NULL) OR (NOT spam AND asset_id IS NOT NULL)
    )
);

CREATE INDEX holding_snapshots_account_idx
    ON holding_snapshots (account_id, observed_at DESC);

-- ---------------------------------------------------------------------------
-- CSV imports and the staging buffer
-- ---------------------------------------------------------------------------
CREATE TABLE import_batches (
    id             uuid        PRIMARY KEY,
    account_id     uuid        NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    status         text        NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'processing', 'complete', 'failed')),
    source         text        NOT NULL CHECK (source IN ('binance', 'okx', 'generic')),
    file_name      text        NOT NULL CHECK (char_length(file_name) BETWEEN 1 AND 255),
    imported_count integer     NOT NULL DEFAULT 0 CHECK (imported_count >= 0),
    skipped_count  integer     NOT NULL DEFAULT 0 CHECK (skipped_count >= 0),
    failed_count   integer     NOT NULL DEFAULT 0 CHECK (failed_count >= 0),
    created_at     timestamptz NOT NULL,
    completed_at   timestamptz
);

CREATE INDEX import_batches_pending_idx ON import_batches (status, created_at);

CREATE TABLE import_upload (
    batch_id   uuid        PRIMARY KEY REFERENCES import_batches (id) ON DELETE CASCADE,
    bytes      bytea       NOT NULL,
    byte_size  integer     NOT NULL CHECK (byte_size > 0 AND byte_size <= 10485760),
    created_at timestamptz NOT NULL,
    CONSTRAINT import_upload_size_matches CHECK (octet_length(bytes) = byte_size)
);

CREATE TABLE imported_transactions (
    id           uuid           PRIMARY KEY,
    batch_id     uuid           NOT NULL REFERENCES import_batches (id) ON DELETE CASCADE,
    account_id   uuid           NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    external_id  text           CHECK (external_id IS NULL OR char_length(external_id) BETWEEN 1 AND 200),
    dedupe_key   text           NOT NULL CHECK (char_length(dedupe_key) BETWEEN 1 AND 400),
    asset_id     text           NOT NULL CHECK (char_length(asset_id) BETWEEN 1 AND 200),
    kind         text           NOT NULL CHECK (kind IN ('buy', 'sell', 'transfer_in', 'transfer_out', 'fee')),
    quantity     numeric(38,18) NOT NULL CHECK (quantity >= 0),
    price        numeric(38,18) CHECK (price IS NULL OR price >= 0),
    executed_at  timestamptz    NOT NULL
);

-- BR5.2: a duplicate row is never stored twice.
CREATE UNIQUE INDEX imported_transactions_dedupe
    ON imported_transactions (account_id, dedupe_key);
CREATE INDEX imported_transactions_account_idx
    ON imported_transactions (account_id, executed_at);

CREATE TABLE import_row_errors (
    id          uuid    PRIMARY KEY,
    batch_id    uuid    NOT NULL REFERENCES import_batches (id) ON DELETE CASCADE,
    line_number integer NOT NULL CHECK (line_number >= 1),
    reason      text    NOT NULL CHECK (char_length(reason) BETWEEN 1 AND 300)
);

CREATE INDEX import_row_errors_batch_idx ON import_row_errors (batch_id, line_number);

-- ---------------------------------------------------------------------------
-- Internal service-token registry (token hash -> caller -> allow-list)
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
-- Privacy anonymisation ledger (a retry applies once)
-- ---------------------------------------------------------------------------
CREATE TABLE privacy_anonymisations (
    user_id             uuid        PRIMARY KEY,
    idempotency_key     text        NOT NULL CHECK (char_length(idempotency_key) BETWEEN 1 AND 200),
    retained_categories jsonb       NOT NULL DEFAULT '[]'::jsonb
        CHECK (jsonb_typeof(retained_categories) = 'array'),
    completed_at        timestamptz NOT NULL
);

-- ---------------------------------------------------------------------------
-- Grants
-- ---------------------------------------------------------------------------
DO $$
DECLARE
    schema_name text := coalesce(current_schema(), 'account');
BEGIN
    EXECUTE format('GRANT USAGE ON SCHEMA %I TO account_svc, account_readonly, account_migrator', schema_name);
    EXECUTE format('GRANT CREATE ON SCHEMA %I TO account_migrator', schema_name);

    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON accounts TO account_svc';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON credentials TO account_svc';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON wallet_connections TO account_svc';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON sync_runs TO account_svc';
    EXECUTE 'GRANT SELECT, INSERT ON holding_snapshots TO account_svc';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON import_batches TO account_svc';
    EXECUTE 'GRANT SELECT, INSERT, DELETE ON import_upload TO account_svc';
    EXECUTE 'GRANT SELECT, INSERT ON imported_transactions TO account_svc';
    EXECUTE 'GRANT SELECT, INSERT ON import_row_errors TO account_svc';
    EXECUTE 'GRANT SELECT ON service_token_registry TO account_svc';
    EXECUTE 'GRANT SELECT, INSERT ON privacy_anonymisations TO account_svc';

    IF to_regclass('_sqlx_migrations') IS NOT NULL THEN
        EXECUTE 'GRANT SELECT ON _sqlx_migrations TO account_svc';
    END IF;

    EXECUTE 'GRANT SELECT ON accounts TO account_readonly';
    EXECUTE 'GRANT SELECT ON credentials TO account_readonly';
    EXECUTE 'GRANT SELECT ON wallet_connections TO account_readonly';
    EXECUTE 'GRANT SELECT ON sync_runs TO account_readonly';
    EXECUTE 'GRANT SELECT ON holding_snapshots TO account_readonly';
    EXECUTE 'GRANT SELECT ON import_batches TO account_readonly';
    EXECUTE 'GRANT SELECT ON imported_transactions TO account_readonly';
    EXECUTE 'GRANT SELECT ON import_row_errors TO account_readonly';
    EXECUTE 'GRANT SELECT ON service_token_registry TO account_readonly';
    EXECUTE 'GRANT SELECT ON privacy_anonymisations TO account_readonly';

    EXECUTE 'GRANT ALL ON accounts TO account_migrator';
    EXECUTE 'GRANT ALL ON credentials TO account_migrator';
    EXECUTE 'GRANT ALL ON wallet_connections TO account_migrator';
    EXECUTE 'GRANT ALL ON sync_runs TO account_migrator';
    EXECUTE 'GRANT ALL ON holding_snapshots TO account_migrator';
    EXECUTE 'GRANT ALL ON import_batches TO account_migrator';
    EXECUTE 'GRANT ALL ON import_upload TO account_migrator';
    EXECUTE 'GRANT ALL ON imported_transactions TO account_migrator';
    EXECUTE 'GRANT ALL ON import_row_errors TO account_migrator';
    EXECUTE 'GRANT ALL ON service_token_registry TO account_migrator';
    EXECUTE 'GRANT ALL ON privacy_anonymisations TO account_migrator';
END
$$;
