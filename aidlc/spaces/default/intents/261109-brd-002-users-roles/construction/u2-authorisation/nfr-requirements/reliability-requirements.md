# Reliability Requirements — u2-authorisation

**Upstream:** `functional-spec.md` (status lookup, fail-closed branch), `rules.md` (BR2.10, BR2.11), `contract-summary.md` (C3, C9, timeouts, retries), `requirements.md` (NFR24).

## What the decision depends on

The decision has **two runtime dependencies**, and only these:

1. **The status read** — the caller's status, once per request.
   - *Inside the Identity service:* a repository read on the service's own pool (pool acquire
     deadline 2 s, statement timeout 2 s, no retry). "u1 unavailable" here means the Identity
     database is unavailable.
   - *Outside the Identity service:* the C3 HTTP call, connect 5 s / read 10 s, at most three
     attempts with exponential backoff and jitter — the only retried dependency.
2. **The audit write** — one per denial, two per role change, synchronous (BR2.11).

The decision's own computation has no dependency.

## Availability

| ID | Requirement |
|---|---|
| NFR1.1 | The decision is available whenever its caller can reach the status read and, for a denial, the audit store. The end-to-end target is the caller service's, 99.9% monthly (BRD-012 NFR-AVL-01). |
| NFR1.2 | **The status cache is outage-only.** In normal operation every request reads the status; when the read fails, a per-process cache of at most 30 seconds answers so a registry blip does not take every caller down. When the cache is cold, the decision fails closed with 503 and the failure is logged and counted. |
| NFR1.3 | The status dependency is reported on `/readyz`; a sustained read failure raises an alert, not a restart loop (`/healthz` stays the Swarm update gate). |

## Fault tolerance

| Failure | Effect | Behaviour |
|---|---|---|
| Status read slow or down, cache warm | Request continues on cached status | Staleness is bounded at 30 seconds |
| Status read down, cache cold | 503 | Fail closed; never guess |
| Audit store down | 503 on every denial; a role change is not attempted | BR2.11, BR2.10 |
| Supabase Auth admin API down | Role change fails with outcome `failed` recorded | Retry by the admin |
| Identity database down | In-process callers 503 at the status read; C3 callers 503 at the hop | Automatic on recovery |

Retries apply **only** to the C3 status read (idempotent, three attempts per the contract). The
audit write and the provider role write are not retried automatically; a failure is surfaced.

## Graceful degradation

The only degradation allowed is the 30-second outage cache. There is no "allow when in doubt"
mode: every uncertain path denies or fails, never permits.

## Freshness

| ID | Requirement |
|---|---|
| NFR24.1 | Role and status are recomputed from the signed token and the status record on **every** request. Staleness exists only in the NFR1.2 outage window, so a disable takes effect on the caller's next request in normal operation and within 30 seconds at worst during a status-read outage. |

## Recovery

| ID | Requirement |
|---|---|
| NFR18.1 | The unit persists nothing, so recovery is the caller service's. Role claims live in Supabase Auth and audit entries in the audit store, each with the platform's own backup (RPO ≤ 24 h, RTO ≤ 4 h). |
