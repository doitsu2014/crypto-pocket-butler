# Observability Design — u6-jurisdiction-gate

**Upstream:** `observability-requirements.md` (NFR17.1–NFR19.3), `reliability-design.md`, `contract-summary.md` (C9).

## Metrics

| Metric | Type | Labels | Purpose |
|---|---|---|---|
| `jurisdiction_checks_total` | counter | `outcome` (allowed/denied/error), `restricted_by` (self_declared/ip/both/none) | Decision volume and mix |
| `jurisdiction_check_duration_seconds` | histogram | `outcome` | Measures the p95 targets |
| `jurisdiction_provider_errors_total` | counter | `kind` (timeout/5xx/invalid) | Provider health |
| `jurisdiction_provider_duration_seconds` | histogram | — | Provider latency budget |
| `jurisdiction_provider_circuit_state` | gauge | — | 0 closed, 1 half-open, 2 open |
| `jurisdiction_list_version` | gauge | `version` | Which list each replica serves |
| `jurisdiction_list_age_seconds` | gauge | — | Staleness |
| `jurisdiction_list_refresh_failures_total` | counter | — | Refresh breaks |
| `jurisdiction_audit_write_failures_total` | counter | — | Evidence gaps |

## Logging

- Structured JSON; correlation id on every line, carried from the caller.
- Log the decision (country codes, signals, list version, outcome); **never** the raw IP, headers, or tokens.
- One INFO line per decision; ERROR for provider/audit/list failures with the correlation id.

## Tracing

- One span for the check, child spans for the provider call and the audit write.
- The correlation id propagates into the provider request and the audit event.

## Alerts

| Alert | Condition | Severity |
|---|---|---|
| Provider failure rate | `provider_errors_total` / provider calls > 5% for 10 min | Page |
| Audit writes failing | `audit_write_failures_total` increases for 5 min | Page |
| List checksum mismatch | Any occurrence | Page |
| Replica on a stale list | `list_age_seconds` > 30 s | Warning |
| List age | `list_age_seconds` > 30 days | Warning |

## SLI/SLO

- **SLI**: successful checks / total checks (a `503` counts as unsuccessful, attributed to its dependency).
- **SLO**: 99.9% gate availability monthly; 99.5% provider-dependency success monthly.
- **Error budget**: 0.1% gate (≈43 min/30 days) and 0.5% provider (≈3.6 h/30 days), reported separately.

## Dashboards

- Decision mix and error rate.
- Latency p50/p95/p99 against the targets.
- Provider health and circuit state.
- List version, age, and per-replica freshness.
- Audit-write success.
