# Monitoring Design — u6-jurisdiction-gate

**Upstream:** `observability-design.md`, `reliability-design.md`, `infrastructure-specification.md`.

## Metrics & KPIs

| Metric | Source | Threshold | Why it matters |
|---|---|---|---|
| `jurisdiction_checks_total{outcome}` | Identity `/metrics` | — | Decision volume and mix |
| `jurisdiction_check_duration_seconds` histogram | Identity `/metrics` | p95 > 1.5 s | Latency SLO |
| `jurisdiction_provider_errors_total` | Identity `/metrics` | > 5% for 10 min | Provider health; fail-closed input |
| `jurisdiction_provider_circuit_state` | Identity `/metrics` | 2 (open) for 1 min | Provider outage detected early |
| `jurisdiction_list_version` | Identity `/metrics` | — | Which version each replica serves |
| `jurisdiction_list_refresh_age_seconds` | Identity `/metrics` | > 30 s | Replica staleness |
| `jurisdiction_list_content_age_seconds` | Identity `/metrics` | > 30 days | List review overdue |
| `jurisdiction_audit_write_failures_total` | Identity `/metrics` | > 0 for 5 min | Evidence gaps |

## Alerts

| Alert | Condition | Severity | Routes to |
|---|---|---|---|
| Provider failure rate | 5xx/timeout ratio > 5% for 10 min | Page | On-call |
| Provider circuit open | Circuit open for 1 min | Page | On-call |
| Audit writes failing | Any failure increase over 5 min | Page | On-call |
| List checksum mismatch | Any occurrence | Page | On-call |
| Replica serving a stale list | Refresh age > 30 s | Ticket | Platform |
| List content age | > 30 days | Ticket | Compliance |
| Gate error rate | `jurisdiction_checks_total{outcome="error"}` ratio > 0.1% for 10 min | Page | On-call |

Alert thresholds sit below the SLOs; the provider alert uses an error-budget burn-rate rule rather than a raw percentage.

## SLIs / SLOs

| SLI | SLO target | Measurement window |
|---|---|---|
| Gate successful-check ratio (excluding provider-caused failures) | 99.9% | 30-day rolling |
| Provider-dependency success ratio | 99.5% | 30-day rolling |
| Check latency under 1.5 s | 99% of requests | 30-day rolling |

## Logs & Tracing

- **Logs**: structured JSON to stdout, collected by Loki; correlation id on every line; no raw IP, headers, or tokens.
- **Traces**: Tempo, one span per check with child spans for the provider call and the audit write; the correlation id propagates.
- **Dashboards** (Grafana): decision mix, latency percentiles, provider health, list version/age, audit-write success.
