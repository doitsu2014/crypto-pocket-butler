# U1 Identity Auth Domain — Reliability Design

**Unit:** U1 `u1-identity-auth-domain`  
**Inputs:** `nfr-requirements/reliability-requirements.md`, `observability-requirements.md`, `functional-design/functional-spec.md`, `inception/contract-design/contract-summary.md`. NFR coverage: NFR3.1, NFR3.2, NFR3.3, NFR3.4, NFR3.5, NFR3.6, NFR3.7, NFR3.8.

## Dependency isolation

- Use separate bounded concurrency limits, connection pools and timeout budgets for Supabase Auth, Notification and Audit/outbox delivery so a slow dependency cannot exhaust all Identity request capacity.
- Use typed external failures mapped at the application boundary. Retry only transient failures with bounded exponential backoff and jitter; side effects reuse stable idempotency keys. Never retry permanent validation or authorization failures.
- Avoid introducing a generic circuit-breaker threshold not supported by measurements. If a breaker is adopted in implementation, it is per dependency and its open/half-open behavior must preserve fail-closed auth decisions and automatic recovery.

## Provider outage behavior

- Previously issued access tokens remain usable only while each service independently validates them and they have not expired.
- Sign-in, signup, refresh, MFA challenge and provider revocation fail with a safe recoverable response when required provider operations are unavailable. There is no local credential or cached-provider-success fallback.
- Provider recovery requires no operator intervention; subsequent attempts resume normally.

## Side-effect safety

- Provider-first revocation: do not change the local session row or record success until provider revocation succeeds or provider semantics confirm the session is already absent.
- Revoke-all and credential-change flows return per-session outcomes; partial failure is explicit and retryable, never reported as full success.
- Notification is best-effort after successful sign-in: bounded delivery failure cannot roll back sign-in; use idempotency to avoid duplicate alerts.
- Security-sensitive local mutation and Audit outbox intent commit atomically. Delivery failures remain pending and retryable; dead-letter work is surfaced to operations.

## Health and deployment

- Preserve `/healthz`, `/readyz` and `/metrics` on the existing Identity serving component. Readiness must reflect whether the process can serve safely, not require Supabase Auth to be available for all already-authenticated requests.
- The current Swarm health check gates rolling updates. Keep its health-gated rollback configuration; no blue/green, canary or new environment tier is introduced here.
- Availability SLI is measured at the Identity API boundary over rolling 30 days with a 99.9% objective. Provider outages remain classified separately but still count in availability; alert when non-provider auth failures exceed 0.1% over rolling 30 minutes.
