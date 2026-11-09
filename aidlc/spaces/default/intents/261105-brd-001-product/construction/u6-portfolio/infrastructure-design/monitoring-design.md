# Monitoring Design — u6-portfolio

Inputs: `nfr-design/observability-design.md`, `reliability-design.md`.

| Metric | Source | Threshold | Why |
|---|---|---|---|
| `portfolio_request_duration_seconds` | API | read p95 < 300/800 ms | Budgets |
| `portfolio_request_errors_total` | API | < 0.05% 5xx | Availability |
| `portfolio_valuations_total` | API/worker | — | Volume/status |
| `portfolio_valuation_run_duration_seconds` | Worker | < 900 s | Whole-run budget |
| `portfolio_valuation_duration_seconds` | API/worker | per valuation p95 < 800 ms | Single valuation |
| `portfolio_unpriced_assets` | API | < 10% | Quality |
| `portfolio_snapshot_digest_mismatch_total` | Worker | 0 | Integrity |
| `portfolio_audit_failures_total` | API | 0 | Audit |
| `portfolio_guardrail_alerts_failed_total` | Worker | 0 | Crossing retries |
| `portfolio_health` | `/readyz` | 1 | Gate |

## Alerts

| Alert | Condition | Severity |
|---|---|---|
| 5xx | > 0.05% 10 min | Page |
| Digest mismatch | any | Page |
| Audit abandoned | any | Page |
| Run missed | 2 consecutive | Page |
| Guardrail alert failure | any | Ticket |
| Unpriced | > 10% of lines 30 min | Ticket |

## SLIs / SLOs

| SLI | SLO | Window |
|---|---|---|
| Read availability (degraded labelled reads good; 5xx bad) | 99.9% | 30 days |
| Snapshot digest verification | 100% | per snapshot |
| Run success | 100% hourly | hourly |

## Logs & tracing / dashboards

- Ids/status only. OTLP read → C5 → C6 → compute → snapshot.
- Dashboards: read latency, unpriced, run status, snapshot growth.
