-- Reverse of 0001_init.up.sql.
--
-- The guard is the important part: a rollback must never silently delete real
-- user records or their key references. If the schema holds any user, this
-- migration refuses and an operator decides. It succeeds on a fresh schema, which
-- is what the pipeline's up/down/re-up check exercises.

DO $$
BEGIN
    IF to_regclass('users') IS NOT NULL AND EXISTS (SELECT 1 FROM users) THEN
        RAISE EXCEPTION
            'refusing to drop the identity schema: it holds user records';
    END IF;
END
$$;

DROP TABLE IF EXISTS deletion_requests;
DROP TABLE IF EXISTS export_requests;
DROP TABLE IF EXISTS device_sessions;
DROP TABLE IF EXISTS support_access_grants;
DROP TABLE IF EXISTS consent_records;
DROP TABLE IF EXISTS user_personal;
DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS jurisdiction_rules;
DROP TABLE IF EXISTS shredding_keys;

DROP FUNCTION IF EXISTS destroy_shredding_key(bigint);
DROP FUNCTION IF EXISTS create_shredding_key(text);
DROP FUNCTION IF EXISTS unprotect(bytea, bigint);
DROP FUNCTION IF EXISTS protect(text, bigint);

-- The roles are cluster bootstrap objects, not this schema's objects: the up
-- migration creates them idempotently so a disposable database converges, and a
-- rollback leaves them in place.
