-- Analytics schema: consented PII-free events, consent-check outcomes, aggregate
-- figures, governed feature flags, the service-token registry, and the salt
-- version catalogue.
--
-- Object names are unqualified on purpose: the migration runs with `search_path`
-- set to the service's own schema, and adapter tests point the same migration at
-- a disposable schema. No cross-schema grants and no Row Level Security.
--
-- Roles are created idempotently so a disposable database and production
-- converge on the same shape:
--   analytics_svc       insert/select/delete events and consent checks;
--                       select/insert/update aggregates and flags
--   analytics_readonly  select only
--   analytics_migrator  DDL
--
-- The salt itself lives in Supabase Vault. This schema stores only version
-- labels, and the SECURITY DEFINER function below reads the secret for the
-- service without a cross-schema grant.

-- ---------------------------------------------------------------------------
-- Roles
-- ---------------------------------------------------------------------------
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'analytics_svc') THEN
        EXECUTE 'CREATE ROLE analytics_svc NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'analytics_readonly') THEN
        EXECUTE 'CREATE ROLE analytics_readonly NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'analytics_migrator') THEN
        EXECUTE 'CREATE ROLE analytics_migrator NOLOGIN';
    END IF;
END
$$;

-- The roles are created NOLOGIN on purpose: a migration must not mint a login or
-- a password. The platform's Supabase Vault bootstrap owns the LOGIN attribute
-- and the credential for each role.

-- ---------------------------------------------------------------------------
-- Events
-- ---------------------------------------------------------------------------
CREATE TABLE events (
    id              uuid        PRIMARY KEY,
    event_name      text        NOT NULL
        CHECK (char_length(event_name) BETWEEN 1 AND 64),
    event_class     text        NOT NULL CHECK (event_class IN ('analytics', 'essential')),
    occurred_at     timestamptz NOT NULL,
    properties      jsonb       NOT NULL DEFAULT '{}'::jsonb
        CHECK (jsonb_typeof(properties) = 'object'),
    user_ref        text        CHECK (user_ref IS NULL OR char_length(user_ref) BETWEEN 32 AND 128),
    consent_purpose text        NOT NULL CHECK (consent_purpose IN ('essential', 'analytics')),
    ingested_at     timestamptz NOT NULL,
    -- The class and the purpose can never disagree.
    CONSTRAINT events_class_purpose CHECK (
        (event_class = 'analytics' AND consent_purpose = 'analytics')
        OR (event_class = 'essential' AND consent_purpose = 'essential')
    )
);

CREATE INDEX events_event_name_occurred_idx ON events (event_name, occurred_at);
CREATE INDEX events_user_ref_idx ON events (user_ref);
CREATE INDEX events_occurred_idx ON events (occurred_at);

-- ---------------------------------------------------------------------------
-- Consent-check outcomes
-- ---------------------------------------------------------------------------
CREATE TABLE consent_checks (
    id          uuid        PRIMARY KEY,
    purpose     text        NOT NULL CHECK (purpose IN ('essential', 'analytics')),
    granted     boolean     NOT NULL,
    checked_at  timestamptz NOT NULL,
    user_ref    text        CHECK (user_ref IS NULL OR char_length(user_ref) BETWEEN 32 AND 128)
);

CREATE INDEX consent_checks_checked_idx ON consent_checks (checked_at);

-- ---------------------------------------------------------------------------
-- Aggregate figures
-- ---------------------------------------------------------------------------
CREATE TABLE metric_aggregates (
    id            uuid          PRIMARY KEY,
    metric        text          NOT NULL
        CHECK (metric IN (
            'activation', 'engagement', 'retention_d7', 'retention_d30', 'conversion', 'quality'
        )),
    period        text          NOT NULL CHECK (period IN ('daily', 'weekly')),
    period_start  timestamptz   NOT NULL,
    cohort        text          NOT NULL CHECK (char_length(cohort) BETWEEN 1 AND 64),
    value         numeric(6, 4) NOT NULL CHECK (value >= 0 AND value <= 1),
    computed_at   timestamptz   NOT NULL,
    CONSTRAINT metric_aggregates_key UNIQUE (metric, period, cohort, period_start)
);

CREATE INDEX metric_aggregates_read_idx ON metric_aggregates (metric, period, period_start);

-- ---------------------------------------------------------------------------
-- Feature flags
-- ---------------------------------------------------------------------------
CREATE TABLE feature_flags (
    id                 uuid        PRIMARY KEY,
    key                text        NOT NULL UNIQUE
        CHECK (char_length(key) BETWEEN 1 AND 80),
    owner              text        NOT NULL CHECK (char_length(owner) BETWEEN 1 AND 120),
    expires_at         timestamptz NOT NULL,
    rollout_percentage smallint    NOT NULL CHECK (rollout_percentage BETWEEN 0 AND 100),
    status             text        NOT NULL CHECK (status IN ('active', 'disabled', 'expired')),
    created_at         timestamptz NOT NULL,
    updated_at         timestamptz NOT NULL
);

CREATE INDEX feature_flags_expires_idx ON feature_flags (expires_at);

-- ---------------------------------------------------------------------------
-- Service-token registry and salt versions
-- ---------------------------------------------------------------------------
-- Only hashes are stored. The deploy job holds write rights for rotation
-- (insert-then-delete); the runtime role reads only.
CREATE TABLE service_token_registry (
    service            text        PRIMARY KEY,
    token_sha256       text        NOT NULL UNIQUE CHECK (char_length(token_sha256) = 64),
    allowed_operations text[]      NOT NULL,
    rotated_at         timestamptz NOT NULL DEFAULT now()
);

-- Version labels only. The salt bytes live in Vault under
-- `analytics/salt/<version>` and are never stored in this schema.
CREATE TABLE salt_versions (
    version    text        PRIMARY KEY,
    created_at timestamptz NOT NULL DEFAULT now()
);

-- Reads one salt from Vault. Dynamic SQL defers relation resolution, so the
-- function can be created in a database that has no Vault schema (disposable
-- tests) and fails loudly only when a salt is actually requested.
CREATE OR REPLACE FUNCTION read_analytics_salt(version text)
RETURNS text
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path FROM CURRENT
AS $$
DECLARE
    secret text;
BEGIN
    IF version IS NULL OR btrim(version) = '' THEN
        RAISE EXCEPTION 'a salt version is required' USING ERRCODE = '22023';
    END IF;
    EXECUTE
        'SELECT decrypted_secret FROM vault.decrypted_secrets WHERE name = $1 LIMIT 1'
        INTO secret
        USING 'analytics/salt/' || version;
    IF secret IS NULL OR btrim(secret) = '' THEN
        RAISE EXCEPTION 'salt version % is not present in Vault', version
            USING ERRCODE = '55000';
    END IF;
    RETURN secret;
END;
$$;

-- ---------------------------------------------------------------------------
-- Grants
-- ---------------------------------------------------------------------------
DO $$
DECLARE
    schema_name text := coalesce(current_schema(), 'analytics');
BEGIN
    EXECUTE format('GRANT USAGE ON SCHEMA %I TO analytics_svc, analytics_readonly, analytics_migrator', schema_name);
    EXECUTE format('GRANT CREATE ON SCHEMA %I TO analytics_migrator', schema_name);

    EXECUTE 'GRANT SELECT, INSERT, DELETE ON events TO analytics_svc';
    EXECUTE 'GRANT SELECT, INSERT, DELETE ON consent_checks TO analytics_svc';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON metric_aggregates TO analytics_svc';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON feature_flags TO analytics_svc';
    EXECUTE 'GRANT SELECT ON service_token_registry TO analytics_svc';
    EXECUTE 'GRANT SELECT ON salt_versions TO analytics_svc';
    EXECUTE 'GRANT EXECUTE ON FUNCTION read_analytics_salt(text) TO analytics_svc';

    IF to_regclass('_sqlx_migrations') IS NOT NULL THEN
        EXECUTE 'GRANT SELECT ON _sqlx_migrations TO analytics_svc';
    END IF;

    EXECUTE 'GRANT SELECT ON events TO analytics_readonly';
    EXECUTE 'GRANT SELECT ON consent_checks TO analytics_readonly';
    EXECUTE 'GRANT SELECT ON metric_aggregates TO analytics_readonly';
    EXECUTE 'GRANT SELECT ON feature_flags TO analytics_readonly';
    EXECUTE 'GRANT SELECT ON service_token_registry TO analytics_readonly';
    EXECUTE 'GRANT SELECT ON salt_versions TO analytics_readonly';

    EXECUTE 'GRANT ALL ON events TO analytics_migrator';
    EXECUTE 'GRANT ALL ON consent_checks TO analytics_migrator';
    EXECUTE 'GRANT ALL ON metric_aggregates TO analytics_migrator';
    EXECUTE 'GRANT ALL ON feature_flags TO analytics_migrator';
    EXECUTE 'GRANT ALL ON service_token_registry TO analytics_migrator';
    EXECUTE 'GRANT ALL ON salt_versions TO analytics_migrator';
    EXECUTE 'GRANT EXECUTE ON FUNCTION read_analytics_salt(text) TO analytics_migrator';
END
$$;
