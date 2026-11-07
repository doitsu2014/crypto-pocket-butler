# Observability Requirements — u10-analytics

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`, and
`inception/requirements-analysis/requirements.md`.

| ID | Requirement | Target |
|---|---|---|
| NFR17.1 | Structured logs | JSON with timestamp, level, service, correlationId, route, status, durationMs, eventName; never a property value, userRef, or subject |
| NFR17.2 | Health metric | `analytics_health` from `/readyz`, scraped every 15 s |
| NFR17.3 | Request metrics | `analytics_request_duration_seconds` and `analytics_request_errors_total` by route/status |
| NFR17.4 | Ingest metrics | `analytics_events_total` by eventName, consentResult, outcome |
| NFR17.5 | Aggregate metrics | `analytics_aggregate_runs_total` by status, `analytics_aggregate_duration_seconds`, `analytics_suppressed_cohorts` |
| NFR17.6 | Erasure metric | `analytics_erased_events_total`; a failed erase increments an error counter |
| NFR17.7 | Alerts | Page on ingest 5xx > 0.15% for 10 min (below the 99.9% SLO) or readiness failing 2 min; ticket on a missed nightly aggregate or suppressed cohorts > 20% |
| NFR17.8 | Dashboards | Ingest rate/errors, consent refusal rate, aggregate freshness, flag evaluations |
