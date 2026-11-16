# U1 Identity Auth Domain — Monitoring Design

**Unit:** U1 `u1-identity-auth-domain`  
**Inputs:** `nfr-design/observability-design.md`, `reliability-design.md`, `performance-design.md`, `functional-design/functional-spec.md`. NFR coverage: NFR3.1–NFR3.2, NFR3.9–NFR3.10, NFR5.1–NFR5.3, NFR7.1–NFR7.2.

## Metrics & KPIs

| Metric | Source | Threshold | Why it matters |
|---|---|---|---|
| Identity auth availability SLI | Identity API outcomes | 99.9% successful eligible operations over rolling 30 days | Measures the agreed Identity-boundary objective; provider failures remain visible and count. |
| Non-provider auth failure rate | Identity API outcome counters | Alert above 0.1% over rolling 30 minutes | Distinguishes application regressions from provider outages while preserving actionable alerting. |
| Provider-dependent failure rate | Supabase Auth adapter outcomes | Track separately; alert on material SLI impact | Identifies provider outage without exposing provider details. |
| Session-list latency | Identity API histogram and end-to-end gateway journey | p95 ≤ 300 ms / p99 ≤ 800 ms at 50 requests/second per replica | Detects latency regression against the requirement. |
| Auth operation request rate and errors | Identity API operation/status counters | Establish baseline; alert on error-budget thresholds above | Indicates traffic, saturation and operation-specific failures. |
| DB pool saturation and query duration | Identity SQLx/Postgres instrumentation | Alert before connection exhaustion or session-list SLO breach; threshold set from measured baseline | Detects resource contention without leaking identity data. |
| Audit outbox pending age and dead-letter count | Identity outbox drainer | Alert on any dead-lettered security event and sustained pending-age growth | Prevents silent loss of security-event delivery. |
| Notification delivery failures | Notification adapter outcomes | Alert on sustained failure; individual failure must not fail sign-in | Makes best-effort new-device alert delivery actionable. |

## Alerts

| Alert | Condition | Severity | Routes to |
|---|---|---|---|
| Identity availability burn | Rolling 30-day SLI trends toward or breaches 99.9% | P1/P2 per existing incident policy | On-call operations |
| Non-provider auth errors | More than 0.1% over rolling 30 minutes | P2; escalate if user journey is broadly blocked | Identity on-call |
| Provider dependency outage | Provider-dependent failures materially affect SLI | P1/P2 according to impact | Identity on-call and provider operations |
| Audit outbox dead letter | Any security event is dead-lettered | P1/P2 based on event impact; actionable ticket/page | Identity/Audit on-call |
| Session-list latency | p95/p99 exceeds target under target load | P2 | Identity on-call |
| Secret redaction failure | Any test or log-review match for prohibited sensitive data | P1 security incident | Security on-call |

## SLIs / SLOs

| SLI | SLO target | Measurement window |
|---|---|---|
| Identity auth operation availability | 99.9% successful eligible operations | Rolling 30 days |
| Non-provider auth failure rate | ≤0.1% | Rolling 30 minutes for alerting |
| Session-list latency | p95 ≤ 300 ms and p99 ≤ 800 ms at 50 requests/second per replica | Load test and continuous metrics once collection is deployed |
| Sensitive-data exposure | Zero credentials/tokens/factor secrets/recovery codes/full fingerprints in logs, errors, audit or support surfaces | Dedicated tests and log review per release |

## Logs & Tracing

Emit structured JSON to stdout with timestamp, severity, service, correlation id, operation, safe error type and outcome. Do not log auth request/response bodies or provider text. Propagate correlation/trace context over gateway-to-Identity and internal HTTP calls; span attributes must be bounded and contain no credentials or personal identifiers.

Identity exposes `/healthz`, `/readyz` and `/metrics`, but this Unit does not add collection. The repository currently has no Prometheus scrape config or deployed Loki/Tempo/Grafana stack; the metrics, alerts and dashboards above become active when the project later provisions collectors and alert routing.
