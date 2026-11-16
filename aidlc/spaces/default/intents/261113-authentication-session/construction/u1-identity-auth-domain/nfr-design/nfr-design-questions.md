# NFR Design — U1 Identity Auth Domain Questions

Answer each with a letter (A–D), or **X** plus your own words if none fits.

## Q1. [architect] How should U1 meet the session-list latency target without weakening freshness?

U1 has p95 ≤ 300 ms and p99 ≤ 800 ms at 50 requests/second per replica, while ownership and revocation state must remain authoritative. The team has not approved a cache or extra data store.

- A. Use bounded cursor pagination, indexed Identity-owned queries and bounded Postgres/HTTP connection pools; do not cache session authorization or revocation state
- B. Add a short-TTL distributed cache for session lists and invalidate it after every revoke
- C. Add a read replica for session-list reads and accept replication lag
- D. Raise the latency budget until production measurements are available
- X. Other (please specify)

[Answer]: A. Use bounded cursor pagination, indexed Identity-owned queries and bounded Postgres/HTTP connection pools; do not cache session authorization or revocation state. **Mode:** guided

## Q2. [security engineer] How should the Identity service isolate and protect provider operations?

Supabase Auth is a critical dependency. Provider failure must not create a fallback authentication route or consume all Identity capacity; provider responses must not leak into API errors.

- A. Per-dependency bounded concurrency and timeouts; retry transient failures only with jitter/idempotency; translate provider errors to typed safe outcomes; fail closed for required auth decisions
- B. One global circuit breaker and one shared concurrency pool for all provider operations
- C. Cache provider success decisions and use them as authentication fallback during outages
- D. Retry every failure until the provider responds, then return its response text
- X. Other (please specify)

[Answer]: A. Per-dependency bounded concurrency and timeouts; retry transient failures only with jitter/idempotency; translate provider errors to typed safe outcomes; fail closed for required auth decisions. **Mode:** guided

## Q3. [operations engineer] How should U1 meet its availability and alerting objectives?

The NFRs set 99.9% monthly availability, separate provider failures from other failures, and a 0.1% non-provider auth failure alert threshold over 30 minutes. Metrics collection is not currently deployed.

- A. Expose bounded-cardinality health, latency and outcome metrics at the Identity API; alert on the stated SLI/error thresholds and keep provider-outage impact separately visible
- B. Add Prometheus, Loki, Tempo and Grafana to the Swarm stack as part of U1
- C. Log errors only and assess availability manually each month
- D. Treat provider-outage failures as excluded from both the alert and the 99.9% availability SLI
- X. Other (please specify)

[Answer]: A. Expose bounded-cardinality health, latency and outcome metrics at the Identity API; alert on the stated SLI/error thresholds and keep provider-outage impact separately visible. **Mode:** guided

## Q4. [architect] What logical failure boundaries should this Unit use?

Identity combines user policy, provider operations, Notification delivery and Audit outbox delivery. Sign-in must survive Notification failure; audit intent must commit with sensitive state changes; provider failures must not consume all request capacity.

- A. Keep one Identity service boundary with separate application ports and bounded dependency-specific capacity for Supabase Auth, Notification and Audit; Notification failure is best-effort, while local audit intent is transactional
- B. Split MFA, sessions, recovery and alerts into separate deployable services
- C. Put provider, Notification and Audit operations behind one shared synchronous call path with common retries
- D. Add a message queue for all three integrations
- X. Other (please specify)

[Answer]: A. Keep one Identity service boundary with separate application ports and bounded dependency-specific capacity for Supabase Auth, Notification and Audit; Notification failure is best-effort, while local audit intent is transactional. **Mode:** guided

[Answer]:
