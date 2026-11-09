# Observability Requirements — u8-reporting

| ID | Requirement | Target |
|---|---|---|
| NFR17.1 | Structured logs | ids, period, status, route; never figures, credentials, or addresses |
| NFR17.2 | Health metric | `reporting_health` from `/readyz` |
| NFR17.3 | Request metrics | `reporting_request_duration_seconds`, `_errors_total` by route/status |
| NFR17.4 | Generation metrics | `reporting_briefs_total` by period/status, `reporting_generation_duration_seconds`, `reporting_source_failures_total` by source |
| NFR17.5 | Delivery metrics | `reporting_deliveries_total` by channel/status |
| NFR17.6 | Traces | OTLP spans run → C3 → C8 → C7 → compose → C9 |
| NFR17.7 | Alerts | page on 5xx > 0.05% 10 min, failed briefs > 1% of briefs attempted in a run, or a run missed twice; ticket on email submission failures |
| NFR17.8 | Dashboards | briefs by period/status, generation time, source failures, deliveries |
