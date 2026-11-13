# Observability Design — u5-account-lifecycle

**Upstream:** `observability-requirements.md` (NFR17.1–NFR17.8), `reliability-design.md`, `security-design.md`.

## Metrics

| Metric | Type | Labels | Emitted by |
|---|---|---|---|
| `account_status_transitions_total` | counter | `from`, `to`, `actor_kind` | The transition use case |
| `account_transition_refusals_total` | counter | `reason` | The transition use case |
| `deletion_requests_total` | counter | `event` (requested, cancelled, erased) | Request/restore/sweep |
| `deletion_erasure_failures_total` | counter | `step` | The sweep |
| `deletion_due_backlog` | gauge | — | The sweep, each pass |
| `deletion_sweep_duration_seconds` | histogram | — | The sweep |
| `deletion_sweep_last_pass_timestamp_seconds` | gauge | — | **End of every pass**, even an empty one — the liveness signal |
| `status_audit_outbox_pending` / `_oldest_age_seconds` | gauge | — | The drainer, **every pass including an empty one (published as 0 and 0)** — a healthy outbox is normally empty, so an absence-based alert would page on a healthy system |

## Structured logging

- One line per transition: account id, edge, actor id, outcome — never the reason text or any PII.
- One line per sweep pass: claimed, erased, failed, skipped-restored.
- One line per erasure failure: account id, step, attempt count.
- **Never logged:** email, subject hash or ciphertext, tokens, wallet addresses.

## Tracing

- A transition is a span over the three writes.
- An erasure is one span per account with a child per cascade step, so the failing step is named.

## Health and readiness

`/healthz` and `/readyz` are the service's. The sweep's liveness is an alert, deliberately not a
readiness condition.

## Alerts

| Alert | Threshold | Severity | Routes to |
|---|---|---|---|
| Erasure step failures | any `deletion_erasure_failures_total` over 15 minutes | P1 (page) | Identity on-call |
| Due backlog | > 500 for 15 minutes | P1 (page) | The sweep is not keeping up |
| Sweep dead | `time() - last_pass > 900`, or the series absent | P1 (page) | The erasure SLO is breaching silently |
| Audit outbox backlog | oldest > 5 minutes, or absent for 10 | P1 (page) | Transitions are undelivered |
| Transition refusals | > 5% over 30 minutes | P2 (ticket) | A client sends illegal edges |
| Batch duration | p95 > 60 s | P2 (ticket) | Batch sizing |

## SLIs / SLOs

| SLI | Target | Window |
|---|---|---|
| Transition success (status + log + outbox accepted together) | 99.9% | 30-day rolling |
| Erasure within 1 hour of the deadline | 99.9% | 30-day rolling |
| Audit delivery lag | 99.9% within 5 minutes | 30-day rolling |
| Status/log agreement | 100% | 30-day rolling |

## Dashboards

One board: transitions by edge and actor, refusals by reason, deletion events, due backlog, erasure
failures by step, sweep duration and liveness, outbox lag.
