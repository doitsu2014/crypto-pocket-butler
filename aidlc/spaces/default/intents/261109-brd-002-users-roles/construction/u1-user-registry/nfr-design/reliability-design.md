# Reliability Design — u1-user-registry

**Upstream:** `reliability-requirements.md` (NFR1.1–NFR24.1), `contract-summary.md` (C3, C8, C9), `functional-spec.md`.

## Resilience patterns

| Dependency | Pattern | Configuration | On failure |
|---|---|---|---|
| Postgres | Pool + readiness probe | Pool 2–10; readiness false when unreachable | `503`; no partial write |
| Jurisdiction gate (C8) | Timeout + fail-closed | Inside its 1.5 s cap | Refuse provisioning; nothing stored |
| Audit service (C9) | Timeout + bounded retry + idempotency key | 300 ms, one retry | Surface the failure; do not report success |

## Provisioning transaction

1. Gate clearance (C8).
2. Begin transaction: insert `shredding_keys`, then `users`.
3. Commit; then audit.
A failure before commit rolls back both rows; a retry re-runs the lookup and returns the existing record (idempotent).

## Health checks

- **Shallow:** `/healthz`.
- **Deep:** `/readyz` — database reachable and migrations applied. Dependency state (gate, audit) is metrics-only, not readiness, so one dependency's outage cannot fail a rolling update.

## Status freshness

NFR24.1: the stored status is read on every resolution; a change by u5 takes effect at the user's next request. Role comes from the token and changes at token refresh.

## Recovery

RPO ≤ 24 h, RTO ≤ 4 h for the identity schema (Supabase Postgres backup/restore).
