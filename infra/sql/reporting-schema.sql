-- The reporting schema, provisioned for a fresh database.
--
-- Object names are unqualified because the file is applied with `search_path`
-- set to `reporting`. The authoritative definition, including every grant, is
-- `services/reporting/migrations/0001_init.up.sql`; this file exists so a
-- disposable or test database converges on the same shape.

CREATE SCHEMA IF NOT EXISTS reporting;

SET search_path TO reporting;

\ir ../../services/reporting/migrations/0001_init.up.sql
