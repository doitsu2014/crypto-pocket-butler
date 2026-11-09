# Observability Design — u6-portfolio

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`, `scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`, `tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

| Metric | Type | Labels |
|---|---|---|
| `portfolio_request_duration_seconds` | histogram | route, status |
| `portfolio_request_errors_total` | counter | route, status |
| `portfolio_valuations_total` | counter | status |
| `portfolio_valuation_duration_seconds` | histogram | — |
| `portfolio_unpriced_assets` | gauge | — |
| `portfolio_snapshots_total` | counter | — |
| `portfolio_snapshot_digest_mismatch_total` | counter | — |
| `portfolio_audit_failures_total` | counter | — |
| `portfolio_health` | gauge | — |

Alerts: page on 5xx > 0.05%/10 min, digest mismatch, any audit abandoned, or a run missed twice;
ticket when unpriced assets exceed 10% of a portfolio's asset lines for 30 min. A failed C9
guardrail notification leaves the crossing state inside so the next run retries it and increments
`portfolio_guardrail_alerts_failed_total`.

SLIs: read availability 99.9% (a labelled degraded read counts as good; only 5xx is bad); snapshot
digest verification 100% (on write, on reproduction, and nightly); run success daily.

Dashboards: read latency, unpriced, run status, snapshot growth. Logs: ids/status only; traces
read → C5 → C6 → compute → snapshot.

NFR references: NFR17.1–NFR17.8.
