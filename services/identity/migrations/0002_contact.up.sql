-- C3 contact support: Identity stores the address Supabase Auth proved for the
-- user, so a service caller (Notification at delivery time) can read the verified
-- address without holding it itself. The address is personal data: it is cleared
-- by the deletion anonymisation and never logged.

ALTER TABLE users
    ADD COLUMN email text
        CHECK (email IS NULL OR (char_length(email) BETWEEN 3 AND 320));

ALTER TABLE users
    ADD COLUMN email_verified boolean NOT NULL DEFAULT false;

-- An existing user's address is learned from the next validated token; until
-- then C3 answers verified=false.
CREATE INDEX users_email_verified_idx
    ON users (email_verified)
    WHERE email IS NOT NULL;
