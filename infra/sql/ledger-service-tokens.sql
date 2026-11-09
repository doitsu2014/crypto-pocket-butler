-- Release-job step: seed and rotate the Ledger service-token registry.
--
-- Run this after `ledger-migrate`, with a role allowed to write the registry
-- (the release job, not the runtime `ledger_svc` role). Never put a token in
-- this file: the job hashes each Docker secret and passes the hex digest.
--
--   sha256sum < /run/secrets/ledger_reporting_token
--   psql "$LEDGER_MIGRATE_DATABASE_URL" \
--        -v reporting_sha256="$REPORTING_SHA256" \
--        -v portfolio_sha256="$PORTFOLIO_SHA256" \
--        -v identity_sha256="$IDENTITY_SHA256" \
--        -v run_sha256="$JOB_ORCHESTRATION_SHA256" \
--        -v smoke_sha256="$SMOKE_SHA256" \
--        -f infra/sql/ledger-service-tokens.sql
--
-- Rotation is insert-then-delete: the registry's primary key is `service`, so a
-- rotation replaces the row and both tokens cannot resolve at once. Deploy the
-- consumer with its new token in the same step.

INSERT INTO ledger.service_token_registry (service, token_sha256, allowed_operations)
VALUES
    ('reporting', :'reporting_sha256', ARRAY['pnl.read']),
    ('portfolio', :'portfolio_sha256', ARRAY['trades.write']),
    ('identity', :'identity_sha256', ARRAY['privacy.export', 'privacy.anonymise']),
    ('job-orchestration', :'run_sha256', ARRAY['runs']),
    ('smoke-reader', :'smoke_sha256', ARRAY['smoke'])
ON CONFLICT (service) DO UPDATE
    SET token_sha256 = EXCLUDED.token_sha256,
        allowed_operations = EXCLUDED.allowed_operations,
        rotated_at = now();
