-- 0007 down — roll support access back to the 0006 shape.
--
-- Export-then-guard contract (NFR18.2): a rollback never destroys an audit
-- intent that has not reached C10. The guard runs before any destructive
-- statement, so a refused rollback leaves the schema exactly as it was.
--
-- Order:
--   1. Guard: raise while a `pending` or `dead` row remains. Those are the two
--      classes the drainer has not delivered; a delivered row is already in C10
--      and is safe to drop with the table.
--   2. Drop the outbox (its two partial indexes go with it).
--   3. Restore `support_grants_approval_shape` WITHOUT the 0007 clause, added
--      `NOT VALID` so a large live table is not rescanned under a rollback.
--   4. Drop the live-pair index.

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM support_audit_outbox WHERE state IN ('pending', 'dead')
    ) THEN
        RAISE EXCEPTION
            'support_audit_outbox still holds pending or dead rows; deliver them before a rollback'
            USING ERRCODE = '55000';
    END IF;
END
$$;

DROP TABLE IF EXISTS support_audit_outbox;

-- The original 0001 predicate, restored as NOT VALID. New rows are still
-- checked; existing rows are trusted because they predate 0007. The drop and
-- the add are one statement so the column is never left without its shape.
ALTER TABLE support_access_grants
    DROP CONSTRAINT support_grants_approval_shape,
    ADD CONSTRAINT support_grants_approval_shape CHECK (
        (status = 'pending_approval'
            AND approved_by IS NULL AND approved_at IS NULL AND expires_at IS NULL)
        OR (status <> 'pending_approval' AND approved_at IS NOT NULL)
        OR (status = 'revoked' AND approved_at IS NULL)
    ) NOT VALID;

DROP INDEX IF EXISTS support_grants_live_pair_idx;