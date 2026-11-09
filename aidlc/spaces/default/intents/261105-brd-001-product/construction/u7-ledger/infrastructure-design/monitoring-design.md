# Monitoring Design — u7-ledger

Inputs: `nfr-design/observability-design.md`, `reliability-design.md`.

| Metric | Source | Threshold | Why |
|---|---|---|---|
| `ledger_request_duration_seconds` | API | p95 < 400 ms | P&L SLO |
| `ledger_request_errors_total` | API | < 0.05% 5xx | Availability |
| `ledger_transactions_total` | Ingest | — | Volume by source |
| `ledger_duplicates_total` | Ingest | < 5% | Dedupe health |
| `ledger_corrections_total` | API | — | Change volume |
| `ledger_reconcile_runs_total` | Worker | daily success | Reconcile |
| `ledger_pnl_coverage` | P&L | 100% bucket ≥ 90% | Coverage |
| `ledger_audit_failures_total` | API | 0 | Audit durability |
| `ledger_health` | `/readyz` | 1 | Swarm gate |

## Alerts

| Alert | Condition | Severity |
|---|---|---|
| API 5xx | > 0.05% for 10 min (half the 0.1% budget) | Page |
| Reconcile failed | any missed nightly run (first miss pages) | Page |
| API latency | p95 > 400 ms for 10 min | Ticket; runbook scales `ledger` (ceiling 4) |
| Audit failure | any increment | Page |
| Duplicates | > 5% of ingested rows | Ticket |
| Coverage | share of cohort at 100% < 90% | Ticket |
| Readiness | any task unready 2 min | Page |

## SLIs / SLOs

| SLI | SLO | Window |
|---|---|---|
| P&L availability (complete or labelled partial; 5xx bad) | 99.9% | 30 days |
| Reconcile success | 100% nightly | daily |
| Ingest idempotency | 100% | per page |

## Logs & Tracing

- JSON: ids and statuses only, never amounts or credentials.
- OTLP: ingest → dedupe → lot rebuild → P&L → audit.

## Dashboards

- Ingest by source, duplicates, corrections, reconcile, coverage distribution.
