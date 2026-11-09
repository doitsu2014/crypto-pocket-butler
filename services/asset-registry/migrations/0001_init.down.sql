-- Reverse of 0001_init.up.sql. The schema itself is owned by the platform role
-- and is not dropped here; only this migration's objects are removed.

DROP TABLE IF EXISTS unmapped_observations;
DROP TABLE IF EXISTS registry_meta;
DROP TABLE IF EXISTS price_sources;
DROP TABLE IF EXISTS token_contracts;
DROP TABLE IF EXISTS native_assets;
DROP TABLE IF EXISTS assets;
DROP TABLE IF EXISTS chains;
