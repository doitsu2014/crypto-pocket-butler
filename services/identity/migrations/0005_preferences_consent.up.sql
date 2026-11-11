-- U4 — preferences and the consent event shape (FR19, BR4.1–BR4.11).
--
-- `consent_records` already exists (0001) with `purpose`, `granted`,
-- `recorded_at`, and `source`. This migration ALTERS it to the append-only event
-- shape the unit's design settles:
--
--   * a new event carries `kind`, `action`, `occurred_at`, and `policy_version`;
--   * `seq` is a database-generated monotonic tie-breaker;
--   * the legacy columns stay, and stay nullable, so a replica still running the
--     previous code keeps appending through the `start-first` overlap;
--   * a shape CHECK keeps a row either wholly legacy or wholly an event.
--
-- LIVE-SAFE ORDER — the runner executes this file ONE TOP-LEVEL STATEMENT AT A
-- TIME, outside a transaction (`identity_infrastructure::migrations`). That is
-- required: PostgreSQL refuses `CREATE INDEX CONCURRENTLY`, and a procedure's
-- `COMMIT`, when a multi-statement simple query runs in one implicit
-- transaction. The order is:
--
--   1. role (metadata)
--   2. nullable columns, sequence, and the legacy NOT NULL/CHECK drops — one
--      short ACCESS EXCLUSIVE statement each
--   3. the event-shape backfill as a procedure that COMMITs every 10 000 rows
--   4. the seq backfill the same way, then `NOT NULL` via a validated CHECK
--      (avoids the long ACCESS EXCLUSIVE table scan)
--   5. the CHECKs added `NOT VALID` and then `VALIDATE CONSTRAINT` (SHARE
--      UPDATE EXCLUSIVE — blocks writes for the scan, not reads)
--   6. the two new indexes built `CONCURRENTLY` outside any transaction
--   7. the new tables, grants, and the append-only trigger (metadata)
--
-- RESIDUAL LOCK WINDOW: each `ALTER TABLE` still takes a brief ACCESS EXCLUSIVE
-- lock, and `VALIDATE CONSTRAINT` holds SHARE UPDATE EXCLUSIVE for the length of
-- its table scan. On a large table that scan is the longest write-blocking step;
-- the backfills and index builds are not. Expand now, contract later: the new
-- columns stay nullable through this release so a previous replica keeps writing.
--
-- Object names are unqualified on purpose: the migration runs with `search_path`
-- set to the service's own schema, and adapter tests point it at a disposable
-- schema. Every step is idempotent.

-- ---------------------------------------------------------------------------
-- The erasure role
-- ---------------------------------------------------------------------------
-- 0001 says "a migration must not mint a login or a password". This role is the
-- deliberate exception the unit's design settles: the erasure job connects as
-- `identity_retention` directly — no membership in `identity_app`, which would
-- inherit the revoked DELETE and undo the append-only lock. The migration mints
-- no password; the credential is materialised from Supabase Vault by the
-- platform's role bootstrap. It may only read `user_id` and delete the three
-- erasure-scoped tables below.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'identity_retention') THEN
        EXECUTE 'CREATE ROLE identity_retention LOGIN';
    END IF;
END
$$;

-- ---------------------------------------------------------------------------
-- The event columns (nullable through this release)
-- ---------------------------------------------------------------------------
ALTER TABLE consent_records ADD COLUMN IF NOT EXISTS kind text;
ALTER TABLE consent_records ADD COLUMN IF NOT EXISTS action text;
ALTER TABLE consent_records ADD COLUMN IF NOT EXISTS occurred_at timestamptz;
ALTER TABLE consent_records ADD COLUMN IF NOT EXISTS policy_version text;
ALTER TABLE consent_records ADD COLUMN IF NOT EXISTS seq bigint;

CREATE SEQUENCE IF NOT EXISTS consent_records_seq_seq AS bigint;

-- The default goes on BEFORE the backfill. A replica still running the previous
-- code inserts a legacy row with no `seq`; the default gives it one, so such a
-- row can never abort the `seq IS NOT NULL` validation below.
ALTER TABLE consent_records
    ALTER COLUMN seq SET DEFAULT nextval('consent_records_seq_seq');

-- A legacy row supplies its own `purpose`, `granted`, `recorded_at`, and
-- `source`; an event row supplies none of them. Both shapes are writable, so the
-- legacy NOT NULLs and CHECKs must go.
ALTER TABLE consent_records ALTER COLUMN purpose DROP NOT NULL;
ALTER TABLE consent_records ALTER COLUMN granted DROP NOT NULL;
ALTER TABLE consent_records ALTER COLUMN recorded_at DROP NOT NULL;
ALTER TABLE consent_records ALTER COLUMN source DROP NOT NULL;
ALTER TABLE consent_records ALTER COLUMN source DROP DEFAULT;
ALTER TABLE consent_records DROP CONSTRAINT IF EXISTS consent_records_purpose_check;
ALTER TABLE consent_records DROP CONSTRAINT IF EXISTS consent_records_source_check;

-- ---------------------------------------------------------------------------
-- Backfill the event shape, batched with a commit per batch
-- ---------------------------------------------------------------------------
-- One-to-one mapping, so no two legacy rows can land on one kind:
--   analytics -> analytics, marketing -> marketing, disclosures -> terms.
-- `source` is preserved in `policy_version` as `legacy:<source>`.
CREATE OR REPLACE PROCEDURE consent_backfill_event_shape()
LANGUAGE plpgsql
AS $backfill$
DECLARE
    batch_size constant integer := 10000;
    processed integer;
BEGIN
    LOOP
        WITH batch AS (
            SELECT id FROM consent_records
            WHERE kind IS NULL
            ORDER BY recorded_at, id
            LIMIT batch_size
        )
        UPDATE consent_records c
        SET kind = CASE c.purpose
                       WHEN 'analytics' THEN 'analytics'
                       WHEN 'marketing' THEN 'marketing'
                       ELSE 'terms'
                   END,
            action = CASE WHEN coalesce(c.granted, false) THEN 'granted' ELSE 'withdrawn' END,
            occurred_at = coalesce(c.recorded_at, now()),
            policy_version = 'legacy:' || coalesce(c.source, 'settings')
        FROM batch
        WHERE c.id = batch.id;
        GET DIAGNOSTICS processed = ROW_COUNT;
        COMMIT;
        EXIT WHEN processed = 0;
    END LOOP;
END
$backfill$;

CALL consent_backfill_event_shape();
DROP PROCEDURE IF EXISTS consent_backfill_event_shape();

-- `seq` is added nullable and backfilled rather than declared `bigserial`: a
-- volatile default would rewrite the table under ACCESS EXCLUSIVE.
CREATE OR REPLACE PROCEDURE consent_backfill_seq()
LANGUAGE plpgsql
AS $backfill$
DECLARE
    batch_size constant integer := 10000;
    processed integer;
BEGIN
    LOOP
        WITH batch AS (
            SELECT id FROM consent_records
            WHERE seq IS NULL
            ORDER BY recorded_at NULLS LAST, id
            LIMIT batch_size
        )
        UPDATE consent_records c
        SET seq = nextval('consent_records_seq_seq')
        FROM batch
        WHERE c.id = batch.id;
        GET DIAGNOSTICS processed = ROW_COUNT;
        COMMIT;
        EXIT WHEN processed = 0;
    END LOOP;
END
$backfill$;

CALL consent_backfill_seq();
-- A second pass just before `SET NOT NULL` catches a row a previous replica
-- wrote between the first pass and this point (it already has a default `seq`,
-- so this is a no-op for it; the pass is the belt to the default's braces).
CALL consent_backfill_seq();
DROP PROCEDURE IF EXISTS consent_backfill_seq();

-- `SET NOT NULL` is cheap once a validated CHECK proves the column. The CHECK is
-- added NOT VALID and validated first, so no long ACCESS EXCLUSIVE scan. It is
-- added only after the default exists, so a late legacy insert cannot make the
-- validation fail.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'consent_records_seq_not_null'
          AND conrelid = 'consent_records'::regclass
    ) THEN
        ALTER TABLE consent_records
            ADD CONSTRAINT consent_records_seq_not_null CHECK (seq IS NOT NULL) NOT VALID;
    END IF;
END
$$;
ALTER TABLE consent_records VALIDATE CONSTRAINT consent_records_seq_not_null;
ALTER TABLE consent_records ALTER COLUMN seq SET NOT NULL;
ALTER TABLE consent_records DROP CONSTRAINT IF EXISTS consent_records_seq_not_null;

ALTER TABLE consent_records ALTER COLUMN id SET DEFAULT gen_random_uuid();

-- ---------------------------------------------------------------------------
-- The new CHECKs, added NOT VALID and validated separately
-- ---------------------------------------------------------------------------
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'consent_records_kind_check'
          AND conrelid = 'consent_records'::regclass
    ) THEN
        ALTER TABLE consent_records
            ADD CONSTRAINT consent_records_kind_check
                CHECK (kind IS NULL OR kind IN ('terms', 'risk', 'age', 'analytics', 'marketing'))
                NOT VALID;
    END IF;
END
$$;
ALTER TABLE consent_records VALIDATE CONSTRAINT consent_records_kind_check;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'consent_records_action_check'
          AND conrelid = 'consent_records'::regclass
    ) THEN
        ALTER TABLE consent_records
            ADD CONSTRAINT consent_records_action_check
                CHECK (action IS NULL OR action IN ('granted', 'withdrawn'))
                NOT VALID;
    END IF;
END
$$;
ALTER TABLE consent_records VALIDATE CONSTRAINT consent_records_action_check;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'consent_records_policy_version_check'
          AND conrelid = 'consent_records'::regclass
    ) THEN
        ALTER TABLE consent_records
            ADD CONSTRAINT consent_records_policy_version_check
                CHECK (policy_version IS NULL OR char_length(policy_version) BETWEEN 1 AND 120)
                NOT VALID;
    END IF;
END
$$;
ALTER TABLE consent_records VALIDATE CONSTRAINT consent_records_policy_version_check;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'consent_records_event_shape_check'
          AND conrelid = 'consent_records'::regclass
    ) THEN
        ALTER TABLE consent_records
            ADD CONSTRAINT consent_records_event_shape_check
                CHECK (
                    (kind IS NULL AND action IS NULL AND occurred_at IS NULL
                        AND policy_version IS NULL)
                    OR (kind IS NOT NULL AND action IS NOT NULL AND occurred_at IS NOT NULL
                        AND policy_version IS NOT NULL)
                )
                NOT VALID;
    END IF;
END
$$;
ALTER TABLE consent_records VALIDATE CONSTRAINT consent_records_event_shape_check;

-- ---------------------------------------------------------------------------
-- Indexes, built concurrently outside any transaction
-- ---------------------------------------------------------------------------
-- `DROP INDEX CONCURRENTLY` avoids the ACCESS EXCLUSIVE lock a plain drop takes.
-- Dropping then recreating makes a re-run after a failed concurrent build
-- converge instead of leaving an INVALID index behind.
DROP INDEX CONCURRENTLY IF EXISTS consent_records_latest_idx;
DROP INDEX CONCURRENTLY IF EXISTS consent_records_state_idx;
CREATE INDEX CONCURRENTLY consent_records_state_idx
    ON consent_records (user_id, kind, occurred_at DESC, seq DESC);
DROP INDEX CONCURRENTLY IF EXISTS consent_records_history_idx;
CREATE INDEX CONCURRENTLY consent_records_history_idx
    ON consent_records (user_id, occurred_at DESC, seq DESC, id);

-- ---------------------------------------------------------------------------
-- Preferences (explicit choices only; a NULL field is "never set", BR4.1)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS user_preferences (
    user_id            uuid        PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    locale             text        CHECK (locale IS NULL OR locale IN ('en', 'vi')),
    base_currency      text        CHECK (
        base_currency IS NULL OR base_currency IN ('USD', 'EUR', 'GBP', 'VND')
    ),
    timezone           text        CHECK (
        timezone IS NULL OR char_length(timezone) BETWEEN 1 AND 64
    ),
    notification_email boolean,
    notification_push  boolean,
    updated_at         timestamptz NOT NULL
);

-- ---------------------------------------------------------------------------
-- The audit outbox (commits with the consent event, NFR19.1–19.3)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS consent_audit_outbox (
    id                uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    consent_record_id uuid        NOT NULL REFERENCES consent_records (id) ON DELETE CASCADE,
    payload           jsonb       NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    idempotency_key   text        NOT NULL UNIQUE
        CHECK (char_length(idempotency_key) BETWEEN 1 AND 120),
    state             text        NOT NULL DEFAULT 'pending'
        CHECK (state IN ('pending', 'delivered', 'dead')),
    attempts          integer     NOT NULL DEFAULT 0 CHECK (attempts BETWEEN 0 AND 10),
    next_attempt_at   timestamptz NOT NULL,
    created_at        timestamptz NOT NULL,
    delivered_at      timestamptz,
    last_error        text        CHECK (last_error IS NULL OR char_length(last_error) <= 300),
    CONSTRAINT consent_audit_outbox_delivered_shape CHECK (
        (state = 'delivered' AND delivered_at IS NOT NULL)
        OR (state <> 'delivered' AND delivered_at IS NULL)
    )
);

CREATE INDEX IF NOT EXISTS consent_audit_outbox_due_idx
    ON consent_audit_outbox (next_attempt_at) WHERE state = 'pending';
CREATE INDEX IF NOT EXISTS consent_audit_outbox_delivered_idx
    ON consent_audit_outbox (delivered_at) WHERE state = 'delivered';

-- ---------------------------------------------------------------------------
-- The rollback archive (shared resource: created here and by the down)
-- ---------------------------------------------------------------------------
-- The archive holds every consent event the down migration removes, so a
-- rollback never loses grant or withdrawal history. `kind` admits all five kinds.
CREATE TABLE IF NOT EXISTS consent_records_legacy_archive (
    id             uuid        PRIMARY KEY,
    user_id        uuid        NOT NULL,
    kind           text        NOT NULL
        CHECK (kind IN ('terms', 'risk', 'age', 'analytics', 'marketing')),
    action         text        NOT NULL CHECK (action IN ('granted', 'withdrawn')),
    occurred_at    timestamptz NOT NULL,
    policy_version text        NOT NULL,
    archived_at    timestamptz NOT NULL DEFAULT now()
);

-- ---------------------------------------------------------------------------
-- Append-only
-- ---------------------------------------------------------------------------
-- Privilege alone is one migration away from being widened, so the trigger is
-- the second lock. It raises only for `identity_app`: the erasure role and the
-- migration role are the two sanctioned writers of a delete.
CREATE OR REPLACE FUNCTION consent_records_append_only()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF current_user = 'identity_app' THEN
        RAISE EXCEPTION 'consent_records is append-only for the application'
            USING ERRCODE = '42501';
    END IF;
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS consent_records_append_only ON consent_records;
CREATE TRIGGER consent_records_append_only
    BEFORE UPDATE OR DELETE ON consent_records
    FOR EACH ROW EXECUTE FUNCTION consent_records_append_only();

-- ---------------------------------------------------------------------------
-- Grants
-- ---------------------------------------------------------------------------
DO $$
DECLARE
    schema_name text := coalesce(current_schema(), 'identity');
BEGIN
    EXECUTE format('GRANT USAGE ON SCHEMA %I TO identity_retention', schema_name);

    -- identity_app: append consent, read it, never rewrite it.
    EXECUTE 'REVOKE UPDATE, DELETE ON consent_records FROM identity_app';
    EXECUTE 'GRANT USAGE ON SEQUENCE consent_records_seq_seq TO identity_app';
    EXECUTE 'GRANT USAGE ON SEQUENCE consent_records_seq_seq TO identity_migrator';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON user_preferences TO identity_app';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON consent_audit_outbox TO identity_app';

    -- identity_retention: the erasure job's delete path. PostgreSQL needs SELECT
    -- on any column a DELETE ... WHERE reads, so `user_id` is granted beside
    -- DELETE.
    EXECUTE 'GRANT SELECT (user_id) ON consent_records TO identity_retention';
    EXECUTE 'GRANT DELETE ON consent_records TO identity_retention';
    EXECUTE 'GRANT SELECT (user_id) ON user_preferences TO identity_retention';
    EXECUTE 'GRANT DELETE ON user_preferences TO identity_retention';
    EXECUTE 'GRANT SELECT (user_id) ON consent_records_legacy_archive TO identity_retention';
    EXECUTE 'GRANT DELETE ON consent_records_legacy_archive TO identity_retention';

    EXECUTE 'GRANT ALL ON user_preferences TO identity_migrator';
    EXECUTE 'GRANT ALL ON consent_audit_outbox TO identity_migrator';
    EXECUTE 'GRANT ALL ON consent_records_legacy_archive TO identity_migrator';
END
$$;
