-- Provisioning for the Analytics service's own schema and roles.
-- Applied once by the platform/database administrator, never by the service.
--
-- The migration (services/analytics/migrations/0001_init.up.sql) also creates
-- these roles idempotently and grants object privileges, so a disposable
-- database and production converge. This file is the bootstrap that runs before
-- the first migration and records where each credential goes:
--
--   analytics_svc      -> Supabase Vault path analytics/svc_dsn      -> Docker secret analytics_database_url
--   analytics_readonly -> Supabase Vault path analytics/readonly_dsn -> (operator tooling only)
--   analytics_migrator -> Supabase Vault path analytics/migrate_dsn  -> Docker secret analytics_migrate_database_url
--
-- Salt versions live in Vault under `analytics/salt/<version>`. The migration's
-- SECURITY DEFINER function `analytics.read_analytics_salt(version)` reads one
-- without a cross-schema grant, and the `analytics.salt_versions` table lists the
-- version labels (never the bytes).
--
-- No credential is written here. The roles below are created NOLOGIN; the
-- platform's Supabase Vault bootstrap owns the LOGIN attribute and the password
-- for each role, then stores it at the Vault path named above.

CREATE SCHEMA IF NOT EXISTS analytics;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'analytics_svc') THEN
        EXECUTE 'CREATE ROLE analytics_svc NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'analytics_readonly') THEN
        EXECUTE 'CREATE ROLE analytics_readonly NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'analytics_migrator') THEN
        EXECUTE 'CREATE ROLE analytics_migrator NOLOGIN';
    END IF;
END
$$;

GRANT USAGE ON SCHEMA analytics TO analytics_svc, analytics_readonly, analytics_migrator;
GRANT CREATE ON SCHEMA analytics TO analytics_migrator;

-- Each role reads and writes only its own schema. No cross-schema grants and no
-- Row Level Security: access is bounded by the role and the schema, and other
-- services reach Analytics over HTTP only.
ALTER ROLE analytics_svc SET search_path TO analytics;
ALTER ROLE analytics_readonly SET search_path TO analytics;
ALTER ROLE analytics_migrator SET search_path TO analytics;
