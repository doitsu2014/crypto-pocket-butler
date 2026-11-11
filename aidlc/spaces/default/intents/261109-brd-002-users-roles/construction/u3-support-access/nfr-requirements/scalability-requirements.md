# Scalability Requirements — u3-support-access

**Upstream:** `functional-spec.md` (W1–W5), `rules.md` (BR3.3, BR3.13), `requirements.md`
(NFR21–NFR24), `team.md` (Deployment — replicated Swarm services), the existing
`consent_audit_outbox` / `status_audit_outbox` and their drainer.

## Load projection

Support access is a staff workflow, not a user-facing one, and the projection is sized from that:

| Quantity | Projection | Basis |
|---|---|---|
| Support agents and admins | ≤ 50 agents, ≤ 20 admins at launch | The team's own support function; the product has no self-service support tier |
| Grant requests | ≤ 1,000 per day (worst case), ≤ 10 per minute at peak | Two orders of magnitude above the expected staff volume; a peak-minute number is what the concurrency rules must survive |
| Concurrent active grants | ≤ 50 | One live grant per (agent, target) pair, and a 60-minute window |
| Metadata reads | ≤ 20 per grant, ≤ 20,000 per day | An agent reads the projection repeatedly while investigating one case |
| Refusals | bounded by the same read volume | Every read either serves or refuses; a refusal is a read |
| Audit events | one per write, up to two per read | Serviced and denied reads each write one; a mutation writes one |
| Outbox rows | ≤ 25,000 per day worst case | One per mutation and per refusal; delivered rows are deleted after 24 hours |

At these volumes the unit needs no shard, cache, or read replica. The requirement is that it
stays correct and bounded at the **peak minute**, because that is where the concurrency rules
matter.

## Concurrency

| ID | Requirement |
|---|---|
| NFR20.1 | At most one live grant per (support agent, target user) is enforced by a **partial unique index** on `(support_agent_id, user_id) WHERE status IN ('pending_approval','active')`, not by a read-then-write check. Two concurrent requests cannot both succeed: the loser takes the unique violation and answers 409 (BR3.3). |
| NFR20.2 | Approval is a conditional `UPDATE ... WHERE id = :id AND status = 'pending_approval'`; zero rows affected means another approver moved the grant first and answers 409. Two concurrent approvals cannot both win, and a stale-request approval cannot race the same grant (BR3.4, BR3.5). |
| NFR20.3 | Revocation is likewise conditional on the current status; a revoke racing an expiry or a second revoke answers 409 rather than overwriting a terminal state (BR3.12). |
| NFR20.4 | The stale-pending expiry a request performs (BR3.3) runs in the same transaction as its insert and touches only that pair's row, so it cannot deadlock against another pair's request. |
| NFR20.5 | The outbox drainer is safe with **N Identity replicas**: it claims rows with `FOR UPDATE SKIP LOCKED` and a short claim lease, exactly as the existing drainer does for the consent and status outboxes, so two replicas never deliver the same row in the same pass. The advisory lock or lease is what makes the replica count a scaling knob rather than a correctness risk. |

## Data growth and indexes

| ID | Requirement |
|---|---|
| NFR20.6 | `support_access_grants` is append-mostly evidence: rows are created once and move through at most two status updates; they are never deleted **except** with the target user's account, whose erasure removes that user's grants in the same transaction as the user row (the `user_id` foreign key makes this mandatory, and BR3.2 already refuses a `pending_deletion` target, so no live support flow depends on them). At 1,000 grants/day worst case the table reaches ~365,000 rows in a year, which needs no partitioning and no archival in this release. |
| NFR20.7 | Indexes after `0007_support_access`: the primary key on `id`; the existing `support_grants_read_idx` on `(user_id, status, expires_at DESC)`, which **survives unchanged** and serves both the grant evaluation and the admin list's per-user filter (that list holds a handful of rows per user and sorts them in memory); and the partial unique index of NFR20.1. No second `(user_id, requested_at)` index is added and none is dropped. No index exists for a sweeper, because there is no sweeper. |
| NFR20.8 | `support_audit_outbox` mirrors `consent_audit_outbox`: a partial index on `(next_attempt_at) WHERE state = 'pending'` for the claim query, and a partial index on `(delivered_at) WHERE state = 'delivered'` for the 24-hour cleanup. Delivered rows are deleted after 24 hours by the drainer's existing cleanup, so the table stays at roughly one day of volume. |
| NFR20.9 | The metadata projection is assembled per request and never cached (BR3.7, NFR24.1), so there is no cache to size, warm, or invalidate — and no stale projection to serve under a different or expired grant. |

## Fan-out cost

The metadata read is Identity's only fan-out: two internal calls, issued concurrently, to services
that own the data. Scaling consequences, stated rather than hidden:

- **Identity scales with the read**: each read holds no connection across the hops and finishes at
  the 500 ms deadline, so a slow Account or Portfolio degrades the read (503) without holding pool
  connections or threads.
- **Account and Portfolio accept a new internal caller profile**: up to 20,000 reads/day at launch,
  each one indexed read by `user_id`. That is below their existing public traffic by orders of
  magnitude; the requirement is only that the endpoint be a single indexed read with no fan-out of
  its own (NFR9.2's no-callback rule keeps it so).
- **The gateway is unaffected**: the four routes are ordinary route-table entries; the gateway sees
  one request per client action, not the fan-out.

## Capacity triggers

No autoscaling rule is introduced, because the projected load does not approach the service's
capacity. The observable triggers that would say otherwise are the read-duration histogram
(NFR17.3) and the source error counters (NFR17.4): if the read p95 leaves its budget or the source
error rate rises, the unit's response is an alert and an investigation of the slow owner, not a
new cache — caching the projection is explicitly rejected in `functional-spec.md`
(brownfield Divergence 3).