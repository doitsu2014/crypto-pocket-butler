# Monitoring Design — u13-support-console

Inputs: `nfr-design/`, `infrastructure-specification.md`.

| Signal | Source | Threshold | Why |
|---|---|---|---|
| `/support` 5xx | CDN/gateway | > 0.5% over 10 min | Bundle availability |
| C2 errors from the console | gateway | 401/403 spike | Role/grant misconfiguration |
| Static asset 404/500 | gateway access log | any spike | Broken bundle |
| Incident declaration failures | support API | any | Operator blocked |
| SEV-1 notice latency | support API + C9 | > 10 min | 15-minute criterion at risk |

## Alerts

| Alert | Condition | Severity |
|---|---|---|
| Support console unavailable | `/support` 5xx > 0.05% 10 min (half the 0.1% budget) | Page |
| C2 authorization spike | 403s > baseline 3× | Ticket |
| SEV-1 notice late | > 10 min | Page |
| Asset error spike | static 404/500 above baseline | Ticket |

## SLIs

| SLI | SLO | Window |
|---|---|---|
| `/support` availability | 99.9% | 30 days |
| SEV-1 notice within 15 min | 100% | per incident |

## Logs and dashboards

- Gateway logs only; no console PII. Dashboard: `/support` availability, C2 error mix, incident
  activity.
