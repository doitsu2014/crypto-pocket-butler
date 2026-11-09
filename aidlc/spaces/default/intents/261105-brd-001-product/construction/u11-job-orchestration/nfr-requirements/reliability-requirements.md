# Reliability Requirements — u11-job-orchestration

| ID | Requirement | Measure |
|---|---|---|
| NFR1.1 | Scheduler availability 99.9% monthly for operator reads | 2xx/4xx without 5xx over 30 days |
| NFR1.2 | A stopped or wedged orchestrator pages within five minutes (AC9.1.2) | Rule `absent(job_orchestration_heartbeat_age_seconds) or job_orchestration_heartbeat_age_seconds > 180`, scrape 15 s, eval 30 s, `for: 90s` |
| NFR1.3 | A silently failing job is visible within five minutes of its first failed attempt | `job_orchestration_job_failures_total{job}` increments on every failed attempt; alert fires on any increment |
| NFR1.4 | A permanently misconfigured job (direct 4xx abandonment) also alerts | `job_orchestration_runs_total{job,outcome="abandoned"}` increments and an alert rule fires on it |
| NFR1.5 | Retries are bounded to `maxAttempts` ≤ 5 | Unit tests over the retry planner |
| NFR1.6 | Missed windows inside 24 h are recovered oldest-first exactly once | Outage simulation test |
| NFR1.7 | Windows older than 24 h after a longer outage are surfaced, never silently dropped | `job_orchestration_windows_unrecovered_total{job}` increments; an alert names the range and suggests a backfill |
| NFR1.8 | A lost lease never loses or duplicates a window | Lease-expiry and duplicate-call tests; the idempotency key makes the target apply once |
| NFR1.9 | `/readyz` fails only on the orchestrator's own Postgres | Readiness test |
| NFR1.10 | Work resumes within 90 s after a restart (lease TTL 60 s + one tick) | Restart test with a seeded missed window |
