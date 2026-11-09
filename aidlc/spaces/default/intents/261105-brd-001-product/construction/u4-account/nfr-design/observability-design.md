# Observability Design — u4-account

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`,
`scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`,
`tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

## Metrics

| Metric | Type | Labels |
|---|---|---|
| `account_request_duration_seconds` | histogram | route, status |
| `account_request_errors_total` | counter | route, status |
| `account_sync_runs_total` | counter | status, venue |
| `account_sync_duration_seconds` | histogram | venue |
| `account_provider_errors_total` | counter | venue, category |
| `account_import_rows_total` | counter | source, outcome |
| `account_lease_expired_total` | counter | kind |
| `account_db_pool_connections` | gauge | state |
| `account_health` | gauge | — |

## Structured logs

- JSON with timestamp, level, service, correlationId, route, status, accountId (internal id only),
  durationMs; no key, token, label with secret, or full address.
- Provider error categories, never response bodies containing credentials.

## Tracing

- OTLP spans for the request, provider call, C4 resolve, DB write, and audit append.

## Alerts

| Alert | Condition | Severity |
|---|---|---|
| `getHoldings` success | below 99.95% over 5 min | Page |
| `getHoldings` latency | fewer than 90% under 150 ms over 10 min | Page |
| Holdings stale | any account's last successful snapshot older than 12 h | Ticket |
| Sync error rate | > 1% over 30 min | Page |
| Readiness failing | any task unready 2 min | Page |
| Lease expired without re-claim | 2 × 60 s sweeps | Ticket |
| Provider auth failure | any `auth` category | Page |
| Key revocation | any `key_revoked` event | Ticket |
| First sync overdue | a first run older than 2 min (wallet) / 10 min (exchange) | Ticket |

## SLIs / SLOs

| SLI | SLO |
|---|---|
| `getHoldings` success | 99.9% / 30 days |
| `getHoldings` latency | 95% under 150 ms |
| Sync success | 99% of scheduled runs |
| First sync | 95% within the bound |

NFR references: NFR17.1–NFR17.5.
