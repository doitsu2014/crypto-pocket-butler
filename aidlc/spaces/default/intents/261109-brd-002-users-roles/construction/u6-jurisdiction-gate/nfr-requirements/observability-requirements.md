# Observability Requirements — u6-jurisdiction-gate

**Upstream:** `functional-spec.md` (W1–W3), `rules.md` (BR1.7), `contract-summary.md` (C9), `requirements.md`.

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR17.1 | Health metrics | `/healthz`, `/readyz`; readiness is false while the list is unreadable or the provider circuit is open | BRD-001 NFR17 |
| NFR17.2 | Decision metrics | `jurisdiction_checks_total{outcome="allowed|denied|error", restricted_by}` counter | BRD-001 NFR17 |
| NFR17.3 | Latency metrics | `jurisdiction_check_duration_seconds` histogram with an outcome label, measuring the p95 targets | NFR2.1, NFR2.2 |
| NFR17.4 | Provider metrics | `jurisdiction_provider_errors_total`, `jurisdiction_provider_duration_seconds`, `jurisdiction_provider_circuit_state` | NFR2.3 |
| NFR17.5 | List metrics | `jurisdiction_list_version` gauge, `jurisdiction_list_age_seconds` gauge, `jurisdiction_list_refresh_failures_total` | NFR20.5 |
| NFR17.6 | Audit metrics | `jurisdiction_audit_write_failures_total` counter | NFR1.7 |
| NFR17.7 | Alerts | Provider error rate > 5% for 10 min; audit-write failure rate > 0 for 5 min; list checksum mismatch (immediate); list age > 30 days (warning); replica serving a version older than 30 s (warning) | NFR1.1, NFR1.2 |
| NFR19.3 | Audit | One event per decision with actor (or system), action, target, timestamp, context, outcome; no personal data | BR1.7, NFR19 |
| NFR17.8 | Tracing | The caller's correlation id flows through the provider call and the audit write | BRD-001 NFR17 |

Alert thresholds sit below the availability targets so a sustained dependency failure is caught before the SLO is breached.
