# U1 Identity Auth Domain — Reliability Requirements

**Unit:** U1 `u1-identity-auth-domain`  
**Source:** Requirements Analysis NFR2, NFR3; FR3.7–FR3.8, FR10.1–FR10.3; Functional Design.

| ID | Requirement | Target / verification |
|---|---|---|
| NFR3.1 | Identity auth operations meet the monthly availability objective at the Identity API boundary. | 99.9% availability over a rolling 30-day window. Planned maintenance is reported separately, not silently removed from the measurement. |
| NFR3.2 | Outside a declared provider outage, auth requests fail at no more than the allowed rate. | ≤0.1% of auth requests fail over a rolling 30-minute alert window for non-provider failures. Provider-outage failures are classified separately and still count toward the availability objective. |
| NFR3.3 | A Supabase Auth outage degrades only provider-dependent auth operations. | Already-issued, valid access tokens continue to work until expiry; sign-in, signup, refresh, MFA challenge and provider revocation return recoverable safe failures. No fallback authentication path. |
| NFR3.4 | Provider, Notification and Audit integration calls have bounded time and retry behavior. | Use configured timeouts, bounded exponential backoff with jitter for transient failures only, and idempotency for side effects; permanent validation/authorization failures are not retried. |
| NFR3.5 | Provider revocation failure never produces a false local success. | Keep the local session row unchanged when provider revocation fails; surface a recoverable failure. Revoke-others reports each remaining session explicitly. |
| NFR3.6 | Notification delivery failure does not undo a successful sign-in. | Preserve sign-in result, expose delivery failure to operations, and retry within the existing bound with the same idempotency key. |
| NFR3.7 | Audit-intent creation is transactionally coupled to security-sensitive state changes. | If the local audit intent cannot commit, roll back the corresponding mutation; after commit, delivery failure remains retryable and dead-lettered work is actionable. |
| NFR3.8 | Session revocation behavior is consistent with the accepted stateless-token limitation. | Browser discards the revoked token; Identity applies local policy; up to one-hour residual validity for an already-issued access token is documented and not misrepresented as immediate global invalidation. |

## Recovery and degradation

- No operator action is needed to resume auth after Supabase Auth recovers; the next attempt may succeed.
- Provider outage errors use the stable non-revealing problem contract and disclose no upstream endpoint or response body.
- Outbox dead letters and failed notification deliveries are surfaced to operations; they are never silently discarded.
- This Unit adds no multi-region or blue/green strategy; production remains the existing Docker Swarm deployment with health-gated rolling updates.
