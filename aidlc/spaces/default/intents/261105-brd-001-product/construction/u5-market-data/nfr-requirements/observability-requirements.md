# Observability Requirements — u5-market-data

Inputs: `functional-design/rules.md`, `functional-design/functional-spec.md`, and `memory/phases/operation.md`.

| ID | Requirement | Target |
|---|---|---|
| NFR17.1 | Structured logs | JSON with timestamp, level, service, correlationId, route, status, assetId, source, durationMs; never a key or token |
| NFR17.2 | Health metric | `market_data_health` gauge from `/readyz`, scraped every 15 s |
| NFR17.3 | Request metrics | `market_data_request_duration_seconds` histogram and `market_data_request_errors_total` counter by route and status |
| NFR17.4 | Refresh metrics | `market_data_refresh_runs_total` by job and status, `market_data_refresh_duration_seconds` by job, `market_data_source_errors_total` by source and category, `market_data_unpriced_assets` gauge |
| NFR17.5 | Freshness metrics | `market_data_price_age_seconds` by tier, `market_data_fx_age_seconds`, and `market_data_freshness_total` by freshness |
| NFR17.6 | Traces | OTLP spans for request, source call, DB read, and audit append |
| NFR17.7 | Alerts | Page when priced-or-stale availability < 99.95% over 10 min or top-100 live freshness < 99.5% over 10 min (both below their SLOs); ticket when unpriced assets rise > 10% for 30 min or `provider` source errors exceed 1% for 30 min |
| NFR17.8 | Dashboards | Read path, freshness by tier, FX age, refresh runs and provider errors, unpriced assets |

## Notes

- Alert thresholds sit below the SLO breach so there is time to remediate.
- No metric label carries an API key, a full provider URL, or a user identifier.
