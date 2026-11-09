# Observability Design — u3-identity

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`,
`scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`,
`tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

## Metrics

| Metric | Type | Labels |
|---|---|---|
| `identity_request_duration_seconds` | histogram | route, status |
| `identity_request_errors_total` | counter | route, status |
| `identity_resolve_principal_duration_seconds` | histogram | — |
| `identity_audit_append_duration_seconds` | histogram | path |
| `identity_export_status_total` | counter | status |
| `identity_deletion_status_total` | counter | status |
| `identity_work_lease_expired_total` | counter | kind |
| `identity_db_pool_connections` | gauge | state |

Every new service also exposes a health metric and an error-rate metric, as the project requires.

## Structured logging

- JSON logs with `timestamp`, `level`, `service`, `correlationId`, `route`, `status`, `userId`
  (internal id only), and `durationMs`.
- Allow-listed fields only; never a token, credential, email, IP, or full address.
- The correlation id is taken from the gateway and propagated to every outbound call and audit
  event.

## Distributed tracing

- OTLP traces to the self-hosted Tempo stack; spans for the request, the JWKS check, the DB
  lookup, and each outbound call.
- Worker spans per record and per contribution, linked by the export/deletion id.

## Alerts

| Alert | Condition | Action |
|---|---|---|
| Principal errors | error rate > 0.5% over 5 min | Page on-call |
| Readiness failing | any task unready 2 min | Page on-call |
| Deletion failed | any request `failed`, or `in_progress` over 1 h | Page on-call |
| Export stuck | completion < 99% over 1 h, any `building` over 15 min, or an expired lease with no re-claim | Ticket; page if repeated |
| Audit append latency | p95 > 100 ms over 10 min | Ticket |

## Dashboards

- Service dashboard: request rate, p50/p95/p99, error rate, pool saturation, and the export/deletion
  status counts.
- Security view: denials, support reads, and MFA refusals (counts only, no record bodies).

## SLI/SLO

- `resolvePrincipal`: 99.9% success over 30 days; 95% under 50 ms.
- Export: 99% reach `ready` within 15 min.
- Deletion: 99% reach `completed` within 24 h.

NFR references: NFR9.2, NFR17.1, NFR17.2, NFR17.3, NFR17.4, NFR17.5.
