-- 0008 up — the support agent's own-grant read (S2, NFR2.3).
--
-- `GET /v1/support-grants/mine` reads one agent's rows with no status filter,
-- newest first, bounded at twenty. `support_grants_read_idx` leads on
-- `user_id` and `support_grants_live_pair_idx` is partial, so neither serves a
-- status-free recency scan by agent. This index does, and the descending order
-- matches the read so no sort is needed.

CREATE INDEX support_grants_mine_idx
    ON support_access_grants (support_agent_id, requested_at DESC);