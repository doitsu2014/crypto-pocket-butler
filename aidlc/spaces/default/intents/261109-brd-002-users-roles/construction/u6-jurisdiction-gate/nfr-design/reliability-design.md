# Reliability Design — u6-jurisdiction-gate

**Upstream:** `reliability-requirements.md` (NFR1.1–NFR18.1), `contract-summary.md` (C8, C9), `functional-spec.md`.

## Resilience patterns

| Dependency | Pattern | Configuration | On failure |
|---|---|---|---|
| Geolocation provider | Circuit breaker + timeout | 800 ms per attempt, one retry, open after 5 consecutive failures for 30 s, half-open probe of one request | Fail closed: `503`, retry path, alert |
| Audit service | Timeout + bounded retry + idempotency key | 300 ms, one retry, idempotency key = decision id | Fail closed: `503`; no decision without evidence |
| Database (list store) | Connection pool + readiness probe | Pool 2–10; readiness false while unreadable | Serve nothing; `503` |
| Peer replicas | None (stateless) | — | No effect |

## Circuit breaker state machine

| State | Trigger | Behaviour |
|---|---|---|
| Closed | Normal | Requests pass; count consecutive failures |
| Open | 5 consecutive failures | Fail fast with `503` for 30 s |
| Half-open | After 30 s | One probe; success closes, failure reopens |

## Health checks

- **Shallow**: process alive (`/healthz`).
- **Deep**: list loaded, checksum valid, provider circuit not open (`/readyz`). A deep failure removes the replica from service without killing it.

## Graceful degradation

The gate has no degraded-allow mode. The only degradation is a stated `503` with a retry path; sign-up and continued use pause rather than proceed unverified. This is a deliberate consequence of the fail-closed requirement (NFR1.3, NFR1.4).

## Recovery

- **List store**: versioned rows; the latest active version is restored on recovery. RPO ≤ 24 h, RTO ≤ 4 h (NFR18.1).
- **Provider outage**: no recovery action on the gate; the circuit closes automatically when the provider returns.
- **Audit outage**: decisions stop until the Audit service returns; queued retries are idempotent, so nothing is duplicated.

## Availability accounting

Gate availability (99.9% monthly) excludes provider-caused failures, which are tracked on the separate provider-dependency SLO (99.5% monthly). Both are reported; neither hides the other.
