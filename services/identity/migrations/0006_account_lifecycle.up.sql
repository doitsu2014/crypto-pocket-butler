-- U5 — the account lifecycle: the erasure tombstone, the recovery window, the
-- append-only status transition log, and its audit outbox
-- (US4.1–US4.5, US7.2, BR5.1–BR5.10).
--
-- Four changes:
--   1. `users.erased_subject_hash` — the pseudonymous tombstone a still-valid
--      token is matched against and denied (BR5.8, AC4.5.1). Erasure copies the
--      hash here BEFORE `user_personal` is deleted, so the subject is never in
--      neither place.
--   2. `deletion_requests` gains the recovery window and the erasure outcome:
--      `recoverable_until`, `cancelled_at`, `erased_at`, `attempt_count`,
--      `next_attempt_at`. `0001`'s status-keyed unique index is replaced by the
--      open-request index (`cancelled_at IS NULL AND erased_at IS NULL`), so a
--      parked or cancelled request stops blocking a new one.
--   3. `status_transitions` — append-only evidence of every status change, with
--      a database-generated monotonic `seq` and an edge CHECK that is the last
--      line behind BR5.1.
--   4. `status_audit_outbox` — the durable audit intent written in the same
--      transaction as the status change and drained to C9 by the worker
--      (BR5.9). No queue, no distributed transaction: an accepted transition
--      always has its audit row.
--
-- The migration is applied by the transactional migrator (`0006` is above the
-- single live-safe version `0005`), so no statement here may run outside a
-- transaction. The operations are metadata-only plus one bounded backfill of
-- `deletion_requests`; `users` gains a nullable column with a partial index, so
-- no table rewrite is required.
--
-- Object names are unqualified on purpose: the migration runs with `search_path`
-- set to the service's own schema, and adapter tests point it at a disposable
-- schema. No cross-schema grant, no Row Level Security, no new role.

-- ---------------------------------------------------------------------------
-- The tombstone
-- ---------------------------------------------------------------------------
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS erased_subject_hash text
        CHECK (erased_subject_hash IS NULL OR char_length(erased_subject_hash) = 64);

-- Only an erased row can carry a tombstone. A live row keeps its subject in
-- `user_personal`; a hash here would make the subject discoverable twice.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'users_erased_subject_shape'
          AND conrelid = 'users'::regclass
    ) THEN
        ALTER TABLE users
            ADD CONSTRAINT users_erased_subject_shape
                CHECK (erased_subject_hash IS NULL OR status = 'deleted');
    END IF;
END
$$;

CREATE INDEX IF NOT EXISTS users_erased_subject_idx
    ON users (erased_subject_hash)
    WHERE erased_subject_hash IS NOT NULL;

-- ---------------------------------------------------------------------------
-- The recovery window on deletion_requests
-- ---------------------------------------------------------------------------
ALTER TABLE deletion_requests
    ADD COLUMN IF NOT EXISTS recoverable_until timestamptz;
ALTER TABLE deletion_requests
    ADD COLUMN IF NOT EXISTS cancelled_at timestamptz;
ALTER TABLE deletion_requests
    ADD COLUMN IF NOT EXISTS erased_at timestamptz;
ALTER TABLE deletion_requests
    ADD COLUMN IF NOT EXISTS attempt_count integer NOT NULL DEFAULT 0;
ALTER TABLE deletion_requests
    ADD COLUMN IF NOT EXISTS next_attempt_at timestamptz NOT NULL DEFAULT now();

-- A legacy request opened before this migration had no explicit deadline. Its
-- window is measured from `requested_at`: midnight UTC on the 30th calendar day,
-- the same rule the application computes for a new request.
UPDATE deletion_requests
SET recoverable_until = date_trunc('day', requested_at + interval '30 days')
WHERE recoverable_until IS NULL;

-- `completed` in the old machine means erased in the new one.
UPDATE deletion_requests
SET erased_at = COALESCE(completed_at, requested_at)
WHERE status = 'completed' AND erased_at IS NULL;

ALTER TABLE deletion_requests
    ALTER COLUMN recoverable_until SET NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'deletion_requests_attempt_count_check'
          AND conrelid = 'deletion_requests'::regclass
    ) THEN
        ALTER TABLE deletion_requests
            ADD CONSTRAINT deletion_requests_attempt_count_check
                CHECK (attempt_count BETWEEN 0 AND 100);
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'deletion_requests_erased_shape'
          AND conrelid = 'deletion_requests'::regclass
    ) THEN
        ALTER TABLE deletion_requests
            ADD CONSTRAINT deletion_requests_erased_shape
                CHECK ((erased_at IS NOT NULL) = (status = 'completed'));
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'deletion_requests_window_shape'
          AND conrelid = 'deletion_requests'::regclass
    ) THEN
        ALTER TABLE deletion_requests
            ADD CONSTRAINT deletion_requests_window_shape
                CHECK (cancelled_at IS NULL OR erased_at IS NULL);
    END IF;
END
$$;

-- The open request is "neither cancelled nor erased", independent of the
-- bookkeeping status, so a parked `failed` row is still closed.
DROP INDEX IF EXISTS deletion_requests_one_active_per_user;
CREATE UNIQUE INDEX IF NOT EXISTS deletion_requests_one_open_per_user
    ON deletion_requests (user_id)
    WHERE cancelled_at IS NULL AND erased_at IS NULL;

CREATE INDEX IF NOT EXISTS deletion_requests_due_idx
    ON deletion_requests (recoverable_until)
    WHERE cancelled_at IS NULL AND erased_at IS NULL;

-- ---------------------------------------------------------------------------
-- The append-only status transition log
-- ---------------------------------------------------------------------------
CREATE SEQUENCE IF NOT EXISTS status_transitions_seq_seq AS bigint;

CREATE TABLE IF NOT EXISTS status_transitions (
    id          uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     uuid        NOT NULL REFERENCES users (id),
    from_status text        NOT NULL
        CHECK (from_status IN ('pending', 'active', 'disabled', 'pending_deletion', 'deleted')),
    to_status   text        NOT NULL
        CHECK (to_status IN ('pending', 'active', 'disabled', 'pending_deletion', 'deleted')),
    actor       text        NOT NULL CHECK (char_length(actor) BETWEEN 1 AND 120),
    reason      text        CHECK (reason IS NULL OR char_length(reason) BETWEEN 1 AND 500),
    occurred_at timestamptz NOT NULL,
    seq         bigint      NOT NULL DEFAULT nextval('status_transitions_seq_seq'),
    -- BR5.1, as the schema's last line: the six permitted edges and no other.
    CONSTRAINT status_transitions_edge_check CHECK (
        (from_status = 'pending' AND to_status = 'active')
        OR (from_status = 'active' AND to_status IN ('disabled', 'pending_deletion'))
        OR (from_status = 'disabled' AND to_status = 'active')
        OR (from_status = 'pending_deletion' AND to_status IN ('active', 'deleted'))
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS status_transitions_seq_idx
    ON status_transitions (seq);
CREATE INDEX IF NOT EXISTS status_transitions_user_idx
    ON status_transitions (user_id, occurred_at DESC, seq DESC);

-- Privilege is one migration away from being widened, so the trigger is the
-- second lock: the application may append, never rewrite or delete evidence.
CREATE OR REPLACE FUNCTION status_transitions_append_only()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF current_user = 'identity_app' THEN
        RAISE EXCEPTION 'status_transitions is append-only for the application'
            USING ERRCODE = '42501';
    END IF;
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS status_transitions_append_only ON status_transitions;
CREATE TRIGGER status_transitions_append_only
    BEFORE UPDATE OR DELETE ON status_transitions
    FOR EACH ROW EXECUTE FUNCTION status_transitions_append_only();

-- ---------------------------------------------------------------------------
-- The audit outbox (written in the same transaction as the status change)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS status_audit_outbox (
    id              uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    transition_id   uuid        NOT NULL REFERENCES status_transitions (id) ON DELETE CASCADE,
    payload         jsonb       NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    idempotency_key text        NOT NULL UNIQUE
        CHECK (char_length(idempotency_key) BETWEEN 1 AND 120),
    state           text        NOT NULL DEFAULT 'pending'
        CHECK (state IN ('pending', 'delivered', 'dead')),
    attempts        integer     NOT NULL DEFAULT 0 CHECK (attempts BETWEEN 0 AND 10),
    next_attempt_at timestamptz NOT NULL,
    created_at      timestamptz NOT NULL,
    delivered_at    timestamptz,
    last_error      text        CHECK (last_error IS NULL OR char_length(last_error) <= 300),
    CONSTRAINT status_audit_outbox_delivered_shape CHECK (
        (state = 'delivered' AND delivered_at IS NOT NULL)
        OR (state <> 'delivered' AND delivered_at IS NULL)
    )
);

CREATE INDEX IF NOT EXISTS status_audit_outbox_due_idx
    ON status_audit_outbox (next_attempt_at) WHERE state = 'pending';
CREATE INDEX IF NOT EXISTS status_audit_outbox_delivered_idx
    ON status_audit_outbox (delivered_at) WHERE state = 'delivered';

-- ---------------------------------------------------------------------------
-- Grants (no new role; the application appends the log and drains the outbox)
-- ---------------------------------------------------------------------------
DO $$
BEGIN
    EXECUTE 'GRANT SELECT, INSERT ON status_transitions TO identity_app';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON status_audit_outbox TO identity_app';
    EXECUTE 'GRANT USAGE ON SEQUENCE status_transitions_seq_seq TO identity_app';
    EXECUTE 'GRANT SELECT ON status_transitions TO identity_readonly';

    EXECUTE 'GRANT ALL ON status_transitions TO identity_migrator';
    EXECUTE 'GRANT ALL ON status_audit_outbox TO identity_migrator';
    EXECUTE 'GRANT ALL ON SEQUENCE status_transitions_seq_seq TO identity_migrator';
END
$$;
