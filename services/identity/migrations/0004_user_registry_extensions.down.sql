-- Reverse of 0004_user_registry_extensions.up.sql.
--
-- The older application only understands pending/active/deleted. Rows in the
-- two newer states must therefore be mapped back to `active` BEFORE the
-- narrower check is re-imposed; otherwise the rollback would fail on any live
-- disabled or pending-deletion row and could not be applied at all. This is a
-- deliberate, lossy downgrade: the pre-0004 application has no way to represent
-- the newer states, and leaving the check wide would let it write a value it
-- cannot read.

UPDATE users
SET status = 'active', updated_at = now()
WHERE status IN ('disabled', 'pending_deletion');

DROP INDEX IF EXISTS users_email_idx;

ALTER TABLE users DROP COLUMN IF EXISTS email_overridden;
ALTER TABLE users DROP COLUMN IF EXISTS display_name_overridden;
ALTER TABLE users DROP COLUMN IF EXISTS display_name;

ALTER TABLE users DROP CONSTRAINT users_status_check;

ALTER TABLE users
    ADD CONSTRAINT users_status_check
        CHECK (status IN ('pending', 'active', 'deleted'));
