-- Release-job step: seed and rotate the Notification service-token registry.
--
-- Run this after `notification-migrate`, with a role allowed to write the
-- registry (the release job, not the runtime `notification_svc` role). Never put
-- a token in this file: the job hashes each Docker secret and passes the hex
-- digest, either as a psql variable (`-v account_sha256=...`) or as a bound
-- parameter.
--
--   sha256sum < /run/secrets/notification_account_token
--   psql "$NOTIFICATION_MIGRATE_DATABASE_URL" \
--        -v gateway_sha256="$GATEWAY_SHA256" \
--        -v identity_sha256="$IDENTITY_SHA256" \
--        -v account_sha256="$ACCOUNT_SHA256" \
--        -v portfolio_sha256="$PORTFOLIO_SHA256" \
--        -v reporting_sha256="$REPORTING_SHA256" \
--        -v run_sha256="$JOB_ORCHESTRATION_SHA256" \
--        -f infra/sql/notification-service-tokens.sql
--
-- The smoke token is not seeded here: the API and worker register
-- `notification_smoke_token` in process from its Docker secret.
--
-- Rotation is insert-then-delete so both tokens resolve while consumers roll:
--   1. run this file with the new digest (the old row stays),
--   2. deploy every consumer with its new token,
--   3. delete the superseded digest with the statement at the bottom.

INSERT INTO notification.service_token_registry (service, token_sha256, allowed_operations)
VALUES
    ('gateway', :'gateway_sha256', ARRAY['smoke']),
    ('identity', :'identity_sha256', ARRAY['alerts.submit']),
    ('account', :'account_sha256', ARRAY['alerts.submit']),
    ('portfolio', :'portfolio_sha256', ARRAY['alerts.submit']),
    ('reporting', :'reporting_sha256', ARRAY['alerts.submit']),
    ('job-orchestration', :'run_sha256', ARRAY['runs'])
ON CONFLICT (service, token_sha256) DO NOTHING;

-- Step 3, after every consumer serves the new token (replace the variables):
-- DELETE FROM notification.service_token_registry
-- WHERE service = :'rotate_service' AND token_sha256 <> :'rotate_new_sha256';
