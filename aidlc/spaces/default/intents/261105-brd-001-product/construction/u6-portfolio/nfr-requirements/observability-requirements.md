# Observability Requirements — u6-portfolio

| ID | Requirement | Target |
|---|---|---|
| NFR17.1 | Structured logs | ids, route, status, duration; never amounts, tokens, or addresses |
| NFR17.2 | Health metric | `portfolio_health` from `/readyz` |
| NFR17.3 | Request metrics | `portfolio_request_duration_seconds`, `_errors_total` by route/status |
| NFR17.4 | Valuation metrics | `portfolio_valuations_total` by status, `portfolio_valuation_duration_seconds`, `portfolio_unpriced_assets` gauge |
| NFR17.5 | Snapshot metrics | `portfolio_snapshots_total`, `portfolio_snapshot_digest_mismatch_total`, `portfolio_audit_failures_total` |
| NFR17.6 | Traces | OTLP spans read → C5 → C6 → compute → snapshot |
| NFR17.7 | Alerts | page on 5xx > 0.05% 10 min, digest mismatch, any audit write abandoned, or valuation run missed twice; ticket when unpriced assets exceed 10% of a portfolio's total asset lines over 30 min |
| NFR17.8 | Dashboards | read latency, unpriced, valuation runs, snapshot growth |
