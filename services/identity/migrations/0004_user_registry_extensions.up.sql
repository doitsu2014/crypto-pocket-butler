-- U1 — user registry extensions (BR1.4, BR1.7, BR1.9, BR1.10).
--
-- Three changes:
--   1. Widen the stored status set to the four public BRD states plus the
--      transient `pending` provisioning state. Transitions stay owned by the
--      account-lifecycle unit; this migration only admits the values.
--   2. Record display name and the two profile override markers: a field the
--      user overrode is never overwritten by a later provider claim (BR1.4).
--   3. Index the exact email address the admin lookup uses (BR1.9).
--
-- Object names are unqualified on purpose: the migration runs with
-- `search_path` set to the service's own schema, and adapter tests point the
-- same migration at a disposable schema. Existing table grants cover the new
-- columns; no new role and no cross-schema access.
--
-- The widening runs in one statement: dropping and re-adding a CHECK constraint
-- validates every existing row, so a row already outside the set fails the
-- migration rather than being silently coerced.

ALTER TABLE users DROP CONSTRAINT users_status_check;

ALTER TABLE users
    ADD CONSTRAINT users_status_check
        CHECK (status IN ('pending', 'active', 'disabled', 'pending_deletion', 'deleted'));

ALTER TABLE users
    ADD COLUMN display_name text
        CHECK (display_name IS NULL OR char_length(display_name) BETWEEN 1 AND 200);

ALTER TABLE users
    ADD COLUMN display_name_overridden boolean NOT NULL DEFAULT false;

ALTER TABLE users
    ADD COLUMN email_overridden boolean NOT NULL DEFAULT false;

-- The admin lookup is an exact match on the stored address. The partial index
-- keeps anonymised rows (email IS NULL) out of the index.
CREATE INDEX users_email_idx
    ON users (email)
    WHERE email IS NOT NULL;
