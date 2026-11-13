-- U6 — the runtime jurisdiction list (BR1.6, BR1.10, W3).
--
-- Two tables: an immutable snapshot per published list version and the country
-- rules that belong to it. A published version is never edited: a new publish
-- inserts a new version, marks the previous one `superseded`, and leaves both
-- readable so every recorded decision keeps a valid citation.
--
-- Object names are unqualified on purpose: the migration runs with `search_path`
-- set to the service's own schema, and adapter tests point the same migration at
-- a disposable schema. No cross-schema grants, no Row Level Security.
--
-- The pre-sign-up gate already owns `jurisdiction_rules` (age and disclosure
-- policy). U6's versioned list therefore uses `jurisdiction_list_rules` so the
-- two concepts never collide in the same schema.
--
-- Publish writes with the existing `identity_app` role: one schema and one role
-- per service, as the project mandates.

CREATE TABLE jurisdiction_list_versions (
    version     text        PRIMARY KEY CHECK (version ~ '^[0-9A-Za-z._-]{1,64}$'),
    updated_at  timestamptz NOT NULL,
    entry_count integer     NOT NULL CHECK (entry_count > 0),
    checksum    text        NOT NULL CHECK (checksum ~ '^[0-9a-f]{64}$'),
    status      text        NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'superseded'))
);

-- At most one active version: a decision must never have two current lists.
CREATE UNIQUE INDEX jurisdiction_list_one_active
    ON jurisdiction_list_versions (status) WHERE status = 'active';

CREATE INDEX jurisdiction_list_versions_published_idx
    ON jurisdiction_list_versions (updated_at DESC);

CREATE TABLE jurisdiction_list_rules (
    version      text        NOT NULL
        REFERENCES jurisdiction_list_versions (version) ON DELETE CASCADE,
    country_code char(2)     NOT NULL CHECK (country_code ~ '^[A-Z]{2}$'),
    status       text        NOT NULL CHECK (status IN ('allowed', 'restricted')),
    source       text        NOT NULL CHECK (char_length(source) BETWEEN 1 AND 120),
    updated_at   timestamptz NOT NULL,
    PRIMARY KEY (version, country_code)
);

CREATE INDEX jurisdiction_list_rules_country_idx
    ON jurisdiction_list_rules (country_code);

DO $$
BEGIN
    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON jurisdiction_list_versions TO identity_app';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON jurisdiction_list_rules TO identity_app';

    EXECUTE 'GRANT SELECT ON jurisdiction_list_versions TO identity_readonly';
    EXECUTE 'GRANT SELECT ON jurisdiction_list_rules TO identity_readonly';

    EXECUTE 'GRANT ALL ON jurisdiction_list_versions TO identity_migrator';
    EXECUTE 'GRANT ALL ON jurisdiction_list_rules TO identity_migrator';
END
$$;
