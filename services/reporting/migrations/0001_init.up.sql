-- Reporting schema: briefs, their delivery attempts, the single-flight
-- generation lease, the service-token registry, and the privacy ledger.
--
-- Object names are unqualified on purpose: the migration runs with `search_path`
-- set to the service's own schema. No cross-schema grants, no Row Level Security.
--
-- Roles are created idempotently so a disposable test database and production
-- converge on the same shape:
--   reporting_svc       read/write the service's own records
--   reporting_readonly  select only, for operator tooling
--   reporting_migrator  DDL and the retention prune
--
-- The roles are NOLOGIN: a migration must not mint a login or a password. The
-- platform's Vault bootstrap owns LOGIN and the credential for each role.

-- ---------------------------------------------------------------------------
-- Roles
-- ---------------------------------------------------------------------------
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'reporting_svc') THEN
        EXECUTE 'CREATE ROLE reporting_svc NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'reporting_readonly') THEN
        EXECUTE 'CREATE ROLE reporting_readonly NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'reporting_migrator') THEN
        EXECUTE 'CREATE ROLE reporting_migrator NOLOGIN';
    END IF;
END
$$;

-- ---------------------------------------------------------------------------
-- Briefs (BR1.1–BR1.3, BR2.1)
-- ---------------------------------------------------------------------------
CREATE TABLE briefs (
    id             uuid        PRIMARY KEY,
    user_id        uuid        NOT NULL,
    period         text        NOT NULL CHECK (period IN ('daily', 'weekly')),
    period_start   timestamptz NOT NULL,
    period_end     timestamptz NOT NULL,
    status         text        NOT NULL CHECK (status IN ('pending', 'generated', 'failed')),
    failure_reason text        CHECK (failure_reason IS NULL OR char_length(failure_reason) BETWEEN 1 AND 500),
    as_of          timestamptz,
    data_status    text        CHECK (data_status IS NULL OR data_status IN ('complete', 'partial', 'unavailable')),
    sections       jsonb       CHECK (sections IS NULL OR jsonb_typeof(sections) = 'object'),
    created_at     timestamptz NOT NULL,
    updated_at     timestamptz NOT NULL,
    anonymised_at  timestamptz,
    CONSTRAINT briefs_window_shape CHECK (period_end > period_start),
    CONSTRAINT briefs_status_shape CHECK (
        (status = 'pending'
            AND failure_reason IS NULL AND as_of IS NULL AND data_status IS NULL AND sections IS NULL)
        OR (status = 'generated'
            AND failure_reason IS NULL AND as_of IS NOT NULL AND data_status IS NOT NULL
            AND sections IS NOT NULL)
        OR (status = 'failed'
            AND failure_reason IS NOT NULL AND as_of IS NULL AND data_status IS NULL
            AND sections IS NULL)
    ),
    -- BR1.1: one brief per user per period window.
    UNIQUE (user_id, period, period_start)
);

-- BR3.1: the list reads one user's briefs newest first.
CREATE INDEX briefs_user_updated_idx ON briefs (user_id, updated_at DESC);

-- ---------------------------------------------------------------------------
-- Delivery attempts (BR2.2–BR2.3)
-- ---------------------------------------------------------------------------
CREATE TABLE brief_deliveries (
    id           uuid        PRIMARY KEY,
    brief_id     uuid        NOT NULL REFERENCES briefs (id) ON DELETE CASCADE,
    channel      text        NOT NULL CHECK (channel IN ('in_app', 'email')),
    alert_id     text        CHECK (alert_id IS NULL OR char_length(alert_id) BETWEEN 1 AND 200),
    status       text        NOT NULL CHECK (status IN ('submitted', 'suppressed', 'failed')),
    submitted_at timestamptz NOT NULL,
    CONSTRAINT brief_deliveries_alert_shape CHECK (
        (status = 'submitted' AND alert_id IS NOT NULL)
        OR (status IN ('suppressed', 'failed') AND alert_id IS NULL)
    ),
    -- A retry on the same channel replaces the earlier attempt.
    UNIQUE (brief_id, channel)
);

CREATE INDEX brief_deliveries_brief_idx ON brief_deliveries (brief_id, submitted_at);

-- ---------------------------------------------------------------------------
-- Single-flight generation lease (NFR1.4)
-- ---------------------------------------------------------------------------
CREATE TABLE brief_runs (
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

CREATE UNIQUE INDEX brief_runs_one_live
    ON brief_runs (job) WHERE status = 'running';
CREATE INDEX brief_runs_running_idx
    ON brief_runs (lease_until) WHERE status = 'running';

-- ---------------------------------------------------------------------------
-- Internal service-token registry (token hash -> caller -> allow-list)
-- ---------------------------------------------------------------------------
CREATE TABLE service_token_registry (
    id         uuid        PRIMARY KEY,
    token_hash char(64)    NOT NULL UNIQUE CHECK (token_hash ~ '^[0-9a-f]{64}$'),
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
    schema_name text := coalesce(current_schema(), 'reporting');
BEGIN
    EXECUTE format('GRANT USAGE ON SCHEMA %I TO reporting_svc, reporting_readonly, reporting_migrator', schema_name);
    EXECUTE format('GRANT CREATE ON SCHEMA %I TO reporting_migrator', schema_name);

    EXECUTE 'GRANT SELECT, INSERT ON briefs TO reporting_svc';
    -- Column-scoped UPDATE: generation and failure replace the brief's own
    -- fields, and erasure replaces only the subject column.
    EXECUTE 'GRANT UPDATE (status, failure_reason, as_of, data_status, sections, updated_at, user_id, anonymised_at) ON briefs TO reporting_svc';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON brief_deliveries TO reporting_svc';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON brief_runs TO reporting_svc';
    EXECUTE 'GRANT SELECT ON service_token_registry TO reporting_svc';
    EXECUTE 'GRANT SELECT, INSERT ON privacy_anonymisations TO reporting_svc';

    IF to_regclass('_sqlx_migrations') IS NOT NULL THEN
        EXECUTE 'GRANT SELECT ON _sqlx_migrations TO reporting_svc';
    END IF;

    EXECUTE 'GRANT SELECT ON briefs TO reporting_readonly';
    EXECUTE 'GRANT SELECT ON brief_deliveries TO reporting_readonly';
    EXECUTE 'GRANT SELECT ON brief_runs TO reporting_readonly';
    EXECUTE 'GRANT SELECT ON service_token_registry TO reporting_readonly';
    EXECUTE 'GRANT SELECT ON privacy_anonymisations TO reporting_readonly';

    EXECUTE 'GRANT ALL ON briefs TO reporting_migrator';
    EXECUTE 'GRANT ALL ON brief_deliveries TO reporting_migrator';
    EXECUTE 'GRANT ALL ON brief_runs TO reporting_migrator';
    EXECUTE 'GRANT ALL ON service_token_registry TO reporting_migrator';
    EXECUTE 'GRANT ALL ON privacy_anonymisations TO reporting_migrator';
END
$$;
