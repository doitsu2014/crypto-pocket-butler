# Observability Design — u5-market-data

Inputs: `nfr-requirements/observability-requirements.md` and `functional-design/rules.md`.

## Metrics

| Metric | Type | Labels |
|---|---|---|
| `market_data_request_duration_seconds` | histogram | route, status |
| `market_data_request_errors_total` | counter | route, status |
| `market_data_refresh_runs_total` | counter | job, status |
| `market_data_refresh_duration_seconds` | histogram | job |
| `market_data_source_errors_total` | counter | source, category |
| `market_data_unpriced_assets` | gauge | job |
| `market_data_price_age_seconds` | histogram | tier |
| `market_data_fx_age_seconds` | gauge | pair |
| `market_data_freshness_total` | counter | freshness, tier |
| `market_data_audit_failures_total` | counter | — |
| `market_data_health` | gauge | — |

## Logs and traces

- JSON logs: timestamp, level, service, correlationId, route, status, assetId, source, durationMs;
  never a key, token, or full provider URL.
- OTLP spans: request → C4 read → Postgres query; and run → source call → write → audit.

## Alerts

| Alert | Condition | Severity |
|---|---|---|
| Availability | priced-or-stale < 99.95% over 10 min | Page |
| Top-100 freshness | live < 99.5% over 10 min | Page |
| Top-100 job overdue | no successful top100 run for 6 min | Page |
| FX freshness | any pair older than 1 h | Ticket |
| Unpriced growth | unpriced assets > 10% for 30 min | Ticket |
| Provider errors | `provider` errors > 1% for 30 min | Ticket |
| Readiness | any task unready 2 min | Page |
| Audit write failure | any `market_data_audit_failures_total` increment | Page |

## SLIs / SLOs

| SLI | SLO | Window |
|---|---|---|
| Priced-or-stale quote ratio | 99.9% | 30 days |
| Top-100 live freshness | 99% within 5 min | 30 days |
| Tail live freshness | 99% within 15 min | 30 days |
| FX availability | 99.9% | 30 days |
| FX freshness | 99% of requested pairs within 1 h | 30 days |

## Dashboards

- Read path: rate, errors, latency.
- Freshness: age by tier, stale/unpriced counts.
- Refresh: runs by job, duration, provider errors, rate-limit deferrals.
- FX: age by pair, derived-rate count.

NFR references: NFR17.1–NFR17.8.
