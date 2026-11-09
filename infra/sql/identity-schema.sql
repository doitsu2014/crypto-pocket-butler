-- Provisioning for the Identity service's own schema and roles.
-- Applied once by the platform/database administrator, never by the service.
--
-- The migration (services/identity/migrations/0001_init.up.sql) also creates
-- these roles idempotently and grants object privileges, so a disposable
-- database and production converge. This file is the bootstrap that runs before
-- the first migration and records where each credential goes:
--
--   identity_app       -> Supabase Vault path identity/app_dsn       -> Docker secret identity_database_url
--   identity_readonly  -> Supabase Vault path identity/readonly_dsn  -> (operator tooling only)
--   identity_migrator  -> Supabase Vault path identity/migrator_dsn  -> pipeline migration step only
--
-- The per-subject crypto-shredding keys live in Supabase Vault and pgsodium;
-- their ids are stored in identity.shredding_keys. No key material is stored in
-- this schema or written here.
--
-- No credential is written here. The roles below are created NOLOGIN; the
-- platform's Supabase Vault bootstrap owns the LOGIN attribute and the password
-- for each role, then stores it at the Vault path named above. This file never
-- mints a login, so a leaked bootstrap cannot create one, and the pipeline
-- materialises the Docker secret from Vault.

CREATE SCHEMA IF NOT EXISTS identity;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'identity_app') THEN
        EXECUTE 'CREATE ROLE identity_app NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'identity_readonly') THEN
        EXECUTE 'CREATE ROLE identity_readonly NOLOGIN';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'identity_migrator') THEN
        EXECUTE 'CREATE ROLE identity_migrator NOLOGIN';
    END IF;
    IF EXISTS (SELECT 1 FROM pg_available_extensions WHERE name = 'pgsodium') THEN
        EXECUTE 'CREATE EXTENSION IF NOT EXISTS pgsodium';
    END IF;
END
$$;

GRANT USAGE ON SCHEMA identity TO identity_app, identity_readonly, identity_migrator;
GRANT CREATE ON SCHEMA identity TO identity_migrator;

-- Each role reads and writes only its own schema. No cross-schema grants and no
-- Row Level Security: access is bounded by the role and the schema, and other
-- services reach Identity over HTTP only.
ALTER ROLE identity_app SET search_path TO identity;
ALTER ROLE identity_readonly SET search_path TO identity;
ALTER ROLE identity_migrator SET search_path TO identity;

-- The private export bucket is created by the platform (Supabase Storage), not
-- by this bootstrap: identity-exports, private, encrypted at rest, 24-hour
-- owner-only signed download.
