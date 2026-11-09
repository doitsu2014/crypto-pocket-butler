# Observability Design — u7-ledger

Inputs: `nfr-requirements/observability-requirements.md`.

| Metric | Type | Labels |
|---|---|---|
| `ledger_request_duration_seconds` | histogram | route, status |
| `ledger_request_errors_total` | counter | route, status |
| `ledger_transactions_total` | counter | kind, source |
| `ledger_duplicates_total` | counter | source |
| `ledger_corrections_total` | counter | field |
| `ledger_reconcile_runs_total` | counter | status |
| `ledger_reconcile_duration_seconds` | histogram | — |
| `ledger_pnl_coverage` | histogram | fixed buckets |
| `ledger_audit_failures_total` | counter | — |
| `ledger_health` | gauge | — |

Alerts: page on 5xx > 0.05% for 10 min, reconcile failure for 2 nights, or any audit failure;
ticket on duplicates > 5% of ingested rows or the share of a cohort at 100% coverage dropping below
90%.

SLIs: P&L availability 99.9% (good = complete or labelled partial; bad = 5xx); reconcile success
daily; ingest idempotency 100%.

Logs: ids and statuses only (no amounts, which are user financial data); no credential. Traces:
ingest → dedupe → lot rebuild → P&L → audit.

Dashboards: ingest by source, duplicates rate, corrections by field, reconcile status/duration,
coverage distribution by bucket.

NFR17.8 dashboards: the five views above.

NFR references: NFR17.1–NFR17.8.
