# Observability Requirements — u2-authorisation

**Upstream:** `functional-spec.md` (W1–W5), `rules.md` (BR2.10, BR2.11, BR2.12), `team.md` (Deployment — every service exposes `/healthz`, `/readyz`, `/metrics`).

## Metrics

| ID | Metric | Type | Labels | Why |
|---|---|---|---|---|
| NFR17.1 | `authorisation_decisions_total` | counter | `operation_class`, `outcome` (allowed, denied), `reason` | The unit's primary signal: what was decided and why |
| NFR17.2 | `authorisation_decision_duration_seconds` | histogram | `path` (in_process, c4_endpoint) | Guards the performance budgets |
| NFR17.3 | `authorisation_status_lookup_errors_total` | counter | `cause` (timeout, unavailable) | u1 as a dependency; drives the availability alert |
| NFR17.4 | `authorisation_audit_write_failures_total` | counter | `context` (denial, role_change) | A non-zero value means requests are failing closed |
| NFR17.5 | `authorisation_role_changes_total` | counter | `outcome` (applied, failed) | Privileged-change volume and failure |
| NFR17.6 | `authorisation_cache_age_seconds` | gauge | — | How stale the status cache is during an outage |

Every metric that can be non-zero on an error path carries the error label; there is no
metric that only counts successes.

## Logging

- Structured logs (`tracing`), one line per role change and one per audit-write failure.
- **Never logged:** the token, the subject plaintext, a credential, a secret, a full wallet
  address, or the reason text of a denial beyond the reason enum (BR2.12).
- A denial is recorded in the audit store, not duplicated into the log at info level; the log
  carries the correlation id so the two can be joined.

## Tracing

- A decision is a span inside the caller's request span, with `operation_class`, `outcome`, and
  `reason` as attributes; the C4 endpoint opens its own server span and links back to the
  caller's trace id.
- The role change is one span covering the audit-intent write, the provider write, and the
  outcome write, so a partial failure is visible as a span that ended without its last step.

## Health and readiness

- `/healthz` — the process is up (Swarm update gate).
- `/readyz` — the decision is ready **and** u1 is reachable; the audit store is reported but does
  not fail readiness (a denial failing closed is the designed behaviour, not an outage).

## Alerts

| Alert | Threshold | Below which SLO |
|---|---|---|
| Audit write failures | any non-zero over 5 minutes | Denials are failing closed before the availability SLO is breached |
| Status lookup errors | > 1% of decisions over 5 minutes | Availability SLO of 99.9% |
| Decision p95 latency | above the path's budget for 10 minutes (warning); above 2× for 10 minutes (escalation) | Performance target |
| Role-change failures | > 5% over 30 minutes | Administrative path health |

Thresholds sit below the SLO so remediation starts before the SLO is breached.

## Dashboards

One board per unit: decisions by class and outcome, latency by path, cache age, audit failures,
role changes. It reuses the existing self-hosted Grafana stack; this unit adds no new
infrastructure.
