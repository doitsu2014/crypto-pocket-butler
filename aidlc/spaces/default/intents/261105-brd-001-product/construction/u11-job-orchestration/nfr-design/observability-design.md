# Observability Design — u11-job-orchestration

Sources: `nfr-requirements/observability-requirements.md`.

## Metrics (`/metrics`)

| Metric | Type | Labels |
|---|---|---|
| `job_orchestration_health` | gauge | service |
| `job_orchestration_heartbeat_age_seconds` | gauge | service |
| `job_orchestration_tick_duration_seconds` | histogram | service; buckets 1/5/10/30/60 |
| `job_orchestration_runs_total` | counter | job, outcome (succeeded, failed, skipped, abandoned) |
| `job_orchestration_job_failures_total` | counter | job (failed attempts and direct 4xx abandonments) |
| `job_orchestration_target_duration_seconds` | histogram | job |
| `job_orchestration_windows_unrecovered_total` | counter | job |
| `job_orchestration_last_prune_timestamp` | gauge | service |
| `http_requests_total` | counter | status_class, route |
| `http_request_duration_seconds` | histogram | route, status_class; buckets 10/50/100/300/1000 ms |

## Logs and traces

Structured JSON: `service`, `correlationId`, `job`, `window`, `attempt`, `trigger`, `outcome`,
`durationMs`. Never a token, response body or target URL credentials. The W3C `traceparent` header
is forwarded to the target; the span is named `job.call {job}`.

## Alerts

| Alert | Expression | For | Severity / route |
|---|---|---|---|
| HeartbeatStale | `absent(job_orchestration_heartbeat_age_seconds) or job_orchestration_heartbeat_age_seconds > 180` | 90s | P2 on-call |
| JobRunAbandoned | `increase(job_orchestration_runs_total{outcome="abandoned"}[10m]) > 0` | 0 | P2 on-call |
| JobStillFailing | `increase(job_orchestration_job_failures_total[5m]) > 0 and on(job) increase(job_orchestration_runs_total{outcome="succeeded"}[5m]) == 0` | 300s | P3 async triage with a dashboard link; a job whose retry succeeds inside five minutes never pages, and a job still failing after five minutes does |
| WindowsUnrecovered | `increase(job_orchestration_windows_unrecovered_total[1h]) > 0` | 0 | P3; the alert links the runbook and the log range |
| PruneMissed | `absent(job_orchestration_last_prune_timestamp) or time() - job_orchestration_last_prune_timestamp > 129600` | 24h | P3; the gauge is the epoch seconds stored by the last successful prune, exported by the service |

The failure counter still increments on every failed attempt and every direct 4xx abandonment, so
it is NFR1.3's five-minute visibility signal on the dashboard; only the *alert* is suppressed,
not the metric.

## Dashboard

Heartbeat age, tick duration p95, run outcomes per job, target latency, failures, abandoned runs,
unrecovered windows, prune age.
