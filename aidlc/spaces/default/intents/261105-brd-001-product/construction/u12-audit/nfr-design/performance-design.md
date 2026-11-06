# Audit — Performance Design

## How the append path stays inside 25 ms

| Decision | Detail |
|---|---|
| One round trip | Validation is pure: the secret detector, the required-field check, and the idempotency lookup run in memory against the prepared statement's parameters. The insert is a single statement, not a read-then-write. |
| Idempotency without a read | Uniqueness cannot live on `audit_events` itself: it is partitioned by `recordedAt`, and a unique index on a partitioned table must include the partition key, which would make the same key legal once per month. So a separate **unpartitioned** table `append_idempotency (emitting_service, idempotency_key, event_id, payload_sha256, created_at)` carries the unique index and is written in the same transaction as the event. Insert, and on a unique violation read that one row to decide `201`-with-existing (same hash) or `409` (different hash). One extra row write, no extra round trip, and the rule survives partitioning. |
| Partition pruning | Inserts name the partition by `recordedAt` month, so the insert never searches partitions. |
| No foreign keys across the trail | `actorId` is stored as a value, not a foreign key to another schema — which the data-boundary rule forbids anyway. Nothing waits on another service during an append. |
| Prepared statements | sqlx's cached prepared statements, one per query shape, so the planning cost is paid once per connection. |
| Connection budget | 8 per replica: 5 appends, 1 query, 2 exports. A waiting append fails fast at 2 s rather than queueing behind an export. |

## The session check

The check is one indexed read of `support_access_records` plus one append, measured as a single
span. Order matters: the read event is written **before** the allow is returned, so a served read
cannot exist without its record. Budget p95 60 ms breaks down as 3 ms lookup, 25 ms append,
~30 ms of headroom for Postgres jitter and TLS.

## Queries and export

- Admin queries are cursor-paginated on `(occurredAt, id)`, page size 100, with a 10-second
  statement timeout. Every query carries a **`recordedAt` range** — that is the partition key — and
  `occurredAt` narrows within it. Without the `recordedAt` predicate the planner cannot prune, and
  the 800 ms budget at 30 million rows depends on pruning, so the range is mandatory and a request
  that omits it is refused.
- Indexes, declared on the partitioned parent so future partitions inherit them: `(occurredAt, id)`
  for the cursor, `(action)` with the standard text-prefix operator class for action prefixes,
  `(target)`, `(sensitivity, occurredAt)`, and `(recordedAt)`. Five indexes over an insert-heavy
  table is the budget; a sixth needs a query that pays for it.
- Exports stream in 5 000-row batches from their own 2 connections and are written straight into
  the response body, releasing the connection between batches. Cap: 100 000 rows. Nothing is
  written to disk, so there is no file to lose and no download to route.
- No caching tier. The audit trail is written constantly and read rarely; a cache would serve
  stale security data and add a component with its own failure mode.

## Performance budgets

| Path | Budget |
|---|---|
| Append (server) | 25 ms p95 / 60 ms p99 |
| Session check (end to end) | 60 ms p95 / 120 ms p99 |
| Admin query page | 800 ms p95 |
| Export | 60 s p95 at the row cap |
| Retention sweep | 30 min, batches of 10 000 with a 200 ms pause between them |

## Sources

- `construction/u12-audit/nfr-requirements/performance-requirements.md` — NFR2.1 to NFR2.8
- `construction/u12-audit/nfr-requirements/security-requirements.md` — the role matrix
- `construction/u12-audit/nfr-requirements/scalability-requirements.md` — NFR20.5, NFR20.6, NFR20.7
- `construction/u12-audit/functional-design/functional-spec.md` — WF1 to WF6
- `inception/contract-design/contract-summary.md` — C10
