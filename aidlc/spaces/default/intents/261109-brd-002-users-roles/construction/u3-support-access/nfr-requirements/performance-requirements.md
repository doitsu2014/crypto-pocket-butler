# Performance Requirements — u3-support-access

**Upstream:** `functional-spec.md` (W1–W5), `rules.md` (BR3.1–BR3.13), `contract-summary.md`
(C1/C2 error envelope and timeouts, C5, C10), `requirements.md` (NFR21–NFR26),
`technology-stack.md` (Identity service, Axum + sqlx).

Every budget below is the Identity service's own time for one request: pool acquisition,
statements, and any synchronous audit append or fan-out hop it performs before responding. The
gateway adds its own hop, which C1/C2 already budget; the numbers here are what the service must
hold.

## Latency budget

| Path | Outcome | Target | Basis |
|---|---|---|---|
| W1 grant request (NFR2.1) | accepted | p95 ≤ 80 ms, p99 ≤ 180 ms | Role from validated claims, one target-user read, one insert plus its `support_audit_outbox` row in a single transaction; no external hop |
| W1 grant request (NFR2.2) | refused | p95 ≤ 60 ms, p99 ≤ 150 ms | The refusal's own audit transaction plus the one insert; a 403 on role fails before any read |
| W2 approve / W3 revoke (NFR2.3) | accepted | p95 ≤ 80 ms, p99 ≤ 180 ms | Role check, one grant read, one conditional `UPDATE` plus its outbox row in one transaction |
| W2 / W3 (NFR2.4) | refused | p95 ≤ 60 ms, p99 ≤ 150 ms | The conditional update affecting zero rows, or the stale-request branch, plus the refusal's own audit transaction |
| W4 grant evaluation (NFR2.5) | — | p95 ≤ 15 ms, p99 ≤ 40 ms | One indexed read of `support_access_grants` on the service's own pool, plus the target-user row for `displayName` / `emailVerified` |
| W4 one metadata source (NFR2.6) | — | p95 ≤ 60 ms, p99 ≤ 80 ms | One internal GET to Account or Portfolio: connect 1 s, **request timeout 120 ms, one attempt**. The timeout sits **above** the source's p99, so a normal read is never cut off by it; the sources are reads with no idempotency key, so the contract's retry rule does not apply and a retry would not fit the budget below |
| W4 metadata read, both sources answering (NFR2.7) | served | p95 ≤ 150 ms, p99 ≤ 300 ms | Grant evaluation (p99 40) + the slower source (p99 80) + the audit append (p99 150) = 270 ms p99; the two source hops are issued concurrently |
| W4 metadata read (NFR2.8) | any refusal | p95 ≤ 60 ms, p99 ≤ 150 ms | Grant evaluation answers before any source is called, so a pending, revoked, expired or foreign grant costs no hop |
| W4 audit append (NFR2.9) | — | p95 ≤ 50 ms, p99 ≤ 150 ms | The existing `HttpAuditClient`: connect 1 s, request timeout 200 ms, **one attempt**; the timeout sits above the p99, and the projection is not returned until the append completes (BR3.13) |
| W4 overall deadline (NFR2.10) | any | 500 ms wall clock | Grant evaluation, both hops, and the audit append, with each hop's own timeout well inside it; a request past this answers 503 `metadata-source-unavailable` rather than returning late |
| W5 support outbox pass (NFR2.11) | — | ≤ 10 s worst case for a full batch of 5 rows; typically < 500 ms | A dedicated client for this sink (1 s connect, **2 s request, one attempt per row**), claimed with the shipped drainer's lease; 5 × 2 s stays inside the 30 s claim lease. Retries happen across passes via `next_attempt_at`, never inside a pass. The consent and status outboxes keep their shipped 5 s/10 s client and batch of 20 unchanged |

A refusal is cheaper than an acceptance on the write paths and cheaper than a served read on the
metadata path; that is why each path carries two numbers. The 500 ms deadline is the number a
caller codes against, and it matches the C4 endpoint's own overall deadline so the Identity
service has one internal-call convention rather than two.

## Statement and pool bounds

- Pool: acquire timeout 5 s, minimum 5 / configured maximum connections, idle 600 s, max lifetime
  1800 s (`services/identity/infrastructure/src/db.rs`).
- The pool's default `statement_timeout` is 10 s. Every support **mutation** transaction sets
  `SET LOCAL statement_timeout = '2s'` so a lock wait cannot hold a request far past its p99.
- The metadata read's statements are each a **one-statement transaction**: `BEGIN; SET LOCAL
  statement_timeout = '500ms'; <the lookup>; COMMIT;`. `SET LOCAL` has no effect outside a
  transaction, so this is the only way to bound them below the pool default. The transaction
  commits — and the connection is released — before either source is called, so no connection is
  held across the fan-out; the two lookups are bounded independently of the source timeouts.
- No support path takes a row lock for longer than one statement: the request's stale-pending
  expiry and insert, the approve/revoke conditional `UPDATE`, and the refusal's outbox insert are
  each a single statement or a short pair inside one transaction.

## Throughput and resource constraints

- Support traffic is staff traffic, not user traffic: the projected volume is in
  `scalability-requirements.md`. Nothing in the design shards, caches, or pre-computes for it.
- The metadata read performs **two internal hops and one audit append**, and nothing else. It
  holds no connection across the hops: the grant read and the user read finish before the fan-out
  begins, so a slow source does not hold a pool connection.
- The facade of the fan-out is bounded: at most two in-flight source calls per request, both
  cancelled at the 500 ms deadline.
- The drainer is the only background loop this unit adds, and it reuses the existing one rather
  than starting a second (see `tech-stack-decisions.md`).

## Benchmarks

No benchmark suite is introduced. Build and Test measures the write paths through the existing
`cargo test` HTTP boundary tests and the read path through an adapter test with a stubbed source;
the budgets above are guarded in production by the read-duration histogram (NFR17.3) and its
alert, not by a bespoke harness.