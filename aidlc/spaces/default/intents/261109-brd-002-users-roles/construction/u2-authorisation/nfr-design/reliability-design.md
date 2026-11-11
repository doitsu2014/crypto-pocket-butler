# Reliability Design — u2-authorisation

**Upstream:** `reliability-requirements.md` (NFR1.1–NFR1.3, NFR18.1, NFR24.1), `functional-spec.md`, `contract-summary.md` (C3, C9).

## Pattern choices

| Pattern | Applied? | Configuration | Why |
|---|---|---|---|
| Timeout | Yes | Status read: 200 ms pool acquire, 100 ms statement; audit write: p95 50 ms with a **250 ms ceiling** (the governing value across this unit's artifacts); C4 call: 500 ms overall | Every dependency is bounded, and the bounds sum inside the C4 deadline |
| Retry with backoff | **Not applied in this unit** | — | The only read is the local repository read, whose failure is a pool or statement timeout inside the same process, not a transient remote failure; the audit write and the role write are not retried (a duplicate role change is worse than a surfaced failure). If a future remote read is added, it may retry only transport errors and 5xx, never 400/403/404/422, and only when it carries an idempotency key |
| Circuit breaker | No | — | The two dependencies fail closed on their own; a breaker would add state without changing the outcome (a denial still fails on the audit write) |
| Bulkhead | Partly | One pooled C4 client per caller; the status read uses the service's own pool | Prevents one caller's burst from exhausting another's connections |
| Fallback | Yes, bounded | The 30-second outage cache for status | The only degradation allowed; never a fallback to "allow" |
| Fail-closed | Yes | Denial audit write, role-change intent entry | A refusal or privileged change is never completed unrecorded |

## Health checks

- `/healthz` — process liveness; the Swarm update gate.
- `/readyz` — **readiness gates traffic**, so it must not fail for a condition the outage cache
  exists to survive. It reports ready when the decision is serving and either the status read
  succeeds **or** the cache has an entry for the caller inside its 30-second window; it reports
  not-ready only when the status read fails **and** the cache is cold. The audit store is
  reported as a dependency but never fails readiness: a denial failing closed is the designed
  behaviour, not an outage.

## Degradation matrix

| Dependency | Warm cache | Cold cache | User-visible effect |
|---|---|---|---|
| Status read | Serve cached status ≤ 30 s | 503 | A disable may take up to 30 s during an outage |
| Audit store | — | 503 on every denial; role change aborted | Denials fail loudly rather than pass unrecorded |
| Provider admin API | — | Role change fails with `failed` recorded | The admin retries |

## Recovery

- Nothing to restore: the unit persists nothing (NFR18.1).
- A crashed replica restarts clean and cold; its cache refills on demand.
- A recovered status source ends the outage window at the next read; entries older than 30 s are
  never served.

## Failure-mode checklist

| Question | Answer |
|---|---|
| Dependency unavailable? | Status: cache then 503. Audit: 503. Provider: surfaced failure. |
| Response time doubles? | The latency alert fires at the budget, escalation at 2× |
| Throughput exceeds capacity? | Replica count; the decision holds no state to migrate |
| Dependency returns corrupted data? | A malformed status is treated as unavailable (503), never as `active` |
| Graceful path? | Only the bounded outage cache |
| Blast radius? | One caller service's requests, never another service's — no shared state |
| Recovery? | Automatic; nothing to replay |
