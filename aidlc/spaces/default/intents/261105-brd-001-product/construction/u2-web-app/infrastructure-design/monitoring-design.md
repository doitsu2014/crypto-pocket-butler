# Monitoring Design — u2-web-app

Inputs: `nfr-requirements/security-requirements.md` NFR17.1, `infrastructure-specification.md`.

## SLIs and SLOs

| SLI | SLO | Window |
|---|---|---|
| Edge availability for the SPA (2xx/3xx) | 99.9% | 30 days |
| Client telemetry error rate (failed bootstrap/chunk) | < 0.5% of sessions | 7 days |
| Lighthouse performance p95 (mobile, cold) | ≤ 4 s | 7 days |

## Metrics and alerts

- nginx: `stub_status` plus `nginx-prometheus-exporter` (added in code generation) scraped by the
  observability stack: request count by status class, `nginx_connections_*`, plus a Grafana panel.
  The `/healthz` static endpoint is liveness only.
- Client telemetry (gateway route): `web_client_errors_total{code,route}`,
  `web_bootstrap_failures_total`. The gateway validates `route` against the fixed SPA route list
  and `errorCode` against a closed enum, drops unknown values and caps label cardinality at 20
  routes × 10 codes. A browser session is counted by a session id the SPA sends once per page
  load; the error rate is errors ÷ sessions.
- Alerts: `WebBootstrapFailures` (any increase in 10 min, P3), `WebEdge5xx` (>1% over 10 min, P2),
  `WebCspViolations` (any report, P3) via the CSP `report-to` collector.

## Logs and traces

Telemetry events land in Loki through the gateway with the correlation id and route, no PII.
No client-side tracing.

## Dashboard

Error rate, bootstrap failures by code, CSP violations, Lighthouse trend, edge status codes.
