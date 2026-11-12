-- 0008 down — drop the own-grant recency index.
--
-- The index backs one read path and destroys nothing; dropping it leaves the
-- table and every row exactly as they were.

DROP INDEX IF EXISTS support_grants_mine_idx;