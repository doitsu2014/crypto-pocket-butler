-- Provisioning for the Portfolio service's own schema and roles.
-- Applied once by the platform/database administrator, never by the service.
--
-- The migration (services/portfolio/migrations/0001_init.up.sql) also creates
-- these roles idempotently and grants object privileges, so a disposable
-- database and production converge. This file is the bootstrap that runs before
-- the first migration and records where each credential goes:
--
--   portfolio_svc      -> Supabase Vault path portfolio/svc_dsn      -> Docker secret portfolio_database_url
--   portfolio_readonly -> Supabase Vault path portfolio/readonly_dsn -> (operator tooling only)
--   portfolio_migrator -> Supabase Vault path portfolio/migrate_dsn  -> Docker secret portfolio_migrate_database_url
--
-- No credential is written here. The roles below are created NOLOGIN; the
-- platform's Supabase Vault bootstrap owns the LOGIN attribute and the password
-- for each role, then stores it at the Vault path named above. This file never
-- mints a login, so a leaked bootstrap cannot create one.

CREATE SCHEMA IF NOT EXISTS portfolio;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'portfolio_svc') THEN
        EXECUTE 'CREATE ROLE portfolio_svc NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'portfolio_readonly') THEN
        EXECUTE 'CREATE ROLE portfolio_readonly NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'portfolio_migrator') THEN
        EXECUTE 'CREATE ROLE portfolio_migrator NOLOGIN';
    END IF;
END
$$;

GRANT USAGE ON SCHEMA portfolio TO portfolio_svc, portfolio_readonly, portfolio_migrator;
GRANT CREATE ON SCHEMA portfolio TO portfolio_migrator;
