# Observability Design — u4-preferences-consent

**Upstream:** `observability-requirements.md` (NFR17.1–17.6), `reliability-design.md`, `security-design.md`.

## Metrics

| Metric | Type | Labels | Emitted by |
|---|---|---|---|
| `consent_state_queries_total` | counter | `outcome` (answered, error) | The state query |
| `consent_state_queries_with_missing_total` | counter | `has_missing` (true, false) | The state query, **once per query** — never once per kind |
| `consent_state_duration_seconds` | histogram | — | The state query |
| `consent_events_total` | counter | `kind`, `action` | Grant and withdrawal use cases |
| `preference_writes_total` | counter | `outcome` (applied, refused) | The preference write |
| `consent_outbox_pending` | gauge | — | The drainer, each pass |
| `consent_outbox_oldest_age_seconds` | gauge | — | The drainer, each pass |
| `consent_outbox_deliveries_total` | counter | `outcome` (delivered, retryable, dead) | The drainer |
| `consent_withdrawal_rollbacks_total` | counter | `cause` (outbox_failed, insert_failed) | The withdrawal transaction's rollback path — **NFR17.6's audit-failure signal**, which the outbox model otherwise hides |

## Structured logging

- One line per withdrawal: `kind`, `previous_state`, `new_state`, `policy_version`, `user_id`,
  `trace_id`. No free-form text.
- A validation refusal logs the field name and the refusal reason — never the value.
- A drainer permanent failure logs the outbox id and the C9 status, never the payload.
- **Never logged:** names, emails, subjects, tokens, wallet addresses.

## Tracing

- The state query is a span with `has_missing` and `missing_kinds` (enum values), linked to the
  caller's trace so a refused connection traces into this unit.
- A withdrawal is one span: lock, read, insert, outbox insert, commit.
- The drainer's post is a span with the outbox id and the delivery outcome.

## Health and readiness

No new endpoints. `/healthz` and `/readyz` are the service's; the outbox backlog is an alert,
not a readiness condition.

## Alerts

| Alert | Threshold | Severity | Routes to |
|---|---|---|---|
| Undelivered outbox row | oldest age > 15 minutes **or the gauge is absent for 10 minutes** | P1 (page) | Identity on-call — audit intents are not reaching C9. The `absent()` branch is what catches a dead drainer, which emits nothing |
| Withdrawal rollback on outbox failure | any `outbox_failed` over 5 minutes | P1 (page) | Identity on-call — withdrawals are being refused because the audit intent cannot be recorded |
| Permanent delivery failures | any `permanent` outcome over 5 minutes | P1 (page) | Identity on-call — a bad payload or an auth failure against C9 |
| Consent-state errors | > 1% over 5 minutes | P1 (page) | Identity on-call — connections are being refused |
| State answers with a missing acceptance | step change (> 3× the 7-day baseline) | P2 (ticket) | Product/Identity — a policy or UI change may be blocking users |
| State-query p95 | above 40 ms for 10 minutes | P2 (ticket) | Identity team |
| Preference refusals | > 5% of writes over 30 minutes | P2 (ticket) | A client is sending invalid values |

The gate alert can only see what the unit answers; it cannot see a connection the calling service
allowed despite a missing kind. That blind spot is stated here rather than papered over.

## SLIs / SLOs

| SLI | Target | Window |
|---|---|---|
| State query answered successfully | 99.9% | 30-day rolling |
| Accepted withdrawal has its outbox row | 100% (guaranteed by the transaction) | 30-day rolling |
| Audit delivery lag | 99.9% delivered within 5 minutes | 30-day rolling |
| Consent-event durability | 100% — a miss is an incident | 30-day rolling |

## Dashboards

One board: state queries and their `has_missing` split, consent events by kind and action,
preference writes and refusals, state-query latency, outbox pending/oldest/deliveries by outcome.
