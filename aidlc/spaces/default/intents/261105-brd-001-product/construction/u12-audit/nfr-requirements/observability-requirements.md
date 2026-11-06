# Audit — Observability Requirements

## Service level indicators

| ID | Requirement |
|---|---|
| NFR17.1 | **Append success ratio** — successful appends divided by total append attempts, over a 30-day rolling window. SLO 99.9%, tied to NFR1.1. |
| NFR17.2 | **Append latency** — server-side p95 and p99, measured at the handler, split by event class. |
| NFR17.3 | **Session check outcome** — counts of allow, deny-inside-window, deny-expired, and deny-scope, recorded because a rising deny count is how support over-reach or a broken console surfaces. |
| NFR17.4 | **Retention currency** — the age of the newest retention run per category; if the last successful `audit_logs` run is older than 48 hours, the retention schedule is broken even though the data is merely kept longer. |

## Metrics

| Metric | Type | Why |
|---|---|---|
| `audit_append_total{class,outcome}` | counter | Volume and failure ratio per event class |
| `audit_append_duration_seconds` | histogram | NFR2.1 against the p95 and p99 targets |
| `audit_append_rejected_total{reason}` | counter | Validation rejections by rule, so an emitter that breaks BR1.2 or BR1.3 is visible immediately |
| `audit_session_check_total{result}` | counter | NFR17.3 |
| `audit_query_duration_seconds` | histogram | Admin query cost against NFR2.3 |
| `audit_export_total{outcome}` | counter | Export volume per caller, which is also the exfiltration signal from the security requirements |
| `audit_retention_deleted_total{category}` | counter | What each sweep removed |
| `audit_retention_last_success_timestamp` | gauge | NFR17.4 |
| `audit_health` and `audit_error_rate` | gauge | The project rule's one health metric and one error-rate metric per service |

## Logging

| ID | Requirement |
|---|---|
| NFR17.5 | Structured logs carry the correlation id from the caller and the event id, and never the event payload, a token, a JWT, or a wallet address. |
| NFR17.6 | Every refusal logs the rule that refused it by id (BR1.2, BR1.3, BR5.1) so a rejection is traceable to a rule without reproducing it. |

## Tracing

| ID | Requirement |
|---|---|
| NFR17.7 | An append continues the caller's trace: the emitter's span is the parent, and the audit insert appears as a child, so a slow security-event append is visible in the caller's own trace. |
| NFR17.8 | Retention runs and exports start their own trace with the run or export id as an attribute. |

## Alerts (threshold set below the SLO breach)

| Alert | Condition | Action |
|---|---|---|
| Append failure | 5xx ratio above 1% for 5 minutes | On-call investigates; security-event appends are blocking their callers |
| Append latency | p95 above 100 ms for 10 minutes | On-call investigates before callers' budgets absorb it |
| Audit gap | no successful append for 15 minutes while other services report traffic | Treat as a broken audit path, not as quiet traffic |
| Retention stale | no successful `audit_logs` run in 48 hours | Run the retention job by hand and find why the schedule missed |
| Export spike | more than 5 exports in an hour by one caller | Security review of the export request factor |

## Dashboards

One board: append rate and failure ratio by class, latency percentiles, rejection reasons,
session-check outcomes, retention currency, and export volume per caller. Everything on it maps to
an SLI above; nothing decorative.

## Sources

- `construction/u12-audit/functional-design/functional-spec.md` — WF1 through WF7
- `construction/u12-audit/functional-design/rules.md` — BR1.1 to BR1.4, BR3.2, BR4.1, BR5.1
- `inception/requirements-analysis/requirements.md` — NFR1, NFR17, NFR19
- `inception/contract-design/contract-summary.md` — C10 and C11
