# Audit — Monitoring Design

## Metrics and KPIs

| Metric | Source | Threshold | Why it matters |
|---|---|---|---|
| `audit_append_total{class,outcome}` | `/metrics` | failure ratio > 1% for 5 min | Security appends block their callers |
| `audit_append_duration_seconds` | `/metrics` histogram | p95 > 100 ms for 10 min | The caller's budget absorbs this latency |
| `audit_append_rejected_total{reason}` | `/metrics` | any sustained rise | An emitter is breaking BR1.2 or BR1.3 |
| `audit_session_check_total{result}` | `/metrics` | deny rate rising | Support over-reach or a broken console |
| `audit_query_duration_seconds` | `/metrics` | p95 > 800 ms | Query cost against a growing table |
| `pg_stat_activity` pool waiters | Postgres exporter | > 0 for 1 min | The connection budget is saturated |
| `audit_partition_horizon_days` | retention service metric | < 60 days | The next release must extend partitions before appends fail |
| `audit_retention_last_success_timestamp` | retention service metric | > 48 h | Backstop for a broken schedule |
| `audit_export_total{outcome}` | `/metrics` | > 5 successful per hour per caller, read from `audit_exports` | Export exfiltration signal |

## Alerts

| Alert | Condition | Severity | Routes to |
|---|---|---|---|
| Audit writes failing | failure ratio > 1% over 5 min | P1 | On-call; security appends are blocking callers |
| Audit silent | no appends for 15 min while other services report traffic | P1 | On-call; the trail is down |
| Partition horizon short | fewer than 60 days of future partitions | P1 | On-call; deploy a release or run the fallback job |
| Retention heartbeat missed | Job Orchestration heartbeat missed | P1 | On-call; primary missed-schedule signal (FR10.2) |
| Append latency | p95 > 100 ms for 10 min | P2 | On-call |
| Retention stale | no successful run in 48 h | P3 | Ticket |
| Export spike | export count per caller above threshold in 1 h | P2 | Security review of the request factor |

## SLIs and SLOs

| SLI | SLO target | Measurement window |
|---|---|---|
| Append success ratio | 99.9% | 30-day rolling |
| Append latency | p95 < 25 ms, p99 < 60 ms | 30-day rolling |
| Session check latency | p95 < 60 ms | 30-day rolling |
| Retention currency | a successful run at least every 24 h | 7-day rolling |

## Logs and tracing

- **Logs**: JSON to stdout, collected by the shared Loki. Fields: timestamp, level, service, trace
  id, correlation id, event id (when applicable), rule id (on refusal), outcome. Never a payload,
  token, address, or user id.
- **Tracing**: OTLP to the shared Tempo. The append span is a child of the caller's span; the
  session-check span is a child of the gateway's; retention and export spans are roots.
- **Dashboard**: one Grafana board with four rows — volume and failures by class; latency
  percentiles; rejection reasons and session-check outcomes; retention currency, partition horizon,
  and export volume.

## Sources

- `construction/u12-audit/nfr-design/observability-design.md` — SLI, alert, and dashboard definitions
- `construction/u12-audit/nfr-design/scalability-design.md` — partition horizon and capacity thresholds
- `construction/u12-audit/nfr-design/reliability-design.md` — health and readiness
- `construction/u12-audit/nfr-requirements/observability-requirements.md` — NFR17.1 to NFR17.8
- `construction/u12-audit/functional-design/functional-spec.md` — WF1 to WF7
