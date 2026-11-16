# U1 Identity Auth Domain — Observability Design

**Unit:** U1 `u1-identity-auth-domain`  
**Inputs:** `nfr-requirements/observability-requirements.md`, `reliability-requirements.md`, `performance-requirements.md`, `inception/contract-design/contract-summary.md`.

## Signals

- **Health:** preserve `/healthz`, `/readyz` and `/metrics`; readiness reflects local ability to serve and required database state, with provider dependency health represented separately.
- **Latency:** histograms by operation and outcome for session list, revocation, signup gate, MFA and recovery; record Identity-boundary and total request latency.
- **Errors:** counters by operation, status class and dependency class; distinguish provider outage from Identity validation, authorization, persistence, Notification and Audit delivery failures.
- **Audit delivery:** pending rows, retry count, oldest pending age, delivered count and dead-letter count. Do not label with event payload fields.
- **Notification delivery:** attempt/outcome and bounded retry metrics; never include notification payload or device key in labels.

## SLI and alerts

- Availability SLI: successful eligible Identity auth operations divided by total eligible Identity auth operations over a rolling 30-day window; target 99.9%. Provider outage remains visible and counts toward this SLI.
- Non-provider auth error alert: rate above 0.1% over a rolling 30-minute window. Provider outage alert is separate and based on its effect on the availability SLI and provider-dependent operation failures.
- Session-list latency objective: p95 ≤ 300 ms and p99 ≤ 800 ms at 50 requests/second per replica; alert thresholds should leave response time before the objective is breached, finalized with the actual collection system.
- Alert on nonzero dead-lettered audit work and on sustained growth/age of pending audit or notification delivery.

## Log and trace policy

- Structured logs include correlation id, operation name, stable safe error type and outcome class.
- Do not log request/response bodies for auth operations. Redact credentials, tokens, factor seeds/codes, recovery codes, full device fingerprints, raw IP and provider response text before any diagnostic logging.
- Trace context propagates over gateway-to-Identity and Identity internal HTTP calls; span attributes are bounded and contain no secrets or personal identifiers.
- Metrics and trace collection/dashboard infrastructure is not deployed today. This Unit exposes the signals and does not add Prometheus, Loki, Tempo or Grafana.
