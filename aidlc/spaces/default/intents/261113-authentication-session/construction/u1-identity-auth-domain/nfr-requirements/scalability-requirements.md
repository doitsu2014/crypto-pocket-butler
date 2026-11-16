# U1 Identity Auth Domain — Scalability Requirements

**Unit:** U1 `u1-identity-auth-domain`  
**Source:** Requirements Analysis NFR1–NFR3, NFR9; Functional Design; existing Docker Swarm topology.

| ID | Requirement | Target / verification |
|---|---|---|
| NFR1.5 | Identity session reads sustain the established per-replica load without weakening ownership or revocation semantics. | 50 requests/second per replica while meeting NFR1.1 latency; validate with a repeatable load run. |
| NFR2.1 | Concurrent refresh/revocation remains correct across multiple tabs and devices. | Deterministic two-actor tests and a database-backed conflicting-revocation integration test; no lost or resurrected session. |
| NFR2.2 | Side-effecting operations remain idempotent under retry and duplicate submission. | Stable idempotency key per logical operation; repeated same-key/same-body requests return the original result; same key with different body is refused. |
| NFR2.3 | Session histories are returned in bounded pages and can grow without unbounded response or query cost. | Cursor pagination with bounded page size; load test covers the NFR1 per-replica target. No sharding or cache is required by current evidence. |
| NFR9.1 | Expired session records are retained only for the approved recent-history period. | Purge session rows 90 days after expiry; audit records follow the independently governed audit retention policy. |
| NFR9.2 | Identity API replicas remain stateless with respect to browser sessions. | Session authorization is derived from validated JWT and Identity-owned persistent policy/session records; no sticky browser-session affinity is required. |

## Capacity and growth assumptions

- Requirements define the measurable request rate as 50 session-list requests/second per replica; they provide no user-count or annual growth projection, so none is invented.
- Existing deployment scales Identity API replicas through Docker Swarm. Database connections, provider calls and Notification/Audit integrations must remain bounded per replica.
- No distributed cache, sharding, read replica, new service, or queue is introduced for this Unit.
