# Performance Design — u2-authorisation

**Upstream:** `performance-requirements.md` (NFR2.1–NFR2.8), `functional-spec.md` (W1–W5, outage-only cache), `contract-summary.md` (timeouts), `tech-stack-decisions.md` (the in-process module and the pool limits this design depends on).

## Decisions

| Decision | Choice | Why |
|---|---|---|
| Decision execution | In-process module in the Identity `application` crate; the C4 route is a thin adapter | The computation is a catalogue lookup plus comparisons over data the caller already holds |
| Status source | **One adapter, the local repository.** U2 is embedded in the Identity service and the C4 route terminates there, so every delivery path reads status in-process from the service's own pool | A remote status adapter would describe a path no deployment can exercise: C3 exposes no by-user-id status read, and the identity the route needs comes from the JWT it already validated |
| Status read timeout | **100 ms statement timeout, 200 ms pool acquire** — short enough to leave the 500 ms C4 deadline room for the audit write | A read that outlives the deadline is a 503 anyway; the short bound is what lets the outage cache answer instead |
| Status caching | **Outage-only**, per process, TTL 30 s, cap 10 000 entries, LRU; consulted **after a failed read**, including a timed-out one | Normal requests always read status (NFR24.1). Because the read is bounded at 100 ms, a slow failure still lets the cache answer inside the deadline |
| C4 client pooling | 2–10 connections per calling service | The **callers** own this pool; the Identity service makes no outbound status call |
| Denial path | Audit write completes before the denial returns | Correctness (BR2.11) outranks the denial's latency; budgeted at NFR2.5 |
| Collections | The owner filter is returned, not applied | One decision per call; the owning service's query already touches the rows |

## Budgets as designed

| Path | Outcome | Budget |
|---|---|---|
| Decision computation | allow (status in hand) | p95 ≤ 2 ms |
| Status read, local repository | — | p95 ≤ 5 ms, hard timeout 100 ms |
| C4 endpoint | allow | p95 ≤ 30 ms |
| C4 endpoint | deny | p95 ≤ 80 ms |
| C4 endpoint | any | **500 ms deadline**: 200 ms pool acquire + 100 ms read + 250 ms audit, with margin |
| Denial audit write | — | p95 ≤ 50 ms, hard ceiling 250 ms |
| Role change | — | p95 ≤ 400 ms |

The C4 deadline is decomposed rather than asserted: the worst case is acquire + read + audit,
and the three bounds above sum below it. A read that misses its 100 ms bound falls back to the
cache or answers 503 — it never eats the deadline.

## What is deliberately absent

No remote status adapter (there is no endpoint for it and no path that needs it), no retry on
the local read (a pool timeout is not a transient remote failure), no background prefetch, and
no read-through cache on the allow path. No N+1: the decision reads one status row and writes
one (or two) audit entries.
