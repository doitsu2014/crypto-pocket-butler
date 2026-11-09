# Observability Design — u8-reporting

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`, `scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`, `tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

| Metric | Type | Labels |
|---|---|---|
| `reporting_request_duration_seconds` | histogram | route, status |
| `reporting_request_errors_total` | counter | route, status |
| `reporting_briefs_total` | counter | period, status |
| `reporting_generation_duration_seconds` | histogram | period |
| `reporting_source_failures_total` | counter | source |
| `reporting_deliveries_total` | counter | channel, status |
| `reporting_health` | gauge | — |

Alerts: page on 5xx > 0.05%/10 min, failed briefs > 1% of briefs attempted in a run, or a run
missed twice; ticket on email submission failures. SLIs: read availability 99.9%; brief generation
success 99% (explicit failures excluded); email accepted 99%. Logs ids/status only; traces
run → C3 → C8 → C7 → compose → C9. Dashboards: briefs by period/status, generation time, source
failures, deliveries.

NFR references: NFR17.1–NFR17.8.
