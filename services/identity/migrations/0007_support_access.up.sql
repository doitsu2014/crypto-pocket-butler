-- 0007 — Support access: the audit outbox and the live-pair index.
--
-- Three changes, each serving one rule:
--   1. `support_grants_approval_shape` admits exactly one more row — a request
--      that expired before it was approved, which has no approval fields at all
--      (BR3.5, W2 step 3). No other shape is relaxed.
--   2. A partial unique index allows at most one `pending_approval` or `active`
--      grant per (support_agent_id, user_id) (BR3.3, NFR20.1). A terminal row
--      does not occupy the slot, so a new request is allowed after a revoke or
--      an expiry.
--   3. `support_audit_outbox` mirrors `consent_audit_outbox` and
--      `status_audit_outbox`: the grant change and its audit intent commit in
--      one transaction (BR3.13), and the existing drainer delivers it to C10
--      with an idempotency key, exponential backoff and a three-attempt dead
--      letter (NFR1.4–NFR1.7).
--
-- Object names are unqualified on purpose: the migration runs with
-- `search_path` set to the service's own schema, and adapter tests point the
-- same migration at a disposable schema. No cross-schema grants, no Row Level
-- Security.

-- ---------------------------------------------------------------------------
-- 1. The approval shape, relaxed by one clause
-- ---------------------------------------------------------------------------
-- The old constraint is dropped and re-added, which validates every existing
-- row. The new predicate is strictly weaker — it adds one OR branch — so every
-- row that already satisfied the old shape still satisfies this one.
ALTER TABLE support_access_grants DROP CONSTRAINT support_grants_approval_shape;

ALTER TABLE support_access_grants
    ADD CONSTRAINT support_grants_approval_shape CHECK (
        (status = 'pending_approval'
            AND approved_by IS NULL AND approved_at IS NULL AND expires_at IS NULL)
        OR (status <> 'pending_approval' AND approved_at IS NOT NULL)
        OR (status = 'revoked' AND approved_at IS NULL)
        -- W2 step 3: a request older than the window expires unapproved.
        OR (status = 'expired'
            AND approved_by IS NULL AND approved_at IS NULL AND expires_at IS NULL)
    );

-- ---------------------------------------------------------------------------
-- 2. One live grant per (agent, target)
-- ---------------------------------------------------------------------------
-- `expired` and `revoked` rows are history and stay out of the index.
CREATE UNIQUE INDEX support_grants_live_pair_idx
    ON support_access_grants (support_agent_id, user_id)
    WHERE status IN ('pending_approval', 'active');

-- ---------------------------------------------------------------------------
-- 3. The support audit outbox
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS support_audit_outbox (
    id              uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
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
    CONSTRAINT support_audit_outbox_delivered_shape CHECK (
        (state = 'delivered' AND delivered_at IS NOT NULL)
        OR (state <> 'delivered' AND delivered_at IS NULL)
    )
);

CREATE INDEX IF NOT EXISTS support_audit_outbox_due_idx
    ON support_audit_outbox (next_attempt_at) WHERE state = 'pending';
CREATE INDEX IF NOT EXISTS support_audit_outbox_delivered_idx
    ON support_audit_outbox (delivered_at) WHERE state = 'delivered';

-- ---------------------------------------------------------------------------
-- Grants (NFR23.1)
-- ---------------------------------------------------------------------------
DO $$
DECLARE
    schema_name text := coalesce(current_schema(), 'identity');
BEGIN
    EXECUTE format('GRANT USAGE ON SCHEMA %I TO identity_app, identity_readonly, identity_migrator',
                   schema_name);

    -- identity_app: the drainer claims, delivers, retries and deletes delivered
    -- rows after the 24 h retention window, so it needs every verb.
    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON support_audit_outbox TO identity_app';

    -- identity_readonly: operator tooling reads the backlog only.
    EXECUTE 'GRANT SELECT ON support_audit_outbox TO identity_readonly';

    -- identity_migrator: DDL.
    EXECUTE 'GRANT ALL ON support_audit_outbox TO identity_migrator';
END
$$;