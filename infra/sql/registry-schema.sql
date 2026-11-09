-- Provisioning for the Asset Registry's own schema and role.
-- Applied once by the platform/database administrator, never by the service.
--
-- The role name is a deployment parameter; replace `registry_service` before
-- running, or set it with `psql -v role=...`.
CREATE SCHEMA IF NOT EXISTS registry;

-- Replace the placeholder role with the deployment's registry role.
-- GRANT USAGE, CREATE ON SCHEMA registry TO registry_service;
-- ALTER ROLE registry_service SET search_path TO registry;
-- No cross-schema grants and no Row Level Security: access is bounded by the
-- role and the schema, and other services read registry data over HTTP only.
