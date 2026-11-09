-- Reverse of 0002_contact.up.sql. The address is derived from the token on the
-- next validated request, so dropping the columns loses nothing durable.

DROP INDEX IF EXISTS users_email_verified_idx;

ALTER TABLE users DROP COLUMN IF EXISTS email_verified;
ALTER TABLE users DROP COLUMN IF EXISTS email;
