-- Local Supabase bootstrap: schemas and login roles.
--
-- Production mints every service role (NOLOGIN) in the migrations and takes the
-- LOGIN credential from Supabase Vault. A developer laptop has no Vault
-- bootstrap, so this file gives each service role a local LOGIN and password
-- and pre-creates the schemas the migrations expect to write into.
--
-- Run as the `postgres` superuser with -v db_password=...
\set ON_ERROR_STOP on

-- psql does not substitute variables inside a dollar-quoted body, so the
-- password reaches the DO blocks through a temporary table.
CREATE TEMP TABLE local_bootstrap (app_password text NOT NULL);
INSERT INTO local_bootstrap (app_password) VALUES (:'db_password');

DO $$
DECLARE
  password_value text;
  role_name text;
BEGIN
  SELECT app_password INTO password_value FROM local_bootstrap;
  -- Roles the service migrations also create (idempotently). Creating them here
  -- with LOGIN means the migration's IF NOT EXISTS branch is taken and its
  -- grants still apply.
  FOREACH role_name IN ARRAY ARRAY[
    'identity_app', 'identity_readonly', 'identity_migrator',
    'account_svc', 'account_readonly', 'account_migrator',
    'analytics_svc', 'analytics_readonly', 'analytics_migrator',
    'audit_app', 'audit_readonly', 'audit_retention', 'audit_migrator',
    'job_orchestration_svc', 'job_orchestration_readonly', 'job_orchestration_migrator',
    'ledger_svc', 'ledger_anonymiser', 'ledger_readonly', 'ledger_migrator',
    'market_data_svc',
    'notification_svc', 'notification_migrator',
    'portfolio_svc', 'portfolio_readonly', 'portfolio_migrator',
    'reporting_svc', 'reporting_readonly', 'reporting_migrator',
    'registry_svc'
  ]::text[] LOOP
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = role_name) THEN
      EXECUTE format('ALTER ROLE %I WITH LOGIN PASSWORD %L', role_name, password_value);
    ELSE
      EXECUTE format('CREATE ROLE %I WITH LOGIN PASSWORD %L', role_name, password_value);
    END IF;
  END LOOP;
END
$$;

-- The Supabase roles the image ships. Give the ones that authenticate a local
-- password; leave the rest alone.
DO $$
DECLARE
  password_value text;
  role_name text;
BEGIN
  SELECT app_password INTO password_value FROM local_bootstrap;
  FOREACH role_name IN ARRAY ARRAY[
    'authenticator', 'supabase_auth_admin', 'supabase_storage_admin'
  ]::text[] LOOP
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = role_name) THEN
      EXECUTE format('ALTER ROLE %I WITH LOGIN PASSWORD %L', role_name, password_value);
    END IF;
  END LOOP;

  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'authenticator') THEN
    EXECUTE 'GRANT anon, authenticated, service_role TO authenticator';
  END IF;
END
$$;

GRANT USAGE ON SCHEMA public TO anon, authenticated, service_role, authenticator;

-- One schema per service. Object names in the migrations are unqualified and
-- the service pins search_path, so these must exist before the migration runs.
CREATE SCHEMA IF NOT EXISTS account;
CREATE SCHEMA IF NOT EXISTS analytics;
CREATE SCHEMA IF NOT EXISTS audit;
CREATE SCHEMA IF NOT EXISTS identity;
CREATE SCHEMA IF NOT EXISTS job_orchestration;
CREATE SCHEMA IF NOT EXISTS ledger;
CREATE SCHEMA IF NOT EXISTS market_data;
CREATE SCHEMA IF NOT EXISTS notification;
CREATE SCHEMA IF NOT EXISTS portfolio;
CREATE SCHEMA IF NOT EXISTS reporting;
CREATE SCHEMA IF NOT EXISTS registry;

-- The registry migration ships no role or grants, so the schema belongs to the
-- service role itself.
ALTER SCHEMA registry OWNER TO registry_svc;

-- Supabase Auth and Storage own their schemas and run their own migrations.
DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'supabase_auth_admin') THEN
    EXECUTE 'CREATE SCHEMA IF NOT EXISTS auth AUTHORIZATION supabase_auth_admin';
    EXECUTE 'ALTER SCHEMA auth OWNER TO supabase_auth_admin';
    EXECUTE 'GRANT ALL ON SCHEMA auth TO supabase_auth_admin';
  END IF;
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'supabase_storage_admin') THEN
    EXECUTE 'CREATE SCHEMA IF NOT EXISTS storage AUTHORIZATION supabase_storage_admin';
    EXECUTE 'ALTER SCHEMA storage OWNER TO supabase_storage_admin';
    EXECUTE 'GRANT ALL ON SCHEMA storage TO supabase_storage_admin';
  END IF;
END
$$;
