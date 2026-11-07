# Observability Design — u10-analytics

Inputs: `nfr-requirements/observability-requirements.md`.

| Metric | Type | Labels |
|---|---|---|
| `analytics_request_duration_seconds` | histogram | route, status |
| `analytics_request_errors_total` | counter | route, status |
| `analytics_events_total` | counter | event_name, consent_result, outcome |
| `analytics_aggregate_runs_total` | counter | status |
| `analytics_aggregate_duration_seconds` | histogram | — |
| `analytics_suppressed_cohorts` | gauge | metric |
| `analytics_erased_events_total` | counter | outcome |
| `analytics_health` | gauge | — |

Alerts: page on 5xx > 0.15% for 10 min (1.5× burn on the 0.1% error budget) or readiness failing
2 min; ticket on a missed nightly aggregate or suppressed cohorts > 20%.

SLIs: ingest availability 99.9% (good = 202 + 403; bad = all 5xx; 401/422 excluded as client
errors); aggregate freshness ≤ 24 h; erase success 100%.

Logs: ids and event names only; never a property value, userRef, or subject.
Traces: request → C3 → insert; run → aggregate → audit.

Dashboards: ingest rate and consent refusals, aggregate freshness, flag evaluations, erasure.

NFR references: NFR17.1–NFR17.8.
