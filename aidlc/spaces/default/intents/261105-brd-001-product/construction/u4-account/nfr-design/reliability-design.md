# Reliability Design — u4-account

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`,
`scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`,
`tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

## Resilience patterns

| Dependency | Pattern | Configuration |
|---|---|---|
| Binance / OKX | Timeout + retry + per-venue backoff | 1 s connect / 5 s read / 30 s total; 3 attempts with jitter; 429 → `rate_limit` |
| EVM / Solana RPC | Timeout + retry + chain isolation | Same budgets; a slow chain fails its run only |
| C4 registry | Timeout + retry | 1 s / 3 s / 10 s; 3 attempts |
| C3 grant check | Timeout | 1 s / 2 s / 5 s |
| C9 notification | Timeout + retry + idempotency | 1 s / 3 s / 10 s; 3 attempts |
| C10 audit | Timeout | 1 s / 2 s / 5 s; one attempt for security events; on failure the operation that needed the record fails (support read 503; a credential or account event surfaces to the caller) and the event is logged for replay |
| C15 Auth admin | Timeout + retry + idempotency | 1 s / 3 s / 10 s; 3 attempts |
| Vault | Timeout + retry | 1 s / 3 s / 10 s; no cache of key material |

## Single-flight and lease

- A `SyncRun` carries a 60-second lease. The owning worker renews it every 20 seconds to
  `now + 60 s`; if renewal fails the worker stops rather than continue without ownership.
- The snapshot write is fenced on the run: it succeeds only with
  `WHERE run.id = :run AND run.lease_until > now()`, so a superseded run cannot write.
- A sweep every 60 seconds marks an expired run failed and releases the guard. A killed worker
  blocks no later sync beyond one lease.

## Health checks

- `GET /healthz`: process liveness only, no dependency.
- `GET /readyz`: Postgres reachable (`SELECT 1`) and Vault reachable; a draining task fails
  readiness and stops claiming runs.
- Probe interval 10 s, failure threshold 3. A task that is unready takes no traffic and its
  rolling update is gated on readiness.

## Graceful degradation

- A failed sync keeps the last snapshot and labels it; holdings older than 24 hours are reported
  stale, and an alert fires at 12 hours.
- A failing provider does not affect other accounts or chains.
- A registry outage fails the run rather than fabricating holdings.
- An audit failure fails the operation that needed the record (support read 503; a credential or
  account event surfaces to the caller) and is logged for replay; it is never silently dropped.

## Backup and recovery

- Supabase backup regime; RPO ≤ 24 h, RTO ≤ 4 h; a restore rehearsal before launch.
- Imports are idempotent by `dedupeKey`, so a retry after a restore cannot duplicate rows.

NFR references: NFR1.1, NFR1.2, NFR4.2, NFR8.1, NFR18.1.
