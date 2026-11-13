-- Release-job step: seed and rotate the Account service-token registry.
--
-- Run this after `account-migrate`, with a role allowed to write the registry
-- (the release job, not the runtime `account_svc` role). Never put a token in
-- this file: the job hashes each Docker secret and passes the hex digest as a
-- psql variable.
--
--   sha256sum < /run/secrets/account_identity_token
--   psql "$ACCOUNT_MIGRATE_DATABASE_URL" \
--        -v identity_sha256="$IDENTITY_SHA256" \
--        -v portfolio_sha256="$PORTFOLIO_SHA256" \
--        -v ledger_sha256="$LEDGER_SHA256" \
--        -v run_sha256="$JOB_ORCHESTRATION_SHA256" \
--        -v smoke_sha256="$SMOKE_SHA256" \
--        -f infra/sql/account-service-tokens.sql
--
-- Unlike the other registries, Account enforces `caller` as unique, so rotation
-- replaces the row's digest in place. Steps:
--   1. run this file with the new digest (the caller's old token stops
--      resolving the moment the row is updated),
--   2. deploy the consumer with its new token read from its Docker secret.
--
-- The Identity row carries `support_metadata`: the new operation the support
-- read path uses (BR3.9). The privacy operations stay on the same row.

INSERT INTO account.service_token_registry (id, token_hash, caller, operations, active, created_at)
VALUES
    (gen_random_uuid(), :'identity_sha256', 'identity', ARRAY['privacy_export', 'privacy_anonymise', 'support_metadata'], true, now()),
    (gen_random_uuid(), :'portfolio_sha256', 'portfolio', ARRAY['holdings_read', 'transactions_read'], true, now()),
    (gen_random_uuid(), :'ledger_sha256', 'ledger', ARRAY['holdings_read', 'transactions_read'], true, now()),
    (gen_random_uuid(), :'run_sha256', 'job-orchestration', ARRAY['run_scheduled_sync'], true, now()),
    (gen_random_uuid(), :'smoke_sha256', 'smoke-reader', ARRAY['holdings_read'], true, now())
ON CONFLICT (caller) DO UPDATE
    SET token_hash = EXCLUDED.token_hash,
        operations = EXCLUDED.operations,
        active = true;