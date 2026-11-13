-- Reverse of 0006_account_lifecycle.up.sql.
--
-- The guard is the important part: a rollback must not strand a deletion that
-- was accepted under the new window. If any open request exists the rollback
-- refuses and an operator decides — the request is either restored (cancelled)
-- or erased before the release is rolled back.
--
-- The reverse restores `0001`'s status-keyed unique index and removes every
-- object and column this migration added. Rows already cancelled or erased keep
-- their history in the columns that survive: `0001`'s `status` and
-- `completed_at` still describe the outcome, so the downgrade is lossy only for
-- the deadline itself.

DO $$
BEGIN
    IF to_regclass('deletion_requests') IS NOT NULL
       AND EXISTS (
           SELECT 1 FROM deletion_requests
           WHERE cancelled_at IS NULL AND erased_at IS NULL
       )
    THEN
        RAISE EXCEPTION
            'refusing to roll back 0006: open deletion requests exist';
    END IF;
END
$$;

DROP INDEX IF EXISTS deletion_requests_due_idx;
DROP INDEX IF EXISTS deletion_requests_one_open_per_user;
CREATE UNIQUE INDEX IF NOT EXISTS deletion_requests_one_active_per_user
    ON deletion_requests (user_id) WHERE status IN ('requested', 'in_progress');

DROP TABLE IF EXISTS status_audit_outbox;
DROP TABLE IF EXISTS status_transitions;
DROP FUNCTION IF EXISTS status_transitions_append_only();
DROP SEQUENCE IF EXISTS status_transitions_seq_seq;

DROP INDEX IF EXISTS users_erased_subject_idx;
ALTER TABLE users DROP CONSTRAINT IF EXISTS users_erased_subject_shape;
ALTER TABLE users DROP COLUMN IF EXISTS erased_subject_hash;

ALTER TABLE deletion_requests DROP CONSTRAINT IF EXISTS deletion_requests_window_shape;
ALTER TABLE deletion_requests DROP CONSTRAINT IF EXISTS deletion_requests_erased_shape;
ALTER TABLE deletion_requests DROP CONSTRAINT IF EXISTS deletion_requests_attempt_count_check;
ALTER TABLE deletion_requests DROP COLUMN IF EXISTS next_attempt_at;
ALTER TABLE deletion_requests DROP COLUMN IF EXISTS attempt_count;
ALTER TABLE deletion_requests DROP COLUMN IF EXISTS erased_at;
ALTER TABLE deletion_requests DROP COLUMN IF EXISTS cancelled_at;
ALTER TABLE deletion_requests DROP COLUMN IF EXISTS recoverable_until;
