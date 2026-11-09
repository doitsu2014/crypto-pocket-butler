-- Identity schema: internal users, encrypted personal subjects, append-only
-- consent, support grants, device sessions, export and deletion requests,
-- jurisdiction rules, and per-subject shredding-key references.
--
-- Object names are unqualified on purpose: the migration runs with `search_path`
-- set to the service's own schema, and adapter tests point the same migration at
-- a disposable schema. No cross-schema grants, no Row Level Security.
--
-- Roles are created idempotently so a disposable test database and production
-- converge on the same shape:
--   identity_app       read/write the service's own records; execute protect/unprotect
--   identity_readonly  select only, for operator tooling
--   identity_migrator  DDL
--
-- The roles are NOLOGIN: a migration must not mint a login or a password. The
-- platform's Vault bootstrap owns LOGIN and the credential for each role.

-- ---------------------------------------------------------------------------
-- Roles
-- ---------------------------------------------------------------------------
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'identity_app') THEN
        EXECUTE 'CREATE ROLE identity_app NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'identity_readonly') THEN
        EXECUTE 'CREATE ROLE identity_readonly NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'identity_migrator') THEN
        EXECUTE 'CREATE ROLE identity_migrator NOLOGIN';
    END IF;
END
$$;

-- pgsodium backs the per-subject protect/unprotect functions. A disposable
-- database without the extension still applies this migration: the functions are
-- plpgsql, so the missing symbols are resolved at call time and refuse clearly.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_available_extensions WHERE name = 'pgsodium') THEN
        EXECUTE 'CREATE EXTENSION IF NOT EXISTS pgsodium';
    END IF;
END
$$;

-- ---------------------------------------------------------------------------
-- Key references and users
-- ---------------------------------------------------------------------------
CREATE TABLE shredding_keys (
    id              uuid        PRIMARY KEY,
    user_id         uuid        NOT NULL UNIQUE,
    vault_reference text        NOT NULL UNIQUE CHECK (char_length(vault_reference) BETWEEN 1 AND 240),
    key_id          bigint      NOT NULL UNIQUE,
    status          text        NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'destroyed')),
    created_at      timestamptz NOT NULL,
    destroyed_at    timestamptz,
    CONSTRAINT shredding_keys_destroyed_shape CHECK (
        (status = 'active' AND destroyed_at IS NULL)
        OR (status = 'destroyed' AND destroyed_at IS NOT NULL)
    )
);

CREATE TABLE users (
    id              uuid        PRIMARY KEY,
    role            text        NOT NULL DEFAULT 'user' CHECK (role IN ('user', 'admin', 'support')),
    status          text        NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'active', 'deleted')),
    locale          text        NOT NULL DEFAULT 'en' CHECK (locale IN ('en', 'vi')),
    base_currency   char(3)     NOT NULL DEFAULT 'USD' CHECK (base_currency ~ '^[A-Z]{3}$'),
    timezone        text        NOT NULL DEFAULT 'UTC' CHECK (char_length(timezone) BETWEEN 1 AND 64),
    created_at      timestamptz NOT NULL,
    updated_at      timestamptz NOT NULL,
    anonymised_at   timestamptz,
    CONSTRAINT users_deleted_shape CHECK (
        (status = 'deleted' AND anonymised_at IS NOT NULL)
        OR (status <> 'deleted' AND anonymised_at IS NULL)
    )
);

-- The Supabase subject is personal data. Only its hash (for the indexed lookup)
-- and its deterministic ciphertext (for the Auth admin delete) are stored; the
-- ciphertext is unreadable once the per-subject key is destroyed.
CREATE TABLE user_personal (
    user_id                uuid        PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    auth_subject_hash      text        NOT NULL UNIQUE CHECK (char_length(auth_subject_hash) = 64),
    auth_subject_protected bytea       NOT NULL,
    shredding_key_id       uuid        NOT NULL REFERENCES shredding_keys (id)
);

-- ---------------------------------------------------------------------------
-- Consent history (append-only)
-- ---------------------------------------------------------------------------
CREATE TABLE consent_records (
    id          uuid        PRIMARY KEY,
    user_id     uuid        NOT NULL REFERENCES users (id),
    purpose     text        NOT NULL CHECK (purpose IN ('analytics', 'marketing', 'disclosures')),
    granted     boolean     NOT NULL,
    recorded_at timestamptz NOT NULL,
    source      text        NOT NULL DEFAULT 'onboarding' CHECK (source IN ('onboarding', 'settings', 'support'))
);

CREATE INDEX consent_records_latest_idx
    ON consent_records (user_id, purpose, recorded_at DESC, id DESC);

-- ---------------------------------------------------------------------------
-- Support access grants
-- ---------------------------------------------------------------------------
CREATE TABLE support_access_grants (
    id               uuid        PRIMARY KEY,
    user_id          uuid        NOT NULL REFERENCES users (id),
    support_agent_id uuid        NOT NULL,
    reason           text        NOT NULL CHECK (char_length(reason) BETWEEN 1 AND 500),
    status           text        NOT NULL DEFAULT 'pending_approval'
        CHECK (status IN ('pending_approval', 'active', 'expired', 'revoked')),
    requested_at     timestamptz NOT NULL,
    approved_by      uuid,
    approved_at      timestamptz,
    expires_at       timestamptz,
    revoked_at       timestamptz,
    CONSTRAINT support_grants_approval_shape CHECK (
        (status = 'pending_approval' AND approved_by IS NULL AND approved_at IS NULL AND expires_at IS NULL)
        OR (status <> 'pending_approval' AND approved_at IS NOT NULL)
        OR (status = 'revoked' AND approved_at IS NULL)
    ),
    CONSTRAINT support_grants_distinct_approver CHECK (
        approved_by IS NULL OR approved_by <> support_agent_id
    ),
    CONSTRAINT support_grants_window CHECK (
        expires_at IS NULL
        OR (expires_at > approved_at AND expires_at <= approved_at + interval '60 minutes')
    )
);

CREATE INDEX support_grants_read_idx ON support_access_grants (user_id, status, expires_at DESC);

-- ---------------------------------------------------------------------------
-- Device sessions (Identity's own view; no token, factor, or full IP)
-- ---------------------------------------------------------------------------
CREATE TABLE device_sessions (
    id               uuid        PRIMARY KEY,
    user_id          uuid        NOT NULL REFERENCES users (id),
    auth_session_id  text        NOT NULL CHECK (char_length(auth_session_id) BETWEEN 1 AND 200),
    device           text        CHECK (device IS NULL OR char_length(device) <= 120),
    location         text        CHECK (location IS NULL OR char_length(location) <= 120),
    last_activity_at timestamptz NOT NULL,
    revoked_at       timestamptz,
    UNIQUE (user_id, auth_session_id)
);

CREATE INDEX device_sessions_activity_idx ON device_sessions (user_id, last_activity_at DESC);

-- ---------------------------------------------------------------------------
-- Export and deletion requests
-- ---------------------------------------------------------------------------
CREATE TABLE export_requests (
    id                   uuid        PRIMARY KEY,
    user_id              uuid        NOT NULL REFERENCES users (id),
    status               text        NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'building', 'ready', 'failed', 'expired')),
    progress             smallint    NOT NULL DEFAULT 0 CHECK (progress BETWEEN 0 AND 100),
    contributions        jsonb       NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(contributions) = 'object'),
    requested_at         timestamptz NOT NULL,
    assembly_deadline    timestamptz NOT NULL,
    file_reference       text        CHECK (file_reference IS NULL OR char_length(file_reference) <= 300),
    download_expires_at  timestamptz,
    failure_reason       text        CHECK (failure_reason IS NULL OR char_length(failure_reason) <= 500),
    completed_at         timestamptz,
    lease_owner          text        CHECK (lease_owner IS NULL OR char_length(lease_owner) <= 120),
    lease_until          timestamptz,
    CONSTRAINT export_requests_ready_shape CHECK (
        status <> 'ready'
        OR (file_reference IS NOT NULL AND download_expires_at IS NOT NULL AND completed_at IS NOT NULL)
    ),
    CONSTRAINT export_requests_lease_shape CHECK (
        (lease_owner IS NULL AND lease_until IS NULL)
        OR (lease_owner IS NOT NULL AND lease_until IS NOT NULL)
    )
);

CREATE INDEX export_requests_claim_idx ON export_requests (status, lease_until);
CREATE UNIQUE INDEX export_requests_one_active_per_user
    ON export_requests (user_id) WHERE status IN ('pending', 'building');

CREATE TABLE deletion_requests (
    id                  uuid        PRIMARY KEY,
    user_id             uuid        NOT NULL REFERENCES users (id),
    status              text        NOT NULL DEFAULT 'requested'
        CHECK (status IN ('requested', 'in_progress', 'completed', 'failed')),
    progress            jsonb       NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(progress) = 'object'),
    retained_categories jsonb       NOT NULL DEFAULT '[]'::jsonb CHECK (jsonb_typeof(retained_categories) = 'array'),
    requested_at        timestamptz NOT NULL,
    completed_at        timestamptz,
    failure_reason      text        CHECK (failure_reason IS NULL OR char_length(failure_reason) <= 500),
    lease_owner         text        CHECK (lease_owner IS NULL OR char_length(lease_owner) <= 120),
    lease_until         timestamptz,
    CONSTRAINT deletion_requests_completed_shape CHECK (
        (status = 'completed' AND completed_at IS NOT NULL)
        OR (status <> 'completed')
    ),
    CONSTRAINT deletion_requests_lease_shape CHECK (
        (lease_owner IS NULL AND lease_until IS NULL)
        OR (lease_owner IS NOT NULL AND lease_until IS NOT NULL)
    )
);

CREATE INDEX deletion_requests_claim_idx ON deletion_requests (status, lease_until);
CREATE UNIQUE INDEX deletion_requests_one_active_per_user
    ON deletion_requests (user_id) WHERE status IN ('requested', 'in_progress');

-- ---------------------------------------------------------------------------
-- Jurisdiction rules (Compliance seeds and maintains this table)
-- ---------------------------------------------------------------------------
CREATE TABLE jurisdiction_rules (
    code                 char(2)     PRIMARY KEY CHECK (code ~ '^[A-Z]{2}$'),
    allowed              boolean     NOT NULL,
    minimum_age          integer     NOT NULL CHECK (minimum_age BETWEEN 0 AND 130),
    disclosures_required boolean     NOT NULL DEFAULT true,
    updated_at           timestamptz NOT NULL
);

-- Launch scope: Vietnam, the United Kingdom, and every EU member state. Every
-- seeded market allows sign-up at 18 with the launch disclosures. Any country not
-- listed is blocked until Compliance adds it.
INSERT INTO jurisdiction_rules (code, allowed, minimum_age, disclosures_required, updated_at)
SELECT code, true, 18, true, now()
FROM unnest(ARRAY[
    'VN', 'GB', 'IE', 'DE', 'FR', 'NL', 'ES', 'IT', 'PT', 'BE', 'AT', 'DK', 'SE',
    'FI', 'PL', 'CZ', 'SK', 'SI', 'HR', 'RO', 'BG', 'EE', 'LV', 'LT', 'LU', 'MT',
    'CY', 'GR', 'HU'
]) AS code
ON CONFLICT (code) DO NOTHING;

-- ---------------------------------------------------------------------------
-- pgsodium protect/unprotect and key lifecycle
-- ---------------------------------------------------------------------------
-- The service passes the key id only; it never reads key material. The
-- additional data binds a ciphertext to this column family, so a ciphertext
-- copied into another column does not decrypt.
CREATE OR REPLACE FUNCTION protect(plaintext text, key_id bigint)
RETURNS bytea
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pgsodium
AS $$
DECLARE
    ciphertext bytea;
BEGIN
    SELECT pgsodium.crypto_aead_det_encrypt(
        convert_to(plaintext, 'utf8'),
        convert_to('identity.user_personal.auth_subject', 'utf8'),
        key_id
    ) INTO ciphertext;
    RETURN ciphertext;
END;
$$;

CREATE OR REPLACE FUNCTION unprotect(ciphertext bytea, key_id bigint)
RETURNS text
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pgsodium
AS $$
DECLARE
    plaintext bytea;
BEGIN
    SELECT pgsodium.crypto_aead_det_decrypt(
        ciphertext,
        convert_to('identity.user_personal.auth_subject', 'utf8'),
        key_id
    ) INTO plaintext;
    RETURN convert_from(plaintext, 'utf8');
END;
$$;

-- Creates one pgsodium key and returns its id. The Vault reference is stored in
-- shredding_keys by the service; only the id crosses into the function.
CREATE OR REPLACE FUNCTION create_shredding_key(owner text)
RETURNS bigint
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pgsodium
AS $$
DECLARE
    created_key_id bigint;
BEGIN
    SELECT key_id INTO created_key_id FROM pgsodium.create_key(name => owner);
    RETURN created_key_id;
END;
$$;

-- Destroys one pgsodium key. This is the crypto-shredding step; it is the last
-- destructive action a deletion performs.
CREATE OR REPLACE FUNCTION destroy_shredding_key(target_key_id bigint)
RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pgsodium
AS $$
BEGIN
    DELETE FROM pgsodium.key WHERE key_id = target_key_id;
END;
$$;

-- ---------------------------------------------------------------------------
-- Grants
-- ---------------------------------------------------------------------------
DO $$
DECLARE
    schema_name text := coalesce(current_schema(), 'identity');
BEGIN
    EXECUTE format('GRANT USAGE ON SCHEMA %I TO identity_app, identity_readonly, identity_migrator', schema_name);
    EXECUTE format('GRANT CREATE ON SCHEMA %I TO identity_migrator', schema_name);

    -- identity_app: the service's own records.
    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON users TO identity_app';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON user_personal TO identity_app';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON shredding_keys TO identity_app';
    EXECUTE 'GRANT SELECT, INSERT, DELETE ON consent_records TO identity_app';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON support_access_grants TO identity_app';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON device_sessions TO identity_app';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON export_requests TO identity_app';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON deletion_requests TO identity_app';
    EXECUTE 'GRANT SELECT ON jurisdiction_rules TO identity_app';

    IF to_regclass('_sqlx_migrations') IS NOT NULL THEN
        EXECUTE 'GRANT SELECT ON _sqlx_migrations TO identity_app';
    END IF;

    EXECUTE 'GRANT EXECUTE ON FUNCTION protect(text, bigint) TO identity_app';
    EXECUTE 'GRANT EXECUTE ON FUNCTION unprotect(bytea, bigint) TO identity_app';
    EXECUTE 'GRANT EXECUTE ON FUNCTION create_shredding_key(text) TO identity_app';
    EXECUTE 'GRANT EXECUTE ON FUNCTION destroy_shredding_key(bigint) TO identity_app';

    -- identity_readonly: select only.
    EXECUTE 'GRANT SELECT ON users TO identity_readonly';
    EXECUTE 'GRANT SELECT ON shredding_keys TO identity_readonly';
    EXECUTE 'GRANT SELECT ON consent_records TO identity_readonly';
    EXECUTE 'GRANT SELECT ON support_access_grants TO identity_readonly';
    EXECUTE 'GRANT SELECT ON device_sessions TO identity_readonly';
    EXECUTE 'GRANT SELECT ON export_requests TO identity_readonly';
    EXECUTE 'GRANT SELECT ON deletion_requests TO identity_readonly';
    EXECUTE 'GRANT SELECT ON jurisdiction_rules TO identity_readonly';

    -- identity_migrator: DDL only.
    EXECUTE 'GRANT ALL ON users TO identity_migrator';
    EXECUTE 'GRANT ALL ON user_personal TO identity_migrator';
    EXECUTE 'GRANT ALL ON shredding_keys TO identity_migrator';
    EXECUTE 'GRANT ALL ON consent_records TO identity_migrator';
    EXECUTE 'GRANT ALL ON support_access_grants TO identity_migrator';
    EXECUTE 'GRANT ALL ON device_sessions TO identity_migrator';
    EXECUTE 'GRANT ALL ON export_requests TO identity_migrator';
    EXECUTE 'GRANT ALL ON deletion_requests TO identity_migrator';
    EXECUTE 'GRANT ALL ON jurisdiction_rules TO identity_migrator';
END
$$;
