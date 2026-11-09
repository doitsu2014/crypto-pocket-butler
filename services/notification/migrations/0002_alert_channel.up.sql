-- C9: the emitter may ask for one channel. A brief asks for `email` so it does
-- not add a second in-app entry; every existing row keeps the pre-C9 behaviour
-- (`both`), which is why the column is added with that default.

ALTER TABLE alerts
    ADD COLUMN requested_channel text NOT NULL DEFAULT 'both'
        CHECK (requested_channel IN ('in_app', 'email', 'both'));
