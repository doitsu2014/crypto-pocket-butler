-- Local service-token registries.
--
-- Every internal caller gets one generated token (deployment/localhost/secrets/
-- service-tokens.json). This file stores the SHA-256 digest of each token in the
-- registry of every service that authenticates callers against the database.
--
-- Local seeding is deliberately generous: each caller is allowed every operation
-- its target service defines. The caller names are still exact, so the
-- identity-only paths (privacy export/anonymise) keep their name checks. Do not
-- copy this seeding into a shared environment.
--
-- Run with -v tokens='{"caller":"<sha256>",...}'.
\set ON_ERROR_STOP on

-- account ---------------------------------------------------------------
DELETE FROM account.service_token_registry
WHERE caller IN (SELECT key FROM jsonb_each_text(:'tokens'::jsonb));
INSERT INTO account.service_token_registry (id, token_hash, caller, operations, active, created_at)
SELECT gen_random_uuid(), value, key,
       ARRAY['privacy_export', 'privacy_anonymise', 'holdings_read',
             'transactions_read', 'run_scheduled_sync'],
       true, now()
FROM jsonb_each_text(:'tokens'::jsonb);

-- analytics -------------------------------------------------------------
DELETE FROM analytics.service_token_registry
WHERE service IN (SELECT key FROM jsonb_each_text(:'tokens'::jsonb));
INSERT INTO analytics.service_token_registry (service, token_sha256, allowed_operations, rotated_at)
SELECT key, value, ARRAY['runs', 'smoke', 'privacy.erase'], now()
FROM jsonb_each_text(:'tokens'::jsonb);

-- job-orchestration -----------------------------------------------------
DELETE FROM job_orchestration.service_token_registry
WHERE caller IN (SELECT key FROM jsonb_each_text(:'tokens'::jsonb));
INSERT INTO job_orchestration.service_token_registry
    (id, token_hash, caller, operations, active, created_at)
SELECT gen_random_uuid(), value, key, ARRAY['smoke'], true, now()
FROM jsonb_each_text(:'tokens'::jsonb);

-- ledger ----------------------------------------------------------------
DELETE FROM ledger.service_token_registry
WHERE service IN (SELECT key FROM jsonb_each_text(:'tokens'::jsonb));
INSERT INTO ledger.service_token_registry (service, token_sha256, allowed_operations, rotated_at)
SELECT key, value,
       ARRAY['pnl.read', 'trades.write', 'privacy.export', 'privacy.anonymise',
             'runs', 'smoke'],
       now()
FROM jsonb_each_text(:'tokens'::jsonb);

-- market-data -----------------------------------------------------------
DELETE FROM market_data.service_token_registry
WHERE caller IN (SELECT key FROM jsonb_each_text(:'tokens'::jsonb));
INSERT INTO market_data.service_token_registry
    (id, token_hash, caller, operations, active, created_at)
SELECT gen_random_uuid(), value, key,
       ARRAY['prices_read', 'fx_read', 'run_refresh'], true, now()
FROM jsonb_each_text(:'tokens'::jsonb);

-- notification ----------------------------------------------------------
DELETE FROM notification.service_token_registry
WHERE service IN (SELECT key FROM jsonb_each_text(:'tokens'::jsonb));
INSERT INTO notification.service_token_registry
    (service, token_sha256, allowed_operations, rotated_at)
SELECT key, value, ARRAY['alerts.submit', 'runs', 'smoke', 'privacy.anonymise'], now()
FROM jsonb_each_text(:'tokens'::jsonb);

-- portfolio -------------------------------------------------------------
DELETE FROM portfolio.service_token_registry
WHERE caller IN (SELECT key FROM jsonb_each_text(:'tokens'::jsonb));
INSERT INTO portfolio.service_token_registry
    (id, token_hash, caller, operations, active, created_at)
SELECT gen_random_uuid(), value, key,
       ARRAY['snapshots_read', 'portfolios_read', 'valuation_read',
             'privacy_export', 'privacy_anonymise', 'run_valuation'],
       true, now()
FROM jsonb_each_text(:'tokens'::jsonb);

-- reporting -------------------------------------------------------------
DELETE FROM reporting.service_token_registry
WHERE caller IN (SELECT key FROM jsonb_each_text(:'tokens'::jsonb));
INSERT INTO reporting.service_token_registry
    (id, token_hash, caller, operations, active, created_at)
SELECT gen_random_uuid(), value, key,
       ARRAY['run_brief_generation', 'privacy_export', 'privacy_anonymise', 'smoke'],
       true, now()
FROM jsonb_each_text(:'tokens'::jsonb);
