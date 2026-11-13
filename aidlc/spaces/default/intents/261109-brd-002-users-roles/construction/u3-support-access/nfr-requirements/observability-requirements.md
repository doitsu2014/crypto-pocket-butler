# Observability Requirements — u3-support-access

**Upstream:** `functional-spec.md` (W1–W5), `rules.md` (BR3.7, BR3.13),
`team.md` (Deployment — every service exposes `/healthz`, `/readyz`, `/metrics`),
`requirements.md` (NFR21, NFR24), the existing `services/identity/http/src/metrics.rs` and the
consent/status outbox gauges.

## Metrics

The unit adds no metrics library: Identity already renders Prometheus exposition by hand in
`http/src/metrics.rs`, and these follow its existing shapes — `*_deliveries_total`, `*_pending`,
`*_oldest_age_seconds`, `*_last_pass_timestamp_seconds`, `*_write_failures_total`.

| ID | Metric | Type | Labels | Why |
|---|---|---|---|---|
| NFR17.1 | `support_grants_total` | counter | `operation` (request, approve, revoke), `outcome` (accepted, refused, failed) | Volume and refusal rate per write path; `failed` is the 503s |
| NFR17.2 | `support_reads_total` | counter | `outcome` (served, refused, unavailable, failed) | The one read path: served, refused by a rule, refused by a source outage, failed otherwise |
| NFR17.3 | `support_read_duration_seconds` | histogram | `outcome` | Guards the read budgets; buckets must resolve the 150 ms p95 and 300 ms p99 targets, so no bucket coarser than 50 ms below 500 ms |
| NFR17.4 | `support_metadata_source_errors_total` | counter | `source` (account, portfolio), `cause` (timeout, unavailable, malformed) | Names the failing owner; a read is refused because one of these answered wrongly |
| NFR17.5 | `support_audit_write_failures_total` | counter | `cause` (read_append, refusal_append) | Non-zero means a read or a refusal failed closed |
| NFR17.6 | `support_outbox_deliveries_total` | counter | `outcome` (delivered, retried, dead) | Delivery health; `dead` is the one nobody can retry automatically |
| NFR17.7 | `support_outbox_pending` | gauge | — | Backlog size, read at scrape time from the table, not pushed by the drain pass |
| NFR17.8 | `support_outbox_oldest_age_seconds` | gauge | — | How long an undelivered audit intent has waited |
| NFR17.9 | `support_outbox_last_pass_timestamp_seconds` | gauge | — | The drainer's heartbeat; staleness is the only way to see a dead drainer |
| NFR17.10 | `support_grant_decision_errors_total` | counter | `cause` (timeout, unavailable) | The grant lookup as a dependency of the read |

Every metric that can be non-zero on an error path carries its error label; no metric counts only
successes. No label carries a grant id, a user id, an email address, or the grant's reason text —
labels are bounded vocabularies only (BR3.13, NFR22.1).

## Logging

- Structured logs (`tracing`), one line per grant mutation, one per refused read, and one per
  audit-write failure. A **served** read is recorded in the audit store, not duplicated into the
  log at info level.
- Every line carries the request's correlation id, the action, the actor and target ids, and the
  refusal code where applicable — identifiers, never the reason text, an email address, a token,
  or a source payload.
- A source failure logs the source name and the error class, not the source's response body.

## Tracing

- A grant mutation is one span covering the grant write and its outbox insert, so a commit that
  did not reach its outbox row is visible as a span missing its last step.
- The metadata read is a parent span with two child spans, one per source, both under the 500 ms
  deadline; the audit append is a third child span. A refused read shows which child answered
  last, which is what distinguishes a slow owner from a failed audit append.
- The drain pass is one span per row with the outbox row id as an attribute — an identifier that
  joins the trace to the row, and to nothing the audit store holds.

## Health and readiness

| ID | Requirement |
|---|---|
| NFR17.11 | `/healthz` reports only that the process is up; it is the gate the Swarm rolling update waits on (`infra/swarm-stack.yml`). |
| NFR17.12 | `/readyz` reports the database and the audit store. Account and Portfolio reachability is reported but does **not** fail readiness: a source outage refuses the read (the designed behaviour) and must not drain every Identity replica out of rotation for every other route. |
| NFR17.13 | The outbox heartbeat is not part of either endpoint — a lagging drainer does not fail readiness, because mutations remain correct while delivery lags. It is visible as NFR17.9 and alerted on (below). |

## Alerts

| Alert | Threshold | Below which target |
|---|---|---|
| Read refusal by a source outage | `support_metadata_source_errors_total` > 0.2% of reads **per source** over 10 minutes | The read's own budget is the product of its four dependencies (~99.6%, ~0.4% refusals at 99.9% each — NFR1.2), so 0.2% for one source is a third of the path's allowed refusals and well below the steady-state 0.1% per dependency |
| Read unavailable, any cause | `support_reads_total{outcome="unavailable"}` > 0.4% of reads over 30 minutes | The 99.6% composed budget of NFR1.2; a sustained breach means the product is no longer holding |
| Audit append failing closed | `support_audit_write_failures_total` any non-zero over 5 minutes | The read and refusal paths are refusing by design before the SLO is breached |
| Outbox backlog not draining | `support_outbox_oldest_age_seconds` > 15 minutes, **or** `support_outbox_last_pass_timestamp_seconds` older than 5 minutes, **or** `absent(support_outbox_last_pass_timestamp_seconds)` | Audit delivery lag; the alerts do not depend on the drainer running, which is the failure they exist for (NFR1.8) |
| Dead-lettered audit rows | `rate(support_outbox_deliveries_total{outcome="dead"})` > 0 over 15 minutes | Audit events that will never arrive without a human. The action is the replay statement in NFR1.5, run by the on-call under `identity_migrator` as the runbook names — not a code change |
| Read latency | p95 above 150 ms or p99 above 300 ms for 10 minutes | The read budget (NFR2.7) |
| Write latency | p95 above 80 ms for 10 minutes | The write budget (NFR2.1, NFR2.3) |

Thresholds sit below the SLO so remediation starts before it is breached. Each alert names an
action in the runbook; the dead-row alert's action is the replay, not a code change.

## Dashboards

One board for the unit on the existing self-hosted Grafana: writes by operation and outcome, the
read by outcome and latency, source errors by owner, audit failures, and the outbox trio (pending,
oldest age, last pass) on one row so a lagging drainer reads at a glance. It reuses the existing
stack; this unit adds no new infrastructure.