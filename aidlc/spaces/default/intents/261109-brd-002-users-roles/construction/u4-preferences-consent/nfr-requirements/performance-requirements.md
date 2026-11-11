# Performance Requirements — u4-preferences-consent

**Upstream:** `functional-spec.md` (W0–W5), `rules.md` (BR4.1–BR4.11), `requirements.md` (NFR26), `contract-summary.md` (C6, C9).

## Latency budget

| Path | Target | Basis |
|---|---|---|
| Preferences read, before the first write (NFR2.1) | p95 ≤ 20 ms | No query: the defaults are computed in the service |
| Preferences read, after a write (NFR2.2) | p95 ≤ 40 ms, p99 ≤ 100 ms | One indexed row read by user id |
| Preferences write, partial (NFR2.3) | p95 ≤ 50 ms, p99 ≤ 120 ms | One upsert on the user's row |
| Consent-state query, W0 (NFR2.4) | p95 ≤ 40 ms, p99 ≤ 100 ms | One grouped read: the newest event per (user, kind). **Targeted at the C6 amendment** (consent-state endpoint) that `reliability-requirements.md` records as a blocking precondition |
| Consent grant (NFR2.5) | p95 ≤ 50 ms, p99 ≤ 120 ms | One insert; **grants are not audited** (no audit write in the budget) |
| Consent withdrawal (NFR2.6) | p95 ≤ 80 ms, p99 ≤ 180 ms | One row-locked read for the newest grant's version, one insert, and one **local** outbox insert in the same transaction — no outbound C9 call on the request path (NFR1.5) |
| Consent history read (NFR2.7) | p95 ≤ 60 ms for the newest 100 events | One indexed read ordered by `occurredAt`, `seq` |

The gate (W0) is the path that matters: it sits in front of every source connection, so it is
budgeted tighter than the writes that feed it.

## Throughput and resource constraints

- Every path is a single indexed read or write on the Identity service's own pool; no join across
  users, no scan.
- The state query reads at most four rows per user (one per kind), never the full history.
- The history read is bounded to the newest 100 events per page; deeper history is paged.

## Benchmarks

No benchmark suite is introduced. The state-query and write budgets are guarded by the decision
histograms in `observability-requirements.md`; Build and Test runs the suites, and a regression
shows up as latency on the gate path rather than as a benchmark result.
