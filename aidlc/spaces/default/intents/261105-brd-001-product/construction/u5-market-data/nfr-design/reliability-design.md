# Reliability Design — u5-market-data

Inputs: `nfr-requirements/reliability-requirements.md`, `functional-design/rules.md`, and C4/C6.

## Dependency handling

| Dependency | Pattern | Configuration |
|---|---|---|
| Price sources | Timeout + retry + per-source bucket + circuit breaker | 1 s connect / 2 s read / 5 s total; 2 attempts with jitter; 429 → `rate_limit`; 3 consecutive failures open the circuit for 5 min |
| C4 on the read path | Timeout, no retry | 150 ms total; on failure degrade immediately to stale (NFR1.11) |
| C4 in a refresh run | Timeout + retry | 1 s / 3 s / 10 s; 3 attempts; failure fails the run |
| Postgres | Pool + timeout | 12 API / 8 worker; acquire timeout 2 s |
| C10 audit | Timeout | 1 s / 2 s / 5 s; a failed write is logged and retried with the next event |

## Observation integrity

- A reading is accepted only when its price is greater than zero and its `observed_at` is not more
  than 300 seconds ahead of `received_at`; otherwise it is stored as rejected and never selected.
- Prices and rates are parsed and stored as `rust_decimal` decimals; no float path exists.

## Run deadlines and circuit breaking

- Each run has a hard deadline: top100 180 s, tail 600 s. On deadline the run stops, records what
  it priced, and ends `succeeded` (or `failed` if nothing was priced).
- A source with 3 consecutive transient failures opens a circuit for 5 minutes; its assets are
  served from the previous observation and the run continues.

## Jobs and leases

- `RefreshRun` status `running` with `lease_until = now + 300 s`, renewed every 60 s by the job.
- A sweep every 60 s marks expired running rows `failed` and releases that job's guard.
- The two jobs hold independent leases; a killed worker blocks its job for at most one lease.

## Degraded modes

- All sources failing: the read path serves the newest accepted observation as `stale` within 24 h;
  beyond that, `unpriced`.
- C4 unavailable on the read path: serve each asset's newest observation as `stale`, skipping
  priority; `unpriced` when none (NFR1.11).
- C4 unavailable during a refresh: the run fails and writes an audit event; previous observations
  keep serving.
- Postgres unavailable: `getPrices`/`getFxRate` return 503; `/readyz` fails.

## Health checks

- `/healthz` liveness only; `/readyz` checks Postgres. Probe every 10 s, 3 failures marks the task
  unhealthy; the Swarm rolling update is gated on the health check.

## Backup and recovery

- Supabase backup regime; RPO ≤ 24 h, RTO ≤ 4 h; a restore rehearsal before launch.
- A restore loses at most one day of observations; the next refresh repopulates them.

NFR references: NFR1.1–NFR1.11, NFR2.5.
