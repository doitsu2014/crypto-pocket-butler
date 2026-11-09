# Observability Requirements — u3-identity

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`,
`inception/requirements-analysis/requirements.md`, and `inception/contract-design/contract-summary.md`.

## Targets

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR17.1 | Structured logs carry a correlation id on every request and worker step | JSON logs; no credential, token, or full address | NFR17 |
| NFR17.2 | Health and readiness endpoints are public | Readiness checks Postgres; health checks the process | NFR17 |
| NFR17.3 | Latency and error-rate metrics per endpoint | Request duration histogram and error counter, labelled by route and status | NFR17 |
| NFR17.4 | Background job metrics | Export and deletion counts by status, failure count, and worker progress | NFR17 |
| NFR17.5 | Actionable alerts | Principal-resolution error rate, readiness failure, deletion failure, and an export or deletion whose lease expired without a re-claim | NFR17 |

## SLIs and SLOs

| SLI | SLO | Alert threshold |
|---|---|---|
| `successful_requests / total_requests` for `resolvePrincipal` | 99.9% over 30 days | Below 99.95% over 5 minutes |
| `requests_below_50ms / total_requests` for `resolvePrincipal` | 95% | Below 90% over 10 minutes |
| Deletion requests reaching `completed` within 24 h | 99% | Any request in `failed` or `in_progress` over 1 h |
| Export requests reaching `ready` within 15 min | 99% | Below 99% completion over 1 hour, or any request still `building` past 15 minutes, or a lease expired without a re-claim within two lease windows |

## Dashboards

- One service dashboard: request rate, p50/p95/p99, error rate, saturation (DB pool), and the
  background status counts.
- The audit trail is the record of security events; the dashboard shows counts, not record bodies.

## Tracing

- The gateway's correlation id is propagated to Identity and to every outbound call to another
  service and to Audit. No trace carries a token or credential.
