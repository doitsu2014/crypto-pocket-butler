-- C9 (added by u8-reporting): seed and rotate the Reporting service-token
-- registry.
--
-- Run this after `reporting-migrate`, with a role allowed to write the registry
-- (the release job, not the runtime `reporting_svc` role). Never put a token in
-- this file: the job hashes each Docker secret and passes the hex digest.
--
--   sha256sum < /run/secrets/reporting_identity_token
--   psql "$REPORTING_MIGRATE_DATABASE_URL" \
--        -v identity_sha256="$IDENTITY_SHA256" \
--        -v run_sha256="$JOB_ORCHESTRATION_SHA256" \
--        -v smoke_sha256="$SMOKE_SHA256" \
--        -f infra/sql/reporting-service-tokens.sql
--
-- The outbound `reporting` token is created in each consuming service's registry
-- by that service's own seed (Identity, Ledger, Portfolio, Notification, Audit).

INSERT INTO reporting.service_token_registry (id, token_hash, caller, operations, active, created_at)
VALUES
    (gen_random_uuid(), :'identity_sha256', 'identity', ARRAY['privacy_export', 'privacy_anonymise'], true, now()),
    (gen_random_uuid(), :'run_sha256', 'job-orchestration', ARRAY['run_brief_generation'], true, now()),
    (gen_random_uuid(), :'smoke_sha256', 'smoke-reader', ARRAY['run_brief_generation'], true, now())
ON CONFLICT (token_hash) DO NOTHING;
