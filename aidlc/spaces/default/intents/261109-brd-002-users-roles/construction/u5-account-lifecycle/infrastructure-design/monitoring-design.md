# Monitoring Design — u5-account-lifecycle

**Upstream:** `observability-design.md`, `infrastructure-specification.md`, `reliability-design.md`.

## The collection gap, stated first

No Prometheus service or scrape configuration exists in the repository (`infra/prometheus/` holds
only the job-orchestration alert rules). The sweep and the drainer run in `identity-worker`, whose
metrics need their own target beside the API's:

```yaml
- job_name: identity-api
  metrics_path: /metrics
  static_configs: [{ targets: ["identity-api:8083"] }]
- job_name: identity-worker        # the sweep, the drainer, the backlog gauges
  metrics_path: /metrics
  static_configs: [{ targets: ["identity-worker:8085"] }]
```

Until that platform work lands, every alert below is defined but not armed.

## Metrics & KPIs

| Metric | Source | Threshold | Why it matters |
|---|---|---|---|
| `account_status_transitions_total` | worker/API `/metrics` | — (by edge and actor) | Every status change |
| `account_transition_refusals_total` | worker/API | > 5% over 30 min | A client sends illegal edges |
| `deletion_requests_total` | worker/API | — (by event) | The lifecycle over time |
| `deletion_erasure_failures_total` | worker | any non-zero over 15 min | Accounts are not being erased |
| `deletion_due_backlog` | worker | > 500 for 15 min | The sweep is not keeping up |
| `deletion_sweep_last_pass_timestamp_seconds` | worker | `time() - value > 900`, or absent | **Liveness** — a dead sweep emits nothing else |
| `deletion_sweep_duration_seconds` | worker | p95 > 60 s | Batch sizing |
| `status_audit_outbox_oldest_age_seconds` | worker | > 5 min, or absent for 10 | Audit delivery lag |

## Alerts

| Alert | Condition | Severity | Routes to |
|---|---|---|---|
| Erasure failures | any non-zero over 15 minutes | P1 (page) | Identity on-call |
| Due backlog | > 500 for 15 minutes | P1 (page) | Identity on-call |
| Sweep dead | last pass older than 15 minutes, or the series absent | P1 (page) | Identity on-call — the erasure SLO is breaching silently |
| Audit outbox lag | oldest > 5 minutes, or absent for 10 | P1 (page) | Identity on-call |
| Transition refusals | > 5% over 30 minutes | P2 (ticket) | Identity team |
| Batch duration | p95 > 60 s | P2 (ticket) | Identity team |

## SLIs / SLOs

| SLI | Target | Window |
|---|---|---|
| Transition success (status + log + outbox accepted together) | 99.9% | 30-day rolling |
| Erasure within 1 hour of the deadline | 99.9% | 30-day rolling |
| Audit delivery lag | 99.9% within 5 minutes | 30-day rolling |
| Status/log agreement | 100% | 30-day rolling |

## Logs & tracing

Structured JSON to stdout via Loki; the transition span covers the three writes, the erasure span
has a child per cascade step. Never the reason text, email, subject, or token.

## Dashboards

One Grafana board: transitions by edge and actor, refusals, deletion events, due backlog, erasure
failures by step, sweep duration and liveness, outbox lag.
