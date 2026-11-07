# Observability Requirements — u4-account

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`,
`inception/requirements-analysis/requirements.md`, and `inception/contract-design/contract-summary.md`.

## Targets

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR17.1 | Structured logs with a correlation id, no secrets | JSON; no key, token, or full address | NFR17 |
| NFR17.2 | Health and readiness endpoints | readiness checks Postgres | NFR17 |
| NFR17.3 | Per-endpoint metrics | duration histogram and error counter | NFR17 |
| NFR17.4 | Sync and import metrics | run status counts, duration, provider error categories, rows imported/skipped/failed | NFR17 |
| NFR17.5 | Actionable alerts | sync error rate, readiness failure, a lease expired without a re-claim within two 60-second sweeps, provider auth failures, key revocations | NFR17 |

## SLIs and SLOs

| SLI | SLO | Alert |
|---|---|---|
| `getHoldings` success | 99.9% / 30 days | below 99.95% over 5 min |
| `getHoldings` latency | 95% under 150 ms | below 90% over 10 min |
| Sync success | 99% of scheduled runs succeed | error rate > 1% over 30 min |
| First sync after add | 95% within 2 min (wallet) / 10 min (exchange) | a first run older than the bound |

## Logs, traces, dashboards

- Logs to Loki with correlation id; provider error categories only.
- OTLP traces: request, provider call, registry resolve, DB write.
- Dashboard: sync rate by status, error categories, lease expiries, holdings volume.

NFR references: NFR17.1–NFR17.5.
