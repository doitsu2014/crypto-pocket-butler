-- Reverse of 0005_preferences_consent.up.sql.
--
-- The release job applies this as its rollback path, so it must finish on real
-- data. Two facts drive the shape:
--
--   * **every** consent event is copied into `consent_records_legacy_archive`
--     BEFORE the collapse, so a rollback never loses a grant or withdrawal —
--     not just the `risk`/`age` kinds the old shape cannot represent;
--   * the archive table is created with `IF NOT EXISTS` and every DDL step is
--     idempotent, so a second rollback (after a redeploy) re-runs cleanly.
--
-- The three mapable kinds then collapse to their newest event with the inverse
-- one-to-one mapping:
--   analytics -> analytics, marketing -> marketing, terms -> disclosures.
-- `legacy:<source>` in `policy_version` restores the original `source`.

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

-- The event-shape portion runs only while the columns exist, so the second
-- rollback skips it instead of referencing dropped columns.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = current_schema()
          AND table_name = 'consent_records'
          AND column_name = 'kind'
    ) THEN
        -- Archive every row: event-shaped, and a legacy-shaped row a previous
        -- replica may have written during the overlap. `ON CONFLICT` keeps a
        -- repeated rollback idempotent.
        INSERT INTO consent_records_legacy_archive
            (id, user_id, kind, action, occurred_at, policy_version)
        SELECT id,
               user_id,
               COALESCE(kind, CASE purpose
                   WHEN 'analytics' THEN 'analytics'
                   WHEN 'marketing' THEN 'marketing'
                   WHEN 'disclosures' THEN 'terms'
               END),
               COALESCE(action, CASE WHEN granted THEN 'granted' ELSE 'withdrawn' END),
               COALESCE(occurred_at, recorded_at),
               COALESCE(policy_version, 'legacy:' || COALESCE(source, 'settings'))
        FROM consent_records
        WHERE COALESCE(kind, CASE purpose
                  WHEN 'analytics' THEN 'analytics'
                  WHEN 'marketing' THEN 'marketing'
                  WHEN 'disclosures' THEN 'terms'
              END) IS NOT NULL
        ON CONFLICT (id) DO NOTHING;

        -- Collapse and rebuild in one data-modifying statement: every row is
        -- removed, and the newest row per mapable kind becomes its legacy row.
        -- The mapping also covers a legacy-shaped row.
        WITH gone AS (
            DELETE FROM consent_records
            RETURNING id, user_id, kind, purpose, granted, recorded_at, source,
                      action, occurred_at, seq, policy_version
        ),
        mapped AS (
            SELECT id, user_id, seq,
                   COALESCE(kind, CASE purpose
                       WHEN 'analytics' THEN 'analytics'
                       WHEN 'marketing' THEN 'marketing'
                       WHEN 'disclosures' THEN 'terms'
                   END) AS mapped_kind,
                   COALESCE(action, CASE WHEN granted THEN 'granted' ELSE 'withdrawn' END)
                       AS mapped_action,
                   COALESCE(occurred_at, recorded_at) AS mapped_at,
                   COALESCE(policy_version, 'legacy:' || COALESCE(source, 'settings'))
                       AS mapped_version
            FROM gone
        ),
        newest AS (
            SELECT DISTINCT ON (user_id, mapped_kind)
                   user_id, mapped_kind, mapped_action, mapped_at, mapped_version, seq
            FROM mapped
            WHERE mapped_kind IN ('analytics', 'marketing', 'terms')
            ORDER BY user_id, mapped_kind, mapped_at DESC, seq DESC
        )
        INSERT INTO consent_records (id, user_id, purpose, granted, recorded_at, source)
        SELECT gen_random_uuid(),
               user_id,
               CASE mapped_kind
                   WHEN 'analytics' THEN 'analytics'
                   WHEN 'marketing' THEN 'marketing'
                   ELSE 'disclosures'
               END,
               mapped_action = 'granted',
               mapped_at,
               CASE
                   WHEN mapped_version IN ('legacy:onboarding', 'legacy:settings', 'legacy:support')
                       THEN substring(mapped_version from 8)
                   ELSE 'settings'
               END
        FROM newest;
    END IF;
END
$$;

-- The application's append-only lock is removed before the old grants return.
DROP TRIGGER IF EXISTS consent_records_append_only ON consent_records;
DROP FUNCTION IF EXISTS consent_records_append_only();

-- Restore the legacy NOT NULLs and defaults before the CHECKs are re-imposed.
ALTER TABLE consent_records ALTER COLUMN purpose SET NOT NULL;
ALTER TABLE consent_records ALTER COLUMN granted SET NOT NULL;
ALTER TABLE consent_records ALTER COLUMN recorded_at SET NOT NULL;
ALTER TABLE consent_records ALTER COLUMN source SET NOT NULL;
ALTER TABLE consent_records ALTER COLUMN source SET DEFAULT 'onboarding';

ALTER TABLE consent_records DROP CONSTRAINT IF EXISTS consent_records_event_shape_check;
ALTER TABLE consent_records DROP CONSTRAINT IF EXISTS consent_records_kind_check;
ALTER TABLE consent_records DROP CONSTRAINT IF EXISTS consent_records_action_check;
ALTER TABLE consent_records DROP CONSTRAINT IF EXISTS consent_records_policy_version_check;

ALTER TABLE consent_records DROP COLUMN IF EXISTS seq;
DROP SEQUENCE IF EXISTS consent_records_seq_seq;
ALTER TABLE consent_records DROP COLUMN IF EXISTS policy_version;
ALTER TABLE consent_records DROP COLUMN IF EXISTS occurred_at;
ALTER TABLE consent_records DROP COLUMN IF EXISTS action;
ALTER TABLE consent_records DROP COLUMN IF EXISTS kind;
ALTER TABLE consent_records ALTER COLUMN id DROP DEFAULT;

-- The old shape's CHECKs, added only when absent so the second rollback passes.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'consent_records_purpose_check'
          AND conrelid = 'consent_records'::regclass
    ) THEN
        ALTER TABLE consent_records
            ADD CONSTRAINT consent_records_purpose_check
                CHECK (purpose IN ('analytics', 'marketing', 'disclosures'));
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'consent_records_source_check'
          AND conrelid = 'consent_records'::regclass
    ) THEN
        ALTER TABLE consent_records
            ADD CONSTRAINT consent_records_source_check
                CHECK (source IN ('onboarding', 'settings', 'support'));
    END IF;
END
$$;

DROP INDEX IF EXISTS consent_records_state_idx;
DROP INDEX IF EXISTS consent_records_history_idx;
CREATE INDEX IF NOT EXISTS consent_records_latest_idx
    ON consent_records (user_id, purpose, recorded_at DESC, id DESC);

-- The two tables this migration added. The archive deliberately stays: it holds
-- the `risk`/`age` provenance the old shape cannot represent.
DROP TABLE IF EXISTS consent_audit_outbox;
DROP TABLE IF EXISTS user_preferences;

-- The old application's grants. 0001 gave it SELECT, INSERT, DELETE — never
-- UPDATE — so UPDATE stays revoked.
DO $$
BEGIN
    EXECUTE 'GRANT SELECT, INSERT, DELETE ON consent_records TO identity_app';
END
$$;
