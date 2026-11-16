# U1 Identity Auth Domain — Performance Design

**Unit:** U1 `u1-identity-auth-domain`  
**Inputs:** `nfr-requirements/performance-requirements.md`, `nfr-requirements/scalability-requirements.md`, `functional-design/functional-spec.md`, `inception/contract-design/contract-summary.md`. NFR coverage: NFR1.1, NFR1.2, NFR1.3, NFR1.4, NFR1.5.

## Session read path

- Serve the Identity-owned device/session view from Identity Postgres; do not cache authorization, MFA policy, ownership or revocation status.
- Use indexed queries aligned with caller ownership and session state, and cursor-based pagination with a bounded page size. The exact index definition belongs to the Identity migration design and must preserve query plans for the list route.
- Return only display label, coarse location, last-seen time, current marker and state. Exclude raw IP, full user-agent and device fingerprint.
- Use the existing sqlx/Postgres pool and outbound HTTP clients with explicit bounded pool sizes/timeouts. Set pool limits from the measured per-replica request/load envelope; do not invent capacity values without benchmarks.
- The Identity-boundary target is p95 ≤ 300 ms and p99 ≤ 800 ms at 50 requests/second per replica. Since this equals the end-to-end target, the test must report both component and end-to-end latency and flag network/gateway overhead as a known budget risk, not imply extra margin exists.

## Provider-bound operations

- Keep provider calls out of the session-list read path unless a use case explicitly requires them.
- Use per-dependency timeouts and bounded concurrency for Supabase Auth; do not let provider waits exhaust the general request pool.
- Retry transient failures only, with bounded exponential backoff and jitter. Side-effecting retries use the same logical idempotency key. Never retry validation or authorization refusals.
- Do not cache successful provider responses as an authentication fallback. Required auth decisions fail closed when trustworthy state is unavailable.

## Verification

- Benchmark the session-list operation at 50 requests/second per replica with representative account history and paginated reads.
- Record p50, p95, p99, error rate and connection-pool saturation for Identity and gateway-inclusive requests.
- Include concurrent two-actor session reads/revocation in deterministic and database-backed tests; load testing must not weaken ownership or freshness assertions.
