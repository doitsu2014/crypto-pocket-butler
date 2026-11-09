# Monitoring Design — u10-analytics

Inputs: `nfr-design/observability-design.md`, `reliability-design.md`.

## Metrics & KPIs

| Metric | Source | Threshold | Why |
|---|---|---|---|
| `analytics_request_duration_seconds` | API | p95 < 120 ms | Ingest budget |
| `analytics_request_errors_total` | API | < 0.1% 5xx | SLI |
| `analytics_events_total` | API | — | Volume and refusal mix |
| `analytics_aggregate_runs_total` | Worker | success daily | Freshness |
| `analytics_aggregate_duration_seconds` | Worker | < 120 s | Budget |
| `analytics_suppressed_cohorts` | Worker | < 20% | Quality |
| `analytics_erased_events_total` | API | errors 0 | Erasure |
| `analytics_health` | `/readyz` | 1 | Swarm gate |

## Alerts

| Alert | Condition | Severity |
|---|---|---|
| Ingest 5xx | > 0.15% over 10 min (1.5× burn on the 0.1% budget) | Page |
| Readiness | any task unready 2 min | Page |
| Missing aggregate | no successful run in 24 h | Ticket |
| Suppression spike | suppressed cohorts > 20% | Ticket |
| Erase failure | any erase error | Page |
| API latency | p95 > 120 ms for 10 min | Ticket; runbook scales `analytics` to N (ceiling 4) |

## SLIs / SLOs

| SLI | SLO | Window |
|---|---|---|
| Ingest availability (202+403 good; 5xx bad; 401/422 excluded) | 99.9% | 30 days |
| Aggregate freshness | ≤ 24 h | daily |
| Erase success | 100% | per request |

## Logs & Tracing

- JSON: ids and event names only, never property values, userRef, or subject.
- OTLP: request → C3 → insert; run → aggregate → audit.

## Dashboards

- Ingest rate, consent refusals, 5xx; aggregate freshness and suppression; erasure.
