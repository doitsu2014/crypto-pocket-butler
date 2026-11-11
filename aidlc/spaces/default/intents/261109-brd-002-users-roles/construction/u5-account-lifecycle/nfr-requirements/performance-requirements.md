# Performance Requirements — u5-account-lifecycle

**Upstream:** `functional-spec.md` (W1–W5), `rules.md` (BR5.1–BR5.10), `requirements.md` (NFR24).

## Latency budget

| Path | Target | Basis |
|---|---|---|
| Admin disable/restore (NFR2.1) | p95 ≤ 60 ms, p99 ≤ 140 ms | One status update, one transition insert, one outbox insert — one transaction |
| Deletion request (NFR2.2) | p95 ≤ 60 ms, p99 ≤ 140 ms | One `deletion_requests` upsert plus the same three writes |
| Owner restore (NFR2.3) | p95 ≤ 60 ms, p99 ≤ 140 ms | Same shape |
| Status read for u2's decision (NFR2.4) | not on this unit's path | u2 reads the `users` column it already reads; u5 adds no hop |
| Expiry sweep, per account (NFR2.5) | p95 ≤ 400 ms, hard ceiling 3 s inside the lease | Anonymisation plus the cascade; bounded by `lease_until` |
| Sweep batch of 100 (NFR2.6) | ≤ 60 s | 100 × 400 ms plus overhead fits the 60 s interval; a batch that cannot finish inside it is a sizing signal (100 × the 3 s ceiling would take 300 s, which is why the p95 — not the ceiling — sizes the batch) |

The user-facing paths are three writes in one round trip. The sweep is the only long path and it is
off the request path entirely.

## Throughput and resource constraints

- Transitions are rare (a handful per account lifetime); the sweep is the only bulk operation.
- The sweep claims with the existing lease (`lease_owner`, `lease_until`) rather than a held row
  lock, so it never blocks a user's restore.
- The outbox drain is shared with the same worker loop; each row is one bound C9 call.

## Benchmarks

No benchmark suite. The sweep's per-account duration and batch duration are emitted as histograms;
a batch that overruns the interval is visible rather than assumed.
