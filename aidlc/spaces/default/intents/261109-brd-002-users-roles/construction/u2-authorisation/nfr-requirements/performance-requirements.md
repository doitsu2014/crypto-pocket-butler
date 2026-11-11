# Performance Requirements — u2-authorisation

**Upstream:** `functional-spec.md` (W1–W5, the outage-only status cache), `rules.md` (BR2.10, BR2.11), `contract-summary.md` (connect 5s, read 10s, three attempts), `requirements.md` (NFR24).

## Latency budget

Every budget is per delivery path and, where the path can deny, per outcome — a denial writes
its audit entry before it returns, so its budget includes that write.

| Path | Outcome | Target | Basis |
|---|---|---|---|
| In-process decision, status read warm or not needed (NFR2.1) | allow | p95 ≤ 2 ms, p99 ≤ 5 ms | Catalogue lookup plus at most four comparisons; **computation only** — the status read below is budgeted separately |
| Status read inside the Identity service, one per request (NFR2.2) | — | p95 ≤ 5 ms, p99 ≤ 15 ms | One indexed read in the same schema on the service's own pool; pool acquire deadline 2 s, statement timeout 2 s, **no retry** (a local pool timeout is not retried) |
| Status read over C3 from outside the Identity service, one per request (NFR2.3) | — | p95 ≤ 60 ms, p99 ≤ 150 ms | Contract's connect 5 s / read 10 s, at most three attempts on this idempotent read; the budget is the typical attempt, the deadline is the contract's |
| C4 `/internal/authorize` (NFR2.4) | allow | p95 ≤ 65 ms, p99 ≤ 150 ms | One hop plus one C3 status read |
| C4 `/internal/authorize` (NFR2.5) | deny | p95 ≤ 115 ms, p99 ≤ 270 ms | The allow path plus the denial audit write below — a denial is never returned before it is recorded |
| C4 call overall deadline (NFR2.6) | any | 500 ms wall clock | The caller's worst case: status read attempts, the audit write, and the hop; a call past this fails with 503 |
| Denial audit write (NFR2.7) | — | p95 ≤ 50 ms, p99 ≤ 120 ms | One write to the audit service; the denial is not returned until it completes (BR2.11) |
| Role change, provider write included (NFR2.8) | — | p95 ≤ 400 ms, p99 ≤ 900 ms | Two audit writes (requested, then outcome) plus one Supabase Auth admin write |

A denial through the C4 endpoint costs the allow budget plus the audit write; that is why the
endpoint carries two numbers rather than one. The 500 ms deadline is the contract a caller codes
against.

## Throughput and resource constraints

- The allow path performs **no I/O of its own** when the caller already holds the status:
  a catalogue lookup and comparisons.
- The status read happens **once per request** in normal operation (the cache is outage-only —
  see `reliability-requirements.md`).
- Every denial performs **one synchronous audit write** before returning; every role change
  performs two.
- The C4 client reuses one pooled connection set per calling service: minimum 2, maximum 10.

## Benchmarks

No benchmark suite is introduced for this unit. Build and Test measures the in-process decision
through the existing `cargo test` timing and the HTTP boundary through the identity service's
tests; a regression that exceeds these budgets is caught by the decision-duration histogram
(NFR17.2) and its alert, not by a bespoke benchmark.
