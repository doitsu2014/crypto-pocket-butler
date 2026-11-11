# Observability Requirements — u4-preferences-consent

**Upstream:** `functional-spec.md` (W0–W5), `rules.md` (BR4.6, BR4.7), `team.md` (health, metrics, tracing).

## Metrics

| ID | Metric | Type | Labels | Why |
|---|---|---|---|---|
| NFR17.1 | `consent_state_queries_total` | counter | `outcome` (answered, error) | The gate's volume — one per connection attempt |
| NFR17.2 | `consent_state_queries_with_missing_total` | counter | `has_missing` (true, false) | State answers that reported a non-empty `missing` list. **One series per query, never summed across kinds** — a refusal is the calling service's decision, not this unit's, so this measures what the unit actually observes |
| NFR17.3 | `consent_events_total` | counter | `kind`, `action` | Grants and withdrawals by kind |
| NFR17.4 | `preference_writes_total` | counter | `outcome` (applied, refused) | Settings activity and validation refusals |
| NFR17.5 | `consent_state_duration_seconds` | histogram | — | Guards the gate's latency budget |
| NFR17.6 | `consent_audit_write_failures_total` | counter | — | A non-zero value means a withdrawal completed unrecorded |

Every error path has its own counter; no metric only counts success.

## Logging

- One structured line per withdrawal: `kind`, `previous_state`, `new_state`, `policyVersion`,
  `user_id`, correlation id. Never the reason text of a policy (there is none) and never a
  credential.
- Validation refusals log the field name and the offending value's **set membership**, not the
  value itself.
- **Never logged:** names, emails, tokens, subjects, wallet addresses, or the content of a
  preference value beyond its enum name.

## Tracing

- The consent-state query is a span with `missing_kind` attributes; it links to the caller's trace
  so a refused connection is traceable from the source-connection path into this unit.
- A withdrawal is a span covering the previous-state read, the insert, and the audit write.

## Health and readiness

The service's existing `/healthz` and `/readyz` cover this unit; no new endpoint. The database
dependency it reports is the same one every Identity unit uses.

## Alerts

| Alert | Threshold | Severity | Routes to |
|---|---|---|---|
| Consent audit-write failures | any non-zero over 5 minutes | P1 (page) | Identity on-call |
| Consent-state errors | > 1% over 5 minutes | P1 (page) | Identity on-call — connections are being refused |
| Gate state with missing acceptance | a step change in `has_missing="true"` (> 3× the 7-day baseline) | P2 (ticket) | Product/Identity — a policy or UI change may be blocking users. The alert cannot see a connection the caller allowed despite a missing kind; it sees the state the unit answered |
| Undelivered outbox rows | any row older than 15 minutes | P1 (page) | Identity on-call — audit intents are not reaching C9 |
| Consent-state p95 | above 40 ms for 10 minutes | P2 (ticket) | Identity team |
| Preference refused writes | > 5% of writes over 30 minutes | P2 (ticket) | A client is sending invalid values |

## SLIs / SLOs

| SLI | SLO target | Measurement window |
|---|---|---|
| Consent-state query success (answered, never a silent empty) | 99.9% | 30-day rolling |
| Consent-event durability (acknowledged write is readable afterwards) | 100% — a miss is an incident | 30-day rolling |
| Withdrawal audit intent durability (an accepted withdrawal has its outbox row) | 100% — guaranteed by the transaction | 30-day rolling |
| Audit delivery lag (outbox row posted to C9 and marked delivered) | 99.9% within 5 minutes | 30-day rolling |
| Consent-state latency within budget | 99% | 30-day rolling |

## Dashboards

One board: gate volume and refusals by missing kind, consent events by kind and action, preference
writes and refusals, state-query latency, audit-write failures. Panels link to the runbooks.
