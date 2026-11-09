-- Asset Registry schema, owned by the `registry` Postgres role.
--
-- Object names are unqualified on purpose: the service connection sets
-- `search_path` to its own schema, and integration tests point the same
-- migrations at a disposable schema. No cross-schema grants, no RLS.

CREATE TABLE chains (
    id              text PRIMARY KEY,
    name            text NOT NULL UNIQUE,
    family          text NOT NULL CHECK (family IN ('evm', 'bnb', 'solana')),
    launch_coverage boolean NOT NULL DEFAULT true
);

CREATE TABLE assets (
    id         text PRIMARY KEY,
    symbol     text NOT NULL,
    name       text NOT NULL,
    decimals   smallint NOT NULL CHECK (decimals BETWEEN 0 AND 36),
    canonical  boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX assets_symbol_idx ON assets (symbol);

-- BR1.1: a native asset resolves by chain plus symbol, never by symbol alone.
CREATE TABLE native_assets (
    chain_id text NOT NULL REFERENCES chains (id),
    asset_id text NOT NULL REFERENCES assets (id),
    symbol   text NOT NULL,
    PRIMARY KEY (chain_id, asset_id),
    UNIQUE (chain_id, symbol)
);

CREATE TABLE token_contracts (
    id                    text PRIMARY KEY,
    chain_id              text NOT NULL REFERENCES chains (id),
    contract_address      text NOT NULL,
    contract_address_key  text NOT NULL,
    asset_id              text NOT NULL REFERENCES assets (id),
    decimals              smallint NOT NULL CHECK (decimals BETWEEN 0 AND 36),
    created_at            timestamptz NOT NULL DEFAULT now(),
    -- BR1.1: one chain-plus-contract observation resolves to exactly one asset.
    UNIQUE (chain_id, contract_address_key)
);

CREATE INDEX token_contracts_asset_idx ON token_contracts (asset_id);

CREATE TABLE price_sources (
    id          text PRIMARY KEY,
    name        text NOT NULL UNIQUE,
    priority    integer NOT NULL CHECK (priority >= 1),
    licence_ref text,
    enabled     boolean NOT NULL DEFAULT false,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);

-- BR2.1: at most one enabled source holds a given priority.
CREATE UNIQUE INDEX price_sources_enabled_priority_idx
    ON price_sources (priority)
    WHERE enabled;

CREATE TABLE unmapped_observations (
    id                   text PRIMARY KEY,
    observed_symbol      text NOT NULL,
    chain_id             text NOT NULL REFERENCES chains (id),
    contract_address     text,
    contract_address_key text,
    first_seen_at        timestamptz NOT NULL,
    status               text NOT NULL DEFAULT 'quarantined'
        CHECK (status IN ('quarantined', 'resolved', 'ignored')),
    resolved_asset_id    text REFERENCES assets (id),
    ignore_reason        text,
    updated_at           timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT unmapped_observations_status_shape CHECK (
        (status = 'quarantined' AND resolved_asset_id IS NULL AND ignore_reason IS NULL)
        OR (status = 'resolved' AND resolved_asset_id IS NOT NULL AND ignore_reason IS NULL)
        OR (status = 'ignored' AND resolved_asset_id IS NULL AND ignore_reason IS NOT NULL)
    )
);

-- BR1.3: the same observation is never quarantined twice.
CREATE UNIQUE INDEX unmapped_observations_token_idx
    ON unmapped_observations (
        chain_id,
        COALESCE(contract_address_key, ''),
        observed_symbol
    );

CREATE TABLE registry_meta (
    id         boolean PRIMARY KEY DEFAULT true CHECK (id),
    version    bigint NOT NULL DEFAULT 0,
    updated_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO registry_meta (id, version) VALUES (true, 0);

-- Launch coverage. Everything else is out of coverage.
INSERT INTO chains (id, name, family, launch_coverage) VALUES
    ('ethereum', 'Ethereum', 'evm', true),
    ('bnb-smart-chain', 'BNB Smart Chain', 'bnb', true),
    ('solana', 'Solana', 'solana', true);

-- Native assets are the only symbols that resolve without a contract address.
INSERT INTO assets (id, symbol, name, decimals, canonical) VALUES
    ('native-eth', 'ETH', 'Ether', 18, true),
    ('native-bnb', 'BNB', 'BNB', 18, true),
    ('native-sol', 'SOL', 'Solana', 9, true);

INSERT INTO native_assets (chain_id, asset_id, symbol) VALUES
    ('ethereum', 'native-eth', 'ETH'),
    ('bnb-smart-chain', 'native-bnb', 'BNB'),
    ('solana', 'native-sol', 'SOL');
