# Performance Design — u1-walking-skeleton

Designs the concrete solutions behind
`nfr-requirements/performance-requirements.md`, constrained by
`nfr-requirements/security-requirements.md`, `nfr-requirements/scalability-requirements.md`,
`nfr-requirements/reliability-requirements.md`, `nfr-requirements/observability-requirements.md`,
and `nfr-requirements/tech-stack-decisions.md`, for the workflows in
`functional-design/functional-spec.md` (WF1 asset resolution, WF4 gateway path). Timeout and
retry budgets follow `inception/contract-design/contract-summary.md`.

## Caching architecture

Asset resolution is read-heavy and changes rarely.

- **In-process cache** in each replica, keyed by `(chainId, contractAddress)` and `(chainId, symbol)`.
- **Freshness:** a `registry_change_version` row increments on every registry write. Each replica
  polls it every **10 s** and drops the cache when the version changes.
- **Backstop TTL:** 60 s, so a missed poll cannot serve stale identity indefinitely.
- **Maximum observable staleness:** 10 s (the poll interval); the TTL only bounds a polling fault.
- **Bound:** 10 000 entries per replica, least-recently-used eviction.
- **Cold start:** resolution falls through to the schema; latency rises toward NFR2.1 but stays
  inside it for a small registry.

## Latency budgets

| Path | Budget | Notes |
|---|---|---|
| Gateway pass-through overhead | <20 ms p95 | Includes JWT validation with cached JWKS; excludes the Identity round-trip before U3 lands |
| Asset resolution (server-side) | <200 ms p95 (NFR2.1) | Cache hit ~1 ms; miss bounded by one indexed query |
| Cache hit ratio | ≥90% at steady state | Read-heavy registry; verified by the cache metric |

## Connection pooling

- sqlx pool per replica: min 5, max 20, acquire timeout 5 s, idle timeout 10 min, max lifetime
  30 min.
- Pool size = requests/s at the replica × average query duration × 1.5 buffer.

## Async and I/O

- Tokio non-blocking I/O throughout; no blocking calls on the request path.
- The registry query is a single indexed lookup; no N+1 fan-out.
- Audit emission is non-blocking with bounded retry; a failure is logged and counted.

## Timeouts on internal calls

| Call | Connect | Read | Total | Retry |
|---|---|---|---|---|
| Gateway → Identity (principal) | 2 s | 5 s | 10 s | 3, idempotent GET only, backoff + jitter |
| Gateway → Registry | 2 s | 5 s | 10 s | 3, idempotent GET only, backoff + jitter |
| Registry → Audit | 2 s | 5 s | 10 s | 3, idempotent append with key |

The 504 path in WF4 fires when the total timeout expires.

## Pagination and query limits

- Registry reads return a bounded page (default 100, max 500).
- Quarantine listing is paginated with a stable order (first seen, then id).
