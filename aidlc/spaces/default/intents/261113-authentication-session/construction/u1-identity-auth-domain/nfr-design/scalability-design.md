# U1 Identity Auth Domain — Scalability Design

**Unit:** U1 `u1-identity-auth-domain`  
**Inputs:** `nfr-requirements/scalability-requirements.md`, `performance-requirements.md`, `functional-design/functional-spec.md`. NFR coverage: NFR1.5, NFR2.1, NFR2.2, NFR2.3, NFR9.1, NFR9.2.

## Scaling approach

- Keep the existing Identity API stateless between requests and scale replicas using the project's current Docker Swarm service configuration and health-gated rolling updates.
- Keep user policy, session, recovery and gate records in the Identity-owned Postgres schema. Do not add a cache, read replica, shard, shared store or new deployable service for this Unit.
- Use bounded cursor pagination for session history; indexes and query shape are designed against Identity's owner-scoped predicates and measured with representative histories.
- Bound Postgres connections and outbound provider/Notification/Audit concurrency per replica. Capacity settings must fit the database connection budget and be measured under the 50 requests/second per replica objective.
- Session revocation, signup authorization and MFA state remain strongly consistent at the Identity boundary; no stale read model is introduced.

## Concurrency and idempotency

- Use database constraints and atomic create-or-get behavior to prevent duplicate InternalUser rows under concurrent first requests.
- Use stable idempotency keys for revocation, recovery completion and notification requests; same-key/same-payload replay returns the original result, while a changed payload is rejected.
- Treat simultaneous refreshes as provider rotation races. The client/session flow must coalesce or serialize refresh attempts as required by the provider contract; Identity does not persist a replacement token or guess which tab owns the current token.
- Verify two concurrent actors with deterministic tests plus a Postgres integration test for conflicting revocations; no numeric concurrent-user load requirement is assumed.

## Retention and capacity

- Purge Identity session rows 90 days after session expiry via the existing Identity-owned maintenance/worker mechanism once its executor is assigned during implementation planning.
- Audit retention remains controlled by the Audit service policy; Identity never deletes or edits Audit rows.
- No user-count growth estimate is available. Revisit storage and partitioning only after measured growth or query performance demonstrates need.
