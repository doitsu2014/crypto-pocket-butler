# Observability Design — u2-authorisation

**Upstream:** `observability-requirements.md` (NFR17.1–NFR17.6), `reliability-design.md`, `team.md` (health, metrics, tracing).

## Metrics

| Metric | Type | Labels | Emitted by |
|---|---|---|---|
| `authorisation_decisions_total` | counter | `operation_class`, `outcome`, `reason` | The decision, once per call |
| `authorisation_decision_duration_seconds` | histogram | `path` (in_process, c4_endpoint) | The decision boundary; the C4 route adds the hop |
| `authorisation_status_lookup_errors_total` | counter | `cause` (timeout, unavailable, malformed) | The status adapter, both variants |
| `authorisation_audit_write_failures_total` | counter | `context` (denial, role_change) | The audit adapter |
| `authorisation_role_changes_total` | counter | `outcome` (success, failed) | The role-change use case |
| `authorisation_cache_age_seconds` | gauge | — | The outage cache, on read |
| `authorisation_cache_entries` | gauge | — | The outage cache, on read |

The decision metric is emitted before the response is returned, so a request that fails on the
audit write still appears — with `reason=audit_unavailable`.

## Structured logging

- One event per role change: `identity.role.change.requested` and `identity.role.changed`, each
  with `actor`, `target`, `previous_role`, `new_role`, `outcome`, and the correlation id. The
  reason text is carried in the audit record and the context, never in the log line.
- One warn per audit-write failure, with the correlation id and the operation class.
- **Never logged:** tokens, subject plaintext, credentials, secrets, full wallet addresses,
  request bodies.
- Every line carries `trace_id` and `correlation_id` so a decision joins its caller's trace and
  its audit row.

## Tracing

- The decision is a child span of the caller's request: attributes `operation_class`, `outcome`,
  `reason`, `role`, and the resource kind. No subject or email.
- The C4 route opens a server span and links its parent to the caller's trace id, so a decision
  made on behalf of Account or Portfolio is visible end to end.
- A role change is one span covering: catalogue check, intent audit write, provider write,
  outcome audit write. A span that ends after step 3 with `outcome=failed` is the signal that the
  intent entry exists without its outcome.

## Alerts and escalation

| Alert | Threshold | Severity | Runbook action |
|---|---|---|---|
| Audit write failures | any over 5 minutes | Page | Check the audit service; denials are failing closed |
| Status lookup errors | > 1% of decisions over 5 minutes | Page | Check Identity Postgres; the outage cache is masking it |
| Decision p95 latency | above the path budget for 10 minutes | Ticket | Profile the path; a warm dependency means code |
| Decision p95 latency | above 2× the path budget for 10 minutes | Page | Escalation |
| Role-change failures | > 5% over 30 minutes | Ticket | Check Supabase Auth admin API |
| Cache age | > 30 seconds observed | Ticket | The cache is serving beyond its bound |

## Dashboards

One board: decisions by class and outcome, denial reasons, latency by path and outcome, status
lookup errors, audit-write failures, role changes, cache age and entries. It lives on the
existing Grafana stack; this unit adds no new observability infrastructure.

## SLI/SLO

| SLI | Target |
|---|---|
| Decision availability — a **user-visible success**: allowed, or denied with its audit entry recorded | 99.9% over 30 days |
| Designed failures — 503 on `audit_unavailable` or a cold-cache status outage, tracked as their own SLI | Reported, not budgeted into availability |
| Latency compliance (requests inside their path budget) | 99% over 30 days |
| Denial audit completeness (denials with a recorded entry) | 100% — a gap is a defect, not a budget |
| Role-change audit completeness (intent and outcome both recorded) | 100% |

The audit-completeness SLI is a correctness floor rather than a rate: any miss is an incident,
which is why the write gates the response.
