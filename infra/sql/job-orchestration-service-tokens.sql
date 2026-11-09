-- Release-job step: seed and rotate the Job Orchestration service-token registry.
--
--   sha256sum < /run/secrets/job_orchestration_smoke_token
--   psql "$JOB_ORCHESTRATION_MIGRATE_DATABASE_URL" \
--        -v smoke_sha256="$SMOKE_SHA256" \
--        -f infra/sql/job-orchestration-service-tokens.sql
--
-- The outbound token is seeded by each provider's own C11 seed: Account,
-- Market Data, Portfolio, Ledger, Analytics, Notification, Reporting, Identity
-- and Audit each allow-list a `job-orchestration` caller for their run endpoints.

INSERT INTO job_orchestration.service_token_registry (id, token_hash, caller, operations, active, created_at)
VALUES
    (gen_random_uuid(), :'smoke_sha256', 'smoke-reader', ARRAY['smoke'], true, now())
ON CONFLICT (token_hash) DO NOTHING;
