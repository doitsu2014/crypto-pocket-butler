# Observability Requirements — u5-account-lifecycle

**Upstream:** `functional-spec.md` (W1–W5), `rules.md` (BR5.9, BR5.10), `team.md` (health, metrics, tracing).

## Metrics

| ID | Metric | Type | Labels | Why |
|---|---|---|---|---|
| NFR17.1 | `account_status_transitions_total` | counter | `from`, `to`, `actor_kind` (user, admin, system) | Every edge, by who caused it |
| NFR17.2 | `account_transition_refusals_total` | counter | `reason` (illegal_edge, not_admin, missing_reason) | Refused attempts, so a client bug is visible |
| NFR17.3 | `deletion_requests_total` | counter | `event` (requested, cancelled, erased) | The lifecycle over time |
| NFR17.4 | `deletion_erasure_failures_total` | counter | `step` | A stuck erasure — the alert signal |
| NFR17.5 | `deletion_due_backlog` | gauge | — | Open requests past their deadline, emitted by the sweep |
| NFR17.6 | `deletion_sweep_duration_seconds` | histogram | — | Guards the batch budget |
| NFR17.7 | `status_audit_outbox_pending` / `_oldest_age_seconds` | gauge | — | Audit delivery lag, shared with the worker loop |
| NFR17.8 | `deletion_sweep_last_pass_timestamp_seconds` | gauge | — | **Sweep liveness.** A dead worker emits neither the backlog gauge nor failure counters, so the alert is `time() - value > 900` (plus an `absent()` branch), not a rising backlog |

## Structured logging

- One line per transition: account id, edge, actor id, outcome. **Never the reason text** (it can
  carry personal detail) and never an email, subject, or token.
- One line per sweep pass: claimed, erased, failed, skipped-restored.
- One line per erasure failure: account id, step, attempt count.

## Tracing

- A transition is a span covering the status write, the transition insert, and the outbox insert.
- An erasure is one span per account covering anonymisation and each cascade step, so a slow or
  failing step is attributable.

## Health and readiness

The service's existing `/healthz` and `/readyz`. The sweep backlog is an alert, not readiness.

## Alerts

| Alert | Threshold | Severity | Routes to |
|---|---|---|---|
| Erasure failures | any `deletion_erasure_failures_total` over 15 minutes | P1 (page) | Identity on-call — accounts are not being erased |
| Due backlog | > 500 past-deadline requests for 15 minutes | P1 (page) | The sweep is not keeping up |
| Audit outbox backlog | oldest row > **5 minutes**, or the gauge absent for 10 minutes | P1 (page) | Transitions are unaudited in delivery, and the 5-minute alert sits below the 5-minute SLO rather than behind it |
| Sweep dead | `time() - deletion_sweep_last_pass_timestamp_seconds > 900`, or the series absent | P1 (page) | The erasure SLO is being breached silently |
| Transition refusals | > 5% of attempts over 30 minutes | P2 (ticket) | A client is sending illegal edges |
| Sweep batch duration | p95 > 60 s | P2 (ticket) | Batch sizing |

## SLIs / SLOs

| SLI | Target | Window |
|---|---|---|
| Transition success (an accepted change has its status, log row, and outbox row) | 99.9% | 30-day rolling |
| Erasure completion within 1 hour of the deadline | 99.9% | 30-day rolling |
| Audit delivery lag | 99.9% within 5 minutes | 30-day rolling |
| Status/log agreement (no status without its transition) | 100% — a miss is an incident | 30-day rolling |

## Dashboards

One board: transitions by edge and actor, refusals by reason, deletion requests by event, due
backlog, erasure failures by step, sweep duration, audit outbox lag.
