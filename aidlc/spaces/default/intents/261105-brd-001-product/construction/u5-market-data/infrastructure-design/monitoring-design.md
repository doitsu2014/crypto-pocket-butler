# Monitoring Design — u5-market-data

Inputs: `nfr-design/observability-design.md`, `reliability-design.md`, and `performance-design.md`.

## Metrics & KPIs

| Metric | Source | Threshold | Why it matters |
|---|---|---|---|
| `market_data_request_duration_seconds` | API | p95 < 250 ms | Read SLO |
| `market_data_request_errors_total` | API | < 0.1% 5xx | Availability |
| `market_data_refresh_runs_total` | Worker | > 99% success | Freshness |
| `market_data_refresh_duration_seconds` | Worker | top100 < 180 s; tail < 600 s | Run deadlines |
| `market_data_source_errors_total` | Worker | provider < 1% | Provider health |
| `market_data_unpriced_assets` | Worker | < 10% | Coverage |
| `market_data_price_age_seconds` | API | top100 ≤ 300 s; tail ≤ 900 s | Freshness SLO |
| `market_data_fx_age_seconds` | API | < 3600 s | FX SLO |
| `market_data_freshness_total` | API | — | Dashboard mix |
| `market_data_audit_failures_total` | Worker | 0 | Audit durability |
| `market_data_health` | `/readyz` | 1 | Swarm gate |

## Alerts

| Alert | Condition | Severity | Routes to |
|---|---|---|---|
| Availability | priced-or-stale < 99.95% over 10 min | Page | On-call |
| Top-100 freshness | live < 99.5% over 10 min | Page | On-call |
| Top-100 job overdue | no successful top100 run for 6 min | Page | On-call |
| Tail freshness | live < 99% over 20 min (below the 15-minute SLO) | Ticket | On-call |
| Tail job overdue | no successful tail run for 12 min | Ticket | On-call |
| API scaling | p95 > 250 ms for 10 min | Ticket | On-call runs `docker service scale market-data=N` (runbook) |
| FX freshness | any pair older than 1 h | Ticket | Market data owner |
| Unpriced growth | > 10% for 30 min | Ticket | Market data owner |
| Provider errors | `provider` > 1% for 30 min | Ticket | On-call |
| Audit failure | any `market_data_audit_failures_total` increment | Page | On-call + security |
| Readiness | any task unready 2 min | Page | On-call |

## SLIs / SLOs

| SLI | SLO | Window |
|---|---|---|
| Priced-or-stale ratio | 99.9% | 30 days |
| Top-100 live freshness | 99% within 5 min | 30 days |
| Tail live freshness | 99% within 15 min | 30 days |
| FX availability | 99.9% | 30 days |
| FX freshness | 99% of requested pairs within 1 h | 30 days |

## Logs & Tracing

- JSON to Loki: timestamp, level, service, correlationId, route, status, assetId, source,
  durationMs; no key or token.
- OTLP to Tempo: request → C4 read → Postgres; run → source → write → audit.

## Dashboards

- Read path, freshness by tier, refresh by job, provider errors, unpriced assets, FX age.
