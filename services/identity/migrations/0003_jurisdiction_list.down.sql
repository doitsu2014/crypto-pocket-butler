-- Reverse of 0003_jurisdiction_list.up.sql.
--
-- The list is evidence: a decision cites its version. Dropping the tables is
-- only safe once no decision depends on them, so the operator decides by
-- applying this migration deliberately. The roles are cluster bootstrap objects
-- and stay in place.

DROP TABLE IF EXISTS jurisdiction_list_rules;
DROP TABLE IF EXISTS jurisdiction_list_versions;
