-- Release-job step: seed and rotate the Portfolio service-token registry.
--
-- Run this after `portfolio-migrate`, with a role allowed to write the registry
-- (the release job, not the runtime `portfolio_svc` role). Never put a token in
-- this file: the job hashes each Docker secret and passes the hex digest, either
-- as a psql variable (`-v reporting_sha256=...`) or as a bound parameter.
--
--   sha256sum < /run/secrets/portfolio_identity_token
--   psql "$PORTFOLIO_MIGRATE_DATABASE_URL" \
--        -v reporting_sha256="$REPORTING_SHA256" \
--        -v identity_sha256="$IDENTITY_SHA256" \
--        -v run_sha256="$JOB_ORCHESTRATION_SHA256" \
--        -v smoke_sha256="$SMOKE_SHA256" \
--        -f infra/sql/portfolio-service-tokens.sql
--
-- The outbound `portfolio` token the service presents to C5–C10 is not seeded
-- here: it is created in each consuming service's registry by that service's
-- own seed, and Portfolio reads it from its Docker secret token file.
--
-- Rotation is insert-then-delete so both tokens resolve while consumers roll:
--   1. run this file with the new digest (the old row stays; `caller` is not
--      unique, only the token hash is),
--   2. deploy every consumer with its new token,
--   3. delete the superseded digest, for example:
--      DELETE FROM portfolio.service_token_registry
--      WHERE caller = 'reporting' AND token_hash <> :'reporting_sha256';

INSERT INTO portfolio.service_token_registry (id, token_hash, caller, operations, active, created_at)
VALUES
    (gen_random_uuid(), :'reporting_sha256', 'reporting', ARRAY['snapshots_read', 'portfolios_read', 'valuation_read'], true, now()),
    (gen_random_uuid(), :'identity_sha256', 'identity', ARRAY['privacy_export', 'privacy_anonymise'], true, now()),
    (gen_random_uuid(), :'run_sha256', 'job-orchestration', ARRAY['run_valuation'], true, now()),
    (gen_random_uuid(), :'smoke_sha256', 'smoke-reader', ARRAY['snapshots_read'], true, now())
ON CONFLICT (token_hash) DO NOTHING;
