# Monitoring Design — u1-user-registry

**Upstream:** `observability-design.md`, `reliability-design.md`.

## Metrics & KPIs

| Metric | Source | Threshold | Why it matters |
|---|---|---|---|
| `identity_principals_resolved_total{outcome}` | Identity `/metrics` | error rate > 0.1% burn | Resolution health |
| `identity_principal_duration_seconds` | Identity `/metrics` | p95 > 100 ms for 15 min | Latency target |
| `identity_users_provisioned_total{outcome}` | Identity `/metrics` | failed > 1% for 10 min | Provisioning health |
| `identity_admin_lookups_total{outcome}` | Identity `/metrics` | — | Admin use and denials |
| `identity_audit_write_failures_total` | Identity `/metrics` | > 0 for 5 min | Evidence gaps |

## Alerts

| Alert | Condition | Severity | Routes to |
|---|---|---|---|
| Provisioning failures | failed rate > 1% for 10 min | Page | On-call |
| Resolution error budget fast burn | 14.4× burn over 5 min | Page | On-call |
| Resolution error budget slow burn | 1× burn over 6 h | Ticket | Platform |
| Resolution latency | p95 > 100 ms for 15 min | Ticket | Platform |
| Database down | `/readyz` false for 1 min | Page | On-call |
| Dead man | zero resolutions for 10 min | Page | On-call |
| Audit writes failing | any increase over 5 min | Page | On-call |

## SLIs / SLOs

| SLI | SLO target | Window |
|---|---|---|
| Successful resolutions / total | 99.9% | 30-day rolling |
| Provisioning success / attempts | 99.5% | 30-day rolling |

## Logs & Tracing

Structured JSON to Loki, correlation id on every line, no subject plaintext or tokens. Tempo spans per resolution; Grafana dashboards for resolution mix, latency, provisioning, and audit failures.
