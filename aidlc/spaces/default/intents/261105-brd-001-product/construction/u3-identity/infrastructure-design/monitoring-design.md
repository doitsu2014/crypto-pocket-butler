# Monitoring Design — u3-identity

Inputs: `nfr-design/performance-design.md`, `security-design.md`, `scalability-design.md`,
`reliability-design.md`, `observability-design.md`, `logical-components.md`,
`inception/domain-design/components.md`, `functional-design/functional-spec.md`, and
`inception/contract-design/contract-summary.md`.

## Metrics & KPIs

| Metric | Source | Threshold | Why it matters |
|---|---|---|---|
| `identity_request_duration_seconds` | Prometheus (OTLP) | p95 ≤ 50 ms for `resolvePrincipal` | Identity is on every request path |
| `identity_request_errors_total` | Prometheus | SLO 99.9% success; alert below 99.95% over 5 min | A principal failure blocks the product |
| `identity_resolve_principal_duration_seconds` | Prometheus | < 90% under 50 ms over 10 min alerts | Directly tracks the latency SLO |
| `identity_audit_append_duration_seconds` | Prometheus | p95 ≤ 100 ms | Bounds the support-read path |
| `identity_export_status_total` | Prometheus | completion ≥ 99% in 15 min | Tracks the export SLO |
| `identity_deletion_status_total` | Prometheus | completion ≥ 99% in 24 h | Tracks the deletion SLO |
| `identity_work_lease_expired_total` | Prometheus | any expired lease without re-claim | Detects a stuck worker |
| `identity_db_pool_connections` | Prometheus | ≤ 20 per replica, ≤ 60 total | Protects the Supabase connection budget |
| `identity_health` | readiness probe | 1 per task | Health metric required by the project rules |

## Alerts

| Alert | Condition | Severity | Routes to |
|---|---|---|---|
| Principal success below floor | success < 99.95% over 5 min | Page | On-call |
| Principal latency | fewer than 90% under 50 ms over 10 min | Page | On-call |
| Task unready | any task unready 2 min | Page | On-call |
| Deletion failed/stuck | any `failed`, or `in_progress` over 1 h | Page | On-call |
| Export completion | < 99% over 1 h, any `building` over 15 min, or an expired lease with no re-claim | Ticket; page if repeated | On-call |
| Audit append slow | p95 > 100 ms over 10 min | Ticket | On-call |
| Pool saturation | 20 connections held for 10 min | Ticket | On-call |
| Archive deletion failed | export object still present after `downloadExpiresAt` + 2 sweeps | Ticket | On-call |

## SLIs / SLOs

| SLI | SLO target | Measurement window |
|---|---|---|
| `resolvePrincipal` success | 99.9% | 30 days |
| `resolvePrincipal` latency under 50 ms | 95% | 30 days |
| Export reaching `ready` within 15 min | 99% | 30 days |
| Deletion reaching `completed` within 24 h | 99% | 30 days |

## Logs & Tracing

- Logs: JSON to the self-hosted Loki stack via the Swarm logging driver; fields allow-listed, no
  token, credential, email, IP, or full address.
- Traces: OTLP to Tempo; spans for the request, JWKS check, DB lookup, and each outbound call;
  worker spans per record and contribution.
- Correlation id: taken from the gateway, carried on every log line and span, and recorded in the
  audit event.

## Dashboards

- Service dashboard: request rate, p50/p95/p99, error rate, pool saturation, export/deletion status
  counts, lease expiries.
- Security view: denials, support reads, MFA refusals (counts only).

NFR references: NFR9.2, NFR17.1, NFR17.2, NFR17.3, NFR17.4, NFR17.5.
