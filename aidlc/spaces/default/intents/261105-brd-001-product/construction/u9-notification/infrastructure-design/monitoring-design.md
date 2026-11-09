# Monitoring Design — u9-notification

Inputs: `nfr-design/observability-design.md`, `reliability-design.md`.

| Metric | Source | Threshold | Why |
|---|---|---|---|
| `notification_request_duration_seconds` | API | submit p95 < 100 ms; preferences < 150 ms; feed < 200 ms | Per-route budgets |
| `notification_request_errors_total` | API | < 0.05% 5xx | Availability |
| `notification_alerts_total` | API | — | Volume by type/status |
| `notification_attempts_total` | Worker | provider failures < 5% | Delivery health |
| `notification_mandatory_delivery_total` | Worker | 100% sent | Mandatory email |
| `notification_duplicates_total` | API | — | Dedupe |
| `notification_audit_failures_total` | API | 0 abandoned | Audit durability |
| `notification_health` | `/readyz` | 1 | Swarm gate |

## Alerts

| Alert | Condition | Severity |
|---|---|---|
| API 5xx | > 0.05% 10 min | Page |
| Mandatory email abandoned | any | Page |
| Provider failures | > 5% 10 min | Page |
| Audit abandoned | after 3 attempts | Page |
| Backlog | > 1,000 queued | Ticket |
| Readiness | any task unready 2 min | Page |

## SLIs / SLOs

| SLI | SLO | Window |
|---|---|---|
| Intake availability | 99.9% | 30 days |
| Delivery success (excluding abandoned address) | 99% | 30 days |
| Mandatory delivery | 100% | per alert |

## Logs & Tracing

- Ids/status only; no address or payload value.
- OTLP: submit → dedupe → deliver → audit with allow-listed attributes.

## Dashboards

- Intake by type, delivery outcomes by channel, mandatory delivery, backlog.
