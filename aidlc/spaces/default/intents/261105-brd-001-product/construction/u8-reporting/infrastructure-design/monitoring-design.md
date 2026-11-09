# Monitoring Design — u8-reporting

Inputs: `nfr-design/observability-design.md`, `reliability-design.md`.

| Metric | Source | Threshold | Why |
|---|---|---|---|
| `reporting_request_duration_seconds` | API | list 200 / open 300 ms | Read budgets |
| `reporting_request_errors_total` | API | < 0.05% 5xx | Availability |
| `reporting_briefs_total` | Worker | failed < 1% of attempted | Generation health |
| `reporting_generation_duration_seconds` | Worker | < 2 s | Per-brief |
| `reporting_source_failures_total` | Worker | — | Source health |
| `reporting_deliveries_total` | Worker | — | Email outcomes |
| `reporting_health` | `/readyz` | 1 | Gate |

## Alerts

| Alert | Condition | Severity |
|---|---|---|
| 5xx | > 0.05% 10 min | Page |
| Failed briefs | > 1% of briefs attempted in a run | Page |
| Run missed | 2 consecutive | Page |
| Email submission failures | any | Ticket |

## SLIs / SLOs

| SLI | SLO | Window |
|---|---|---|
| Read availability | 99.9% | 30 days |
| Brief generation success (excluding explicit source failures) | 99% | 30 days |
| Email accepted | 99% | 30 days |
| Run success | 100% daily | daily |

## Logs & tracing / dashboards

- Ids/status only; traces run → C3 → C8 → C7 → compose → C9.
- Dashboards: briefs by period/status, generation time, source failures, deliveries.
