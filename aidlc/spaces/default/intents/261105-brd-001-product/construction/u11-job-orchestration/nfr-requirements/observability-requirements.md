# Observability Requirements — u11-job-orchestration

| ID | Requirement | Measure |
|---|---|---|
| NFR17.1 | Structured JSON logs with correlation id, job, window, attempt and outcome; no token or response body | Log-field test |
| NFR17.2 | `/metrics` exports `job_orchestration_health`, `job_orchestration_heartbeat_age_seconds`, and the `job_orchestration_tick_duration_seconds` histogram | Exposition test |
| NFR17.3 | Request metrics by status class and route, with p95 latency for operator reads | `/metrics` |
| NFR17.4 | `job_orchestration_runs_total{job,outcome}` and a per-target-call duration histogram | `/metrics` |
| NFR17.5 | `job_orchestration_job_failures_total{job}` increments on every failed attempt and on every direct 4xx abandonment | `/metrics` |
| NFR17.6 | The W3C `traceparent` propagates to the target call | Client test |
| NFR17.7 | Alert rules: (a) heartbeat `absent(...) or > 180` with `for: 90s`, P2 on-call; (b) any `job_failures_total` increment, P3 async triage with a dashboard link, suppressed for a job whose retry succeeds within one window; (c) any `abandoned` increment, P2 | Rule files in the design |
| NFR17.8 | A dashboard shows heartbeat age, tick duration, run outcomes per job, target latency, failures, abandoned runs and unrecovered windows | Grafana JSON in the design |
