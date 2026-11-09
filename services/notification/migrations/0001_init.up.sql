-- Notification schema: per-user preferences, deduplicated alerts, delivery
-- attempts, the durable wake flag and delivery lease, and the service-token
-- registry.
--
-- Object names are unqualified on purpose: the migration runs with `search_path`
-- set to the service's own schema, and adapter tests point the same migration at
-- a disposable schema. No cross-schema grants and no Row Level Security.
--
-- Roles are created idempotently so a disposable database and production
-- converge on the same shape:
--   notification_svc       select/insert/update/delete alerts and preferences;
--                          select/insert/delete attempts; select/update the
--                          delivery control row; select the token registry
--   notification_migrator  DDL
--
-- The verified email address never reaches this schema (NFR9.3).

-- ---------------------------------------------------------------------------
-- Roles
-- ---------------------------------------------------------------------------
DO $$
BEGIN
    BEGIN
        IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'notification_svc') THEN
            EXECUTE 'CREATE ROLE notification_svc NOLOGIN';
        END IF;
        IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'notification_migrator') THEN
            EXECUTE 'CREATE ROLE notification_migrator NOLOGIN';
        END IF;
    EXCEPTION WHEN insufficient_privilege THEN
        -- A disposable test database may not grant CREATEROLE. Production's
        -- Vault bootstrap owns the roles; the tables still converge here.
        RAISE NOTICE 'skipping notification role creation: %', SQLERRM;
    END;
END
$$;

-- The roles are created NOLOGIN on purpose: a migration must not mint a login or
-- a password. The platform's Supabase Vault bootstrap owns the LOGIN attribute
-- and the credential for each role.

-- ---------------------------------------------------------------------------
-- Preferences (BR2.1–BR2.3, BR3.1)
-- ---------------------------------------------------------------------------
CREATE TABLE notification_preferences (
    id         uuid        PRIMARY KEY,
    user_id    uuid        NOT NULL,
    type       text        NOT NULL
        CHECK (type IN ('drift', 'sync_failed', 'key_revoked', 'security', 'brief')),
    channel    text        NOT NULL CHECK (channel IN ('in_app', 'email')),
    enabled    boolean     NOT NULL,
    updated_at timestamptz NOT NULL,
    CONSTRAINT notification_preferences_pair UNIQUE (user_id, type, channel)
);

CREATE INDEX notification_preferences_user_idx ON notification_preferences (user_id);

-- ---------------------------------------------------------------------------
-- Alerts (BR1.1)
-- ---------------------------------------------------------------------------
-- The alerts table is the queue; there is no message queue (project rule).
-- `sanitised_fields` names payload keys the content guard dropped (BR5.1); the
-- dropped values are never stored.
CREATE TABLE alerts (
    id               uuid        PRIMARY KEY,
    user_id          uuid        NOT NULL,
    type             text        NOT NULL
        CHECK (type IN ('drift', 'sync_failed', 'key_revoked', 'security', 'brief')),
    idempotency_key  text        NOT NULL UNIQUE
        CHECK (char_length(idempotency_key) BETWEEN 1 AND 128),
    payload          jsonb       NOT NULL DEFAULT '{}'::jsonb
        CHECK (jsonb_typeof(payload) = 'object'),
    status           text        NOT NULL
        CHECK (status IN ('queued', 'delivered', 'partially_delivered', 'failed')),
    sanitised_fields text[]      NOT NULL DEFAULT '{}',
    next_attempt_at  timestamptz,
    created_at       timestamptz NOT NULL,
    updated_at       timestamptz NOT NULL
);

-- The feed keyset is (user_id, created_at desc, id).
CREATE INDEX alerts_feed_idx ON alerts (user_id, created_at DESC, id DESC);
-- Due work only ever looks at queued alerts.
CREATE INDEX alerts_due_idx ON alerts (next_attempt_at) WHERE status = 'queued';
CREATE INDEX alerts_retention_idx ON alerts (created_at);

-- ---------------------------------------------------------------------------
-- Delivery attempts (BR4.1, BR4.2)
-- ---------------------------------------------------------------------------
CREATE TABLE delivery_attempts (
    id             uuid        PRIMARY KEY,
    alert_id       uuid        NOT NULL REFERENCES alerts (id) ON DELETE CASCADE,
    user_id        uuid        NOT NULL,
    channel        text        NOT NULL CHECK (channel IN ('in_app', 'email')),
    attempt        smallint    NOT NULL CHECK (attempt BETWEEN 1 AND 3),
    status         text        NOT NULL CHECK (status IN ('sent', 'failed', 'abandoned')),
    error_category text        CHECK (
        error_category IS NULL
        OR error_category IN ('provider', 'network', 'address', 'content')
    ),
    attempted_at   timestamptz NOT NULL,
    CONSTRAINT delivery_attempts_number UNIQUE (alert_id, channel, attempt),
    -- A sent attempt carries no category; a failed or abandoned one must.
    CONSTRAINT delivery_attempts_category CHECK (
        (status = 'sent' AND error_category IS NULL)
        OR (status <> 'sent' AND error_category IS NOT NULL)
    )
);

CREATE INDEX delivery_attempts_alert_idx ON delivery_attempts (alert_id);
CREATE INDEX delivery_attempts_retention_idx ON delivery_attempts (attempted_at);

-- ---------------------------------------------------------------------------
-- Wake flag and delivery lease (NFR2.5, NFR20.4)
-- ---------------------------------------------------------------------------
-- One singleton row. The API replicas set the wake flag; the single leased
-- worker polls it every five seconds and holds the lease while it runs. The
-- provider-call allowance is a column so `reserve_email` is atomic.
CREATE TABLE delivery_control (
    id                text        PRIMARY KEY,
    wake_requested_at timestamptz,
    lease_owner       uuid,
    lease_expires_at  timestamptz,
    emails_sent       integer     NOT NULL DEFAULT 0 CHECK (emails_sent >= 0),
    last_completed_at timestamptz
);

INSERT INTO delivery_control (id) VALUES ('notification-delivery');

-- ---------------------------------------------------------------------------
-- Service-token registry
-- ---------------------------------------------------------------------------
-- Only a SHA-256 hash is stored. The deploy job holds write rights for rotation
-- (insert-then-delete); the runtime role reads only. The primary key carries the
-- hash so a rotation can hold the old and new token at once.
CREATE TABLE service_token_registry (
    service            text        NOT NULL,
    token_sha256       text        NOT NULL UNIQUE CHECK (char_length(token_sha256) = 64),
    allowed_operations text[]      NOT NULL,
    rotated_at         timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (service, token_sha256)
);

-- ---------------------------------------------------------------------------
-- Grants
-- ---------------------------------------------------------------------------
DO $$
DECLARE
    schema_name text := coalesce(current_schema(), 'notification');
BEGIN
    EXECUTE format(
        'GRANT USAGE ON SCHEMA %I TO notification_svc, notification_migrator',
        schema_name
    );
    EXECUTE format('GRANT CREATE ON SCHEMA %I TO notification_migrator', schema_name);

    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON notification_preferences TO notification_svc';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON alerts TO notification_svc';
    EXECUTE 'GRANT SELECT, INSERT, DELETE ON delivery_attempts TO notification_svc';
    EXECUTE 'GRANT SELECT, UPDATE ON delivery_control TO notification_svc';
    EXECUTE 'GRANT SELECT ON service_token_registry TO notification_svc';

    IF to_regclass('_sqlx_migrations') IS NOT NULL THEN
        EXECUTE 'GRANT SELECT ON _sqlx_migrations TO notification_svc';
    END IF;

    EXECUTE 'GRANT ALL ON notification_preferences TO notification_migrator';
    EXECUTE 'GRANT ALL ON alerts TO notification_migrator';
    EXECUTE 'GRANT ALL ON delivery_attempts TO notification_migrator';
    EXECUTE 'GRANT ALL ON delivery_control TO notification_migrator';
    EXECUTE 'GRANT ALL ON service_token_registry TO notification_migrator';
END
$$;
