# Monitoring Design — u4-account

Inputs: `nfr-design/observability-design.md`, `reliability-design.md`,
`performance-design.md`, and `security-design.md`.

## Metrics & KPIs

| Metric | Source | Threshold | Why it matters |
|---|---|---|---|
| `account_request_duration_seconds` | Account API middleware | p95 < 150 ms | The C5 read is the hot path for Portfolio and Ledger |
| `account_request_errors_total` | Account API middleware | < 0.1% 5xx | Read-path success SLO |
| `account_sync_runs_total` | Worker | > 99% success over 24 h | Holdings freshness |
| `account_sync_duration_seconds` | Worker | p95 < 60 s per account | Scheduled runs must fit the sync window |
| `account_provider_errors_total` | Venue/RPC adapters | auth category = 0 | A revoked or over-permissioned key is a security event |
| `account_import_rows_total` | Import path | error rows < 1% | CSV quality and user feedback |
| `account_import_duration_seconds` | Worker import path | p95 < 60 s per 10,000-row file | Import budget |
| `account_import_staging_duration_seconds` | API upload path | p95 < 2 s per 10,000-row file | Upload acknowledgement |
| `account_lease_expired_total` | Worker sweep | 0 in 24 h | Duplicate or lost runs |
| `account_holdings_staleness_seconds` | Worker snapshot write (gauge by account cohort) | < 43,200 s (12 h) | Backs the staleness alert and dashboard panel |
| `account_first_sync_overdue_total` | Worker | 0 per 10 min | Backs the first-sync alert |
| `account_key_revoked_total` | Provider verification | 0 | Backs the key-revoked alert |
| `account_support_reads_total` | Support metadata read | — (audited) | Dashboard panel; every read is grant-gated |
| `account_audit_write_failures_total` | Audit client | 0 | Security events must not be lost |
| `account_db_pool_in_use` | sqlx pool | > 80% of 12 for 5 min | Pool saturation precedes latency |
| `account_health` | `/readyz` | 1 | Swarm health check and rolling-update gate |

## Alerts

| Alert | Condition | Severity | Routes to |
|---|---|---|---|
| C5 success below SLO | `getHoldings` success < 99.95% over 5 min | Page | On-call engineer |
| C5 latency | < 90% of requests under 150 ms over 10 min | Page | On-call engineer |
| Sync error rate | > 1% of runs over 30 min | Page | On-call engineer |
| Provider auth failure | any `auth` category error | Page | On-call + security |
| Holdings stale | any account with a snapshot older than 12 h | Ticket | Account team |
| Readiness failing | any task unready 2 min | Page | On-call engineer |
| Lease expired without re-claim | 2 × 60 s sweeps | Ticket | Account team |
| Key revoked | any `key_revoked` audit event | Ticket | Security |
| First sync overdue | first run older than 2 min (wallet) / 10 min (exchange) | Ticket | Account team |
| Migration rollback | the release job applied a down migration after a failed update | Page | On-call + release owner |
| Audit write failure | any `account_audit_write_failures_total` increment | Page | On-call + security |

## SLIs / SLOs

| SLI | SLO target | Measurement window |
|---|---|---|
| `getHoldings` success | 99.9% | 30-day rolling |
| `getHoldings` latency | 95% under 150 ms | 30-day rolling |
| Scheduled sync success | 99% of runs | 30-day rolling |
| First sync completion | 95% within 2 min (wallet) / 10 min (exchange) | 30-day rolling |
| Import staging | 95% of 10,000-row uploads staged within 2 s | 30-day rolling |
| Import processing | 95% of 10,000-row files processed within 60 s | 30-day rolling |

## Logs & Tracing

- JSON logs to Loki: timestamp, level, service, correlationId, route, status, accountId (internal
  id only), durationMs. Never a credential, token, secret label, or full address.
- OTLP traces to Tempo: request → provider call → C4 resolve → DB write → audit append. Trace
  attributes carry ids and categories only.
- Prometheus scrapes `/metrics` on both services; the worker exposes run, import, and lease
  counters.

## Dashboards

- **Account read path**: request rate, error rate, latency percentiles, pool in-use ratio.
- **Sync health**: runs by status and venue, run duration, provider errors, staleness per account
  cohort (`account_holdings_staleness_seconds`), first-sync overdue
  (`account_first_sync_overdue_total`), lease expiries.
- **Imports**: rows by source and outcome, row-error reasons, staging and processing duration.
- **Security**: credential events, `account_key_revoked_total`, support metadata reads
  (`account_support_reads_total`), audit write failures.
