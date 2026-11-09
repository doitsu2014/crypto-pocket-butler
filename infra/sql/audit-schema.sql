-- Provisioning for the Audit service's own schema and roles.
-- Applied once by the platform/database administrator, never by the service.
--
-- The migration (services/audit/migrations/0001_init.up.sql) also creates these
-- roles idempotently and grants object privileges, so a disposable database and
-- production converge. This file is the bootstrap that runs before the first
-- migration and records where each credential goes:
--
--   audit_app       -> Supabase Vault path audit/app_dsn       -> Docker secret audit_app_dsn
--   audit_readonly  -> Supabase Vault path audit/readonly_dsn  -> (operator tooling only)
--   audit_retention -> Supabase Vault path audit/retention_dsn -> Docker secret audit_retention_dsn
--   audit_migrator  -> Supabase Vault path audit/migrator_dsn  -> pipeline migration step only
--
-- No credential is written here. The roles below are created NOLOGIN; the
-- platform's Supabase Vault bootstrap owns the LOGIN attribute and the password
-- for each role, then stores it at the Vault path named above. This file never
-- mints a login, so a leaked bootstrap cannot create one, and the pipeline
-- materialises the Docker secret from Vault.

CREATE SCHEMA IF NOT EXISTS audit;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'audit_app') THEN
        EXECUTE 'CREATE ROLE audit_app NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'audit_readonly') THEN
        EXECUTE 'CREATE ROLE audit_readonly NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'audit_retention') THEN
        EXECUTE 'CREATE ROLE audit_retention NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'audit_migrator') THEN
        EXECUTE 'CREATE ROLE audit_migrator NOLOGIN';
    END IF;
END
$$;

GRANT USAGE ON SCHEMA audit TO audit_app, audit_readonly, audit_retention, audit_migrator;
GRANT CREATE ON SCHEMA audit TO audit_migrator;

-- Each role reads and writes only its own schema. No cross-schema grants and no
-- Row Level Security: access is bounded by the role and the schema, and other
-- services reach Audit over HTTP only.
ALTER ROLE audit_app SET search_path TO audit;
ALTER ROLE audit_readonly SET search_path TO audit;
ALTER ROLE audit_retention SET search_path TO audit;
ALTER ROLE audit_migrator SET search_path TO audit;
