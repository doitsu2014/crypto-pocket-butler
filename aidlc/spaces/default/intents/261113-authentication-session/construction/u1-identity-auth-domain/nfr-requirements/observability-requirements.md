# U1 Identity Auth Domain — Observability Requirements

**Unit:** U1 `u1-identity-auth-domain`  
**Source:** Requirements Analysis NFR3, NFR5–NFR7; Operations guardrails; Functional Design.

| ID | Requirement | Target / verification |
|---|---|---|
| NFR5.1 | Record security-relevant outcomes with actor/system, action, target, timestamp, sanitized context and outcome. | Append-only Audit record for sign-in, sign-out, failed sign-in, MFA enrol/verify/reset, session revocation, recovery and jurisdiction refusal. Identity outbox entry is committed atomically with local changes. |
| NFR5.2 | Do not include credentials, tokens, factor seeds, recovery codes, full fingerprints, raw IP or full user-agent in logs, errors, analytics, audit or support surfaces. | Dedicated redaction tests assert zero occurrences; logs and audit review use synthetic sensitive markers. |
| NFR7.1 | Expose a health/readiness signal and auth error-rate metric for the serving component. | Preserve `/healthz`, `/readyz`, `/metrics`; expose auth operation outcomes and latency without user/session identifiers. |
| NFR7.2 | Separate provider failures from Identity/application failures for actionable alerting. | Metric labels classify provider dependency, operation and outcome class only; no provider raw error text or user data. |
| NFR3.9 | Alert on non-provider auth failures that exceed the agreed error budget threshold. | Alert when non-provider auth failures exceed 0.1% over a rolling 30-minute window. Provider outage failures have a separate series and alert based on their availability impact. |
| NFR3.10 | Make monthly availability measurable at the Identity API boundary. | SLI = eligible successful auth operations / eligible total auth operations over rolling 30 days; objective 99.9%. Keep the provider-outage classification visible rather than excluding it from the SLI. |
| NFR5.3 | Surface failed outbox delivery, dead letters and notification delivery failures. | Actionable counters and structured warning/error logs; retries and dead-lettering remain observable without logging payload secrets. |

## Logging and metric dimensions

- Structured logs include correlation id, operation name, outcome class and safe error type.
- Metrics use bounded-cardinality labels: operation, status class, dependency class and service instance.
- Never use user id, email, session id, token, full device fingerprint, recovery-case id or raw provider error as a metric label.
- Traces may correlate internal gateway-to-Identity calls by correlation id, but span attributes must not contain credentials or sensitive request bodies.

## Current collection constraint

Services emit structured logs and expose metrics endpoints, but the project does not currently collect Prometheus metrics or distributed traces. Collection/dashboard setup is not added by U1; these requirements describe the signals the Identity surface must expose and the evidence needed for later operational collection.
