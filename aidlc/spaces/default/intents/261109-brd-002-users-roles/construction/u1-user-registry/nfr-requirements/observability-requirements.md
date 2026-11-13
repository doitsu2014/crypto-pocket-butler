# Observability Requirements — u1-user-registry

**Upstream:** `functional-spec.md`, `rules.md` (BR1.7, BR1.9), `requirements.md`.

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR17.1 | Health | `/healthz`, `/readyz`; readiness false when the database is unreachable | BRD-001 NFR17 |
| NFR17.2 | Metrics | `identity_principals_resolved_total{outcome="ok\|status_denied\|not_found\|error"}`, `identity_users_provisioned_total{outcome="created\|existing\|failed"}`, `identity_admin_lookups_total{outcome}` | BRD-001 NFR17 |
| NFR17.3 | Latency | A principal-resolution duration histogram against the p95 target | NFR2.1 |
| NFR17.4 | Alerts | Provisioning failure rate > 1% for 10 min; database unreachable for 1 min; dead-man on zero resolutions for 10 min | NFR1.1 |
| NFR19.2 | Audit | Provisioning, activation, and admin lookup events; no subject plaintext, credentials, or tokens | BR1.8; NFR19 |
| NFR17.5 | Tracing | Correlation id propagated from the gateway through provisioning and audit | BRD-001 NFR17 |

Dashboards: provisioning rate, resolution latency, denial reasons by status.
