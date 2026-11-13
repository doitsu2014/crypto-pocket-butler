# Observability Design — u1-user-registry

**Upstream:** `observability-requirements.md` (NFR17.1–NFR19.2), `reliability-design.md`.

## Metrics

| Metric | Type | Labels | Purpose |
|---|---|---|---|
| `identity_principals_resolved_total` | counter | `outcome=ok\|status_denied\|not_found\|error` | Resolution mix and error rate |
| `identity_principal_duration_seconds` | histogram | `outcome` | Latency vs the p95 target |
| `identity_users_provisioned_total` | counter | `outcome=created\|existing\|failed` | Provisioning rate and failures (backs the failure alert) |
| `identity_admin_lookups_total` | counter | `outcome` | Admin use and denials |
| `identity_audit_write_failures_total` | counter | — | Evidence gaps |

## Logs and tracing

- Structured JSON to stdout (Loki); correlation id on every line.
- Logs carry the internal user id, never the subject plaintext, email at full length, tokens, or credentials.
- One span per resolution with a child span for the gate and the audit write; the correlation id propagates.

## Alerts

| Alert | Condition | Severity |
|---|---|---|
| Provisioning failures | `provisioned_total{outcome="failed"}` rate > 1% for 10 min | Page |
| Resolution errors | `principals_resolved_total{outcome="error"}` > 1% for 10 min | Page |
| Latency | p95 resolution > 100 ms for 15 min | Ticket |
| Database down | `/readyz` false for 1 min | Page |
| Dead man | Zero resolutions for 10 min | Page |

## SLI/SLO

- SLI: successful resolutions / total resolutions.
- SLO: 99.9% monthly over a 30-day window; error budget ≈ 43 min/30 days.
