-- Job Orchestration schema: the fixed catalogue, one run row per attempt, the
-- single orchestrator lease, the heartbeat, backfill requests, the retention
-- watermark, and the inbound service-token registry.
--
-- Object names are unqualified: the migration runs with `search_path` set to
-- this service's schema. No cross-schema grants, no Row Level Security.

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'job_orchestration_svc') THEN
        EXECUTE 'CREATE ROLE job_orchestration_svc NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'job_orchestration_readonly') THEN
        EXECUTE 'CREATE ROLE job_orchestration_readonly NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'job_orchestration_migrator') THEN
        EXECUTE 'CREATE ROLE job_orchestration_migrator NOLOGIN';
    END IF;
END
$$;

CREATE TABLE job_definitions (
    name               text        PRIMARY KEY,
    provider           text        NOT NULL CHECK (char_length(provider) BETWEEN 1 AND 80),
    schedule           text        NOT NULL CHECK (char_length(schedule) BETWEEN 1 AND 40),
    timeout_seconds    integer     NOT NULL CHECK (timeout_seconds BETWEEN 1 AND 3600),
    max_attempts       integer     NOT NULL CHECK (max_attempts BETWEEN 1 AND 5),
    enabled            boolean     NOT NULL DEFAULT true,
    last_scheduled_for timestamptz
);

CREATE TABLE job_runs (
    id               uuid        PRIMARY KEY,
    job              text        NOT NULL REFERENCES job_definitions (name),
    scheduled_for    timestamptz NOT NULL,
    attempt          integer     NOT NULL CHECK (attempt >= 1),
    status           text        NOT NULL
        CHECK (status IN ('pending', 'running', 'succeeded', 'failed', 'skipped', 'abandoned')),
    trigger          text        NOT NULL
        CHECK (trigger IN ('scheduled', 'missed', 'backfill', 'manual')),
    actor_id         uuid,
    reason           text        CHECK (reason IS NULL OR char_length(reason) BETWEEN 1 AND 200),
    owner            text,
    lease_until      timestamptz,
    next_attempt_at  timestamptz,
    started_at       timestamptz,
    finished_at      timestamptz,
    response_status  smallint,
    detail           text        CHECK (detail IS NULL OR char_length(detail) <= 500),
    CONSTRAINT job_runs_window_unique UNIQUE (job, scheduled_for, attempt),
    CONSTRAINT job_runs_actor_shape CHECK (
        (trigger IN ('backfill', 'manual') AND actor_id IS NOT NULL AND reason IS NOT NULL)
        OR trigger IN ('scheduled', 'missed')
    )
);

CREATE INDEX job_runs_window_idx ON job_runs (job, scheduled_for DESC);
CREATE INDEX job_runs_retry_idx ON job_runs (status, next_attempt_at) WHERE status = 'failed';
CREATE INDEX job_runs_pending_idx ON job_runs (scheduled_for) WHERE status = 'pending';
CREATE INDEX job_runs_running_idx ON job_runs (lease_until) WHERE status = 'running';

CREATE TABLE orchestrator_lease (
    name        text        PRIMARY KEY,
    owner       text,
    lease_until timestamptz
);

INSERT INTO orchestrator_lease (name, owner, lease_until)
VALUES ('job-orchestrator', NULL, NULL);

CREATE TABLE heartbeat (
    name       text        PRIMARY KEY,
    owner      text,
    updated_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO heartbeat (name, owner) VALUES ('job-orchestrator', NULL);

CREATE TABLE backfill_requests (
    id           uuid        PRIMARY KEY,
    job          text        NOT NULL REFERENCES job_definitions (name),
    from_at      timestamptz NOT NULL,
    to_at        timestamptz NOT NULL,
    actor_id     uuid        NOT NULL,
    reason       text        NOT NULL CHECK (char_length(reason) BETWEEN 1 AND 200),
    requested_at timestamptz NOT NULL,
    CONSTRAINT backfill_width CHECK (to_at >= from_at AND to_at - from_at <= interval '7 days')
);

CREATE TABLE prune_state (
    name           text        PRIMARY KEY,
    last_pruned_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO prune_state (name) VALUES ('job-orchestrator');

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

-- The fixed catalogue (functional-design/rules.md).
INSERT INTO job_definitions (name, provider, schedule, timeout_seconds, max_attempts) VALUES
    ('account-sync', 'account', 'every:5', 600, 3),
    ('market-data-refresh', 'market-data', 'every:5', 300, 3),
    ('market-data-fx', 'market-data', 'hourly:5', 300, 3),
    ('portfolio-valuation', 'portfolio', 'hourly:10', 900, 3),
    ('portfolio-snapshot', 'portfolio', 'daily:00:15', 900, 3),
    ('ledger-reconcile', 'ledger', 'hourly:20', 600, 3),
    ('ledger-ingest', 'ledger', 'every:10', 600, 3),
    ('analytics-aggregate', 'analytics', 'hourly:30', 900, 3),
    ('analytics-flag-expiry', 'analytics', 'daily:01:00', 600, 3),
    ('notification-delivery', 'notification', 'every:1', 300, 3),
    ('notification-digest', 'notification', 'daily:08:00', 600, 3),
    ('brief-generation', 'reporting', 'hourly:0', 1800, 2),
    ('audit-privacy-sweep', 'identity', 'every:5', 600, 3),
    ('audit-retention', 'audit', 'daily:02:30', 1800, 2);

DO $$
DECLARE
    schema_name text := coalesce(current_schema(), 'job_orchestration');
BEGIN
    EXECUTE format('GRANT USAGE ON SCHEMA %I TO job_orchestration_svc, job_orchestration_readonly, job_orchestration_migrator', schema_name);
    EXECUTE format('GRANT CREATE ON SCHEMA %I TO job_orchestration_migrator', schema_name);

    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON job_definitions TO job_orchestration_svc';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON job_runs TO job_orchestration_svc';
    EXECUTE 'GRANT SELECT, UPDATE ON orchestrator_lease TO job_orchestration_svc';
    EXECUTE 'GRANT SELECT, UPDATE ON heartbeat TO job_orchestration_svc';
    EXECUTE 'GRANT SELECT, INSERT ON backfill_requests TO job_orchestration_svc';
    EXECUTE 'GRANT SELECT, UPDATE ON prune_state TO job_orchestration_svc';
    EXECUTE 'GRANT SELECT ON service_token_registry TO job_orchestration_svc';

    IF to_regclass('_sqlx_migrations') IS NOT NULL THEN
        EXECUTE 'GRANT SELECT ON _sqlx_migrations TO job_orchestration_svc';
    END IF;

    EXECUTE 'GRANT SELECT ON ALL TABLES IN SCHEMA ' || quote_ident(schema_name) || ' TO job_orchestration_readonly';
    EXECUTE 'GRANT ALL ON ALL TABLES IN SCHEMA ' || quote_ident(schema_name) || ' TO job_orchestration_migrator';
END
$$;
