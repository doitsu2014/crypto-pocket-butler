# Reliability Design — u10-analytics

Inputs: `nfr-requirements/reliability-requirements.md`.

| Dependency | Pattern | Configuration |
|---|---|---|
| C3 consent | Timeout, no retry on ingest | 80 ms; on failure 503, never accept unconsented |
| Postgres | Pool + timeout | acquire 2 s; ingest 503 on failure |
| Vault salt | Startup read + per-request cache | 5 min cache; a missing salt fails readiness |
| C10 audit | Timeout | 1 s/2 s/5 s; failure logged and counted, never blocks ingest |
| Aggregation | Lease + retry | one run, lease 300 s, one retry after 10 min |
| Erasure | Idempotent delete | re-run from the start; 204 when nothing remains |

- Idempotency: events have no natural key; duplicates are accepted and counted.
- Health: `/healthz` liveness; `/readyz` checks Postgres. The salt is read lazily and a missing
  salt fails ingest with 503 while flag reads still work (their bucket is a hash of the subject,
  not of userRef).
- Backup: Supabase regime, RPO ≤24 h, RTO ≤4 h.

NFR references: NFR1.1–NFR1.7.
