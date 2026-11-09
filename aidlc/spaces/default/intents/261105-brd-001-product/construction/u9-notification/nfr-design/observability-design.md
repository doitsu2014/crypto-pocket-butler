# Observability Design — u9-notification

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`, `scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`, `tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

| Metric | Type | Labels |
|---|---|---|
| `notification_request_duration_seconds` | histogram | route, status |
| `notification_request_errors_total` | counter | route, status |
| `notification_alerts_total` | counter | type, status |
| `notification_attempts_total` | counter | channel, status |
| `notification_duplicates_total` | counter | type |
| `notification_mandatory_delivery_total` | counter | type, status |
| `notification_audit_failures_total` | counter | — |
| `notification_health` | gauge | — |

Alerts: page on 5xx > 0.05%/10 min, any abandoned mandatory email, provider failure > 5%/10 min, or
an audit write abandoned after 3 attempts; ticket on backlog > 1,000.

Dashboards: intake by type, delivery outcomes by channel, mandatory delivery, backlog.

SLIs: intake availability 99.9% (bad = 5xx); delivery success 99% (excluding abandoned address);
mandatory delivery 100%.

Logs: ids/status only. Traces: submit → dedupe → deliver → audit, allow-listed attributes.

NFR references: NFR17.1–NFR17.8.
