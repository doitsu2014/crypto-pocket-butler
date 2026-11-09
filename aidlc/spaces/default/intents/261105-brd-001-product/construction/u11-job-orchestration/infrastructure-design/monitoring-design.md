# Monitoring Design — u11-job-orchestration

Inputs: `nfr-design/observability-design.md`, `infrastructure-specification.md`.

## SLIs and SLOs

| SLI | SLO | Window |
|---|---|---|
| Operator read availability (2xx/4xx without 5xx) | 99.9% | 30 days |
| Operator read latency `GET /v1/jobs*` p95 | ≤ 300 ms | 7 days |
| Backfill/manual latency p95 | ≤ 500 ms | 7 days |
| Run success ratio per job (excluding skipped) | 99% | 7 days |
| Tick p95 duration | ≤ 30 s | 24 h |

The heartbeat is not an SLO row: its 180 s dead-man threshold is the alert below.

## Metrics

Exactly `nfr-design/observability-design.md`: heartbeat age, tick duration histogram, run outcomes,
job failures, target duration, unrecovered windows, prune timestamp, inbound request duration and
status.

## Alerts

| Alert | Expression | For | Severity |
|---|---|---|---|
| HeartbeatStale | `absent(job_orchestration_heartbeat_age_seconds) or job_orchestration_heartbeat_age_seconds > 180` | 90s | P2 on-call |
| JobRunAbandoned | `increase(job_orchestration_runs_total{outcome="abandoned"}[10m]) > 0` | 0 | P2 |
| JobStillFailing | `increase(job_orchestration_job_failures_total[5m]) > 0 unless on(job) increase(job_orchestration_runs_total{outcome="succeeded"}[5m]) > 0` | 300s | P3 async |
| WindowsUnrecovered | `increase(job_orchestration_windows_unrecovered_total[1h]) > 0` | 0 | P3 |
| PruneMissed | `absent(job_orchestration_last_prune_timestamp)` (grace until 24 h after deploy) OR `time() - job_orchestration_last_prune_timestamp > 129600` | 1h | P3 |

Before the first prune the service exports the deployment instant from `prune_state` (created by
the migration), so the staleness branch evaluates immediately; the absent-only branch covers a
service that never exports the gauge.

Every metric also appears on the dashboard even when it does not page; the failure counter is the
five-minute visibility signal independent of paging.

## Logs and traces

JSON logs with `service`, `correlationId`, `job`, `window`, `attempt`, `trigger`, `outcome`,
`durationMs`; no token or body. W3C `traceparent` forwarded to providers; Grafana datasources from
Loki/Tempo.

## Dashboard

Heartbeat age, tick p95, outcomes per job, target latency, failures, abandoned runs, unrecovered
windows, prune age.
