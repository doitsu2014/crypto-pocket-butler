# Audit — Observability Design

## Instrumentation

`tracing` spans plus an OpenTelemetry exporter to the shared Tempo; metrics through the
Prometheus exporter on `/metrics`; structured JSON logs to Loki. One correlation id per request,
taken from the incoming header or generated, echoed in the response and in every log line.

### The spans that must exist

| Span | Attributes |
|---|---|
| `append` | `event.action`, `event.class`, `event.source_service`, `event.outcome` |
| `append.validate` | `rule.id` when it refuses |
| `session.check` | `session.id`, `result` (allow / deny_expired / deny_scope / deny_role) |
| `query` | `filter.action_prefix`, `filter.window_hours`, `page.size` |
| `export` | `export.id`, `row_count`, `outcome` |
| `retention.run` | `run.id`, `category`, `deleted_count`, `outcome` |

Never an attribute: a target value, a payload field, a token, a JWT, an address.

## SLI to metric mapping

| SLI | Metric | SLO |
|---|---|---|
| Append success ratio | `audit_append_total{outcome}` | 99.9% over 30 days |
| Append latency | `audit_append_duration_seconds` | p95 < 25 ms |
| Session-check latency | `audit_session_check_duration_seconds` | p95 < 60 ms |
| Retention currency | `audit_retention_last_success_timestamp` | newest run < 48 h |

## Alerting

| Alert | Expression | Severity | Runbook |
|---|---|---|---|
| Append failure ratio | `rate(audit_append_total{outcome="error"}[5m]) / rate(audit_append_total[5m]) > 0.01` | P1 | Security-event appends are blocking callers |
| Append latency | `histogram_quantile(0.95, rate(audit_append_duration_seconds_bucket[10m])) > 0.1` | P2 | Check pool waiters, then export budget |
| Audit silence | `absent(audit_append_total)` while other services report traffic | P1 | The audit path is down, not quiet |
| Retention heartbeat | missed job heartbeat from Job Orchestration | P1 | Primary missed-schedule alert (FR10.2, five minutes) |
| Retention stale | `time() - audit_retention_last_success_timestamp > 172800` | P3 | Backstop for when the heartbeat itself is wrong |
| Export spike | `increase(audit_export_total{outcome="success"}[1h]) > 5` per caller | P2 | Security review of the caller |

Every alert above maps to a runbook section; an alert with no action is deleted, per the team's
on-call rule.

## Dashboards

One board, four rows: (1) append rate and failure ratio by class, (2) append and session-check
latency percentiles, (3) rejection reasons by rule id and session-check outcomes, (4) retention
currency, partition count, and export volume per caller. Nothing on the board is decorative; each
panel answers a question an on-call engineer would actually ask.

## Logging discipline

Log lines carry ids and outcomes only. A payload is never logged, even at debug level, because a
debug line is a stored record with none of the trail's guarantees. Refusals log the rule id that
refused them, so a validation failure is traceable without reproducing the payload.

## Sources

- `construction/u12-audit/nfr-requirements/observability-requirements.md` — NFR17.1 to NFR17.8
- `construction/u12-audit/nfr-requirements/reliability-requirements.md` — NFR14.3, NFR1.1
- `construction/u12-audit/functional-design/rules.md` — BR1.3, BR1.4, BR3.2, BR5.1
- `construction/u12-audit/functional-design/functional-spec.md` — WF1 to WF7
- `inception/contract-design/contract-summary.md` — C10
