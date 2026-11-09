# Observability Requirements — u7-ledger

| ID | Requirement | Target |
|---|---|---|
| NFR17.1 | Structured logs | JSON with timestamp, level, service, correlationId, route, status, userId hash, transactionId, durationMs; never a credential or full address |
| NFR17.2 | Health metric | `ledger_health` from `/readyz`, scraped every 15 s |
| NFR17.3 | Request metrics | `ledger_request_duration_seconds` and `ledger_request_errors_total` by route/status |
| NFR17.4 | Ledger metrics | `ledger_transactions_total` by kind/source, `ledger_duplicates_total`, `ledger_corrections_total`, `ledger_reconcile_runs_total` and duration |
| NFR17.5 | Coverage metric | `ledger_pnl_coverage` histogram over fixed coverage buckets (0, 0–25%, 25–50%, 50–75%, 75–99%, 100%), no user label |
| NFR17.6 | Traces | OTLP spans for ingest, lot rebuild, P&L compute, audit append |
| NFR17.7 | Alerts | page on 5xx > 0.05% for 10 min (half the 0.1% budget) or reconcile failure 2 nights; ticket on duplicates > 5% of ingested rows or coverage below 90% in the 100% bucket for a cohort |
| NFR17.8 | Dashboards | ingest by source, duplicates, corrections, reconcile, coverage distribution |
