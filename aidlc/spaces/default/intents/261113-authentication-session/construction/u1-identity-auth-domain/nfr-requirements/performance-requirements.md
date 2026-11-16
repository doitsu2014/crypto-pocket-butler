# U1 Identity Auth Domain — Performance Requirements

**Unit:** U1 `u1-identity-auth-domain`  
**Source:** `inception/requirements-analysis/requirements.md` NFR1, NFR2; `construction/u1-identity-auth-domain/functional-design/functional-spec.md`; `construction/u1-identity-auth-domain/functional-design/rules.md`; answers in `nfr-requirements-questions.md`.

| ID | Requirement | Target / verification |
|---|---|---|
| NFR1.1 | Identity's session-list operation meets the existing end-to-end latency target at expected per-replica load. | p95 ≤ 300 ms and p99 ≤ 800 ms at 50 requests/second per replica. Measure at the Identity API boundary; the end-to-end route must remain within the same bound. |
| NFR1.2 | Session-list and revocation operations must not perform unbounded work as device history grows. | Use bounded page size and cursor-based pagination; no request returns an unbounded session set. The contract defines cursor semantics; no total dataset cap is inferred here. |
| NFR1.3 | Provider-dependent operations have bounded latency and fail recoverably. | Provider calls use the existing contract's configured timeout/retry policy; retry transient errors only, with jitter and idempotency for side effects. No new numeric provider timeout is established by U1. |
| NFR1.4 | Session data reads do not use a cache that can expose stale ownership or revocation state. | Read the Identity-owned session view from its authoritative store; any future caching requires an explicit consistency and invalidation design. |

## Measurement

- Record request latency histograms by operation and status, with dimensions limited to operation, outcome class and service instance; never label metrics with user id, token, session id or full device fingerprint.
- Evaluate the p95/p99 objective with a repeatable test at 50 requests/second per Identity replica and report both the Identity-boundary and end-to-end measurements.
- Exclude provider calls from the local database-read latency bucket while retaining a total request latency measure; separate provider-outage timeouts from internal processing regressions.
