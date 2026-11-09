# Observability Requirements — u9-notification

| ID | Requirement | Target |
|---|---|---|
| NFR17.1 | Structured logs | ids, type, channel, status; never an address, credential, or payload value |
| NFR17.2 | Health metric | `notification_health` from `/readyz` |
| NFR17.3 | Request metrics | `notification_request_duration_seconds` and `notification_request_errors_total` by route/status |
| NFR17.4 | Delivery metrics | `notification_alerts_total` by type/status, `notification_attempts_total` by channel/status, `notification_duplicates_total` |
| NFR17.5 | Mandatory metric | `notification_mandatory_delivery_total` by type/status |
| NFR17.6 | Traces | OTLP spans submit → dedupe → deliver → audit; attributes carry allow-listed ids (alertId, type, channel, outcome) only, never an address or payload value |
| NFR17.7 | Alerts | page on 5xx > 0.05% 10 min, any abandoned mandatory email, or provider failure > 5% 10 min; ticket on delivery backlog > 1,000 |
| NFR17.8 | Dashboards | intake by type, delivery outcomes, mandatory delivery, backlog |
