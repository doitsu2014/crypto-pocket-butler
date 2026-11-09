-- pgsodium support for the Identity service's per-subject keys.
--
-- The supabase/postgres image installs pgsodium and generates the root key. The
-- extension still restricts bigint-key operations to its own roles, so the
-- superuser that owns Identity's SECURITY DEFINER functions needs membership and
-- table rights before create_shredding_key / protect / unprotect can run.
\set ON_ERROR_STOP on

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pgsodium') THEN
    EXECUTE 'CREATE EXTENSION pgsodium';
  END IF;

  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'pgsodium_keyiduser') THEN
    EXECUTE 'GRANT pgsodium_keyiduser TO postgres';
  END IF;
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'pgsodium_keymaker') THEN
    EXECUTE 'GRANT pgsodium_keymaker TO postgres';
  END IF;
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'pgsodium_keyholder') THEN
    EXECUTE 'GRANT pgsodium_keyholder TO postgres';
  END IF;

  IF to_regclass('pgsodium.key') IS NOT NULL THEN
    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON pgsodium.key TO postgres';
  END IF;
END
$$;

DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
    WHERE n.nspname = 'pgsodium'
  ) THEN
    EXECUTE 'GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA pgsodium TO postgres';
  END IF;
END
$$;
