# Logical Components — u1-user-registry

**Upstream:** `functional-design/functional-spec.md`, `nfr-requirements/*`, `components.md`.

## Component inventory inside the Identity service

| Logical component | Responsibility | Depends on | Failure domain |
|---|---|---|---|
| `UserRegistry` (domain) | `User`, `ShreddingKey`, `UserStatus`, `CountryCode`, subject hash/cipher value objects; pure rules BR1.1–BR1.10 | Nothing | Cannot fail externally |
| `ProvisionUser` (application) | Resolve-or-create on first authentication; gate call; key reference; profile refresh | Domain, ports (`UserRepository`, `JurisdictionGate`, `AuditSink`, `Clock`) | Main failure surface |
| `ResolvePrincipal` (application) | Look up by subject hash, map to the principal, apply status denial | Domain, `UserRepository`, `AuditSink` | Reads only |
| `AdminLookup` (application) | Exact-match lookup within the metadata boundary | Domain, `UserRepository`, `AuditSink` | Reads only |
| `PgUserRepository` (infrastructure) | sqlx reads/writes of `users` and `shredding_keys`; transactional provision | Postgres | Database failure |
| `HttpJurisdictionGate` (infrastructure) | C8 client with the gate's budget | u6 gate | Gate failure (fail closed) |
| `HttpAuditSink` (infrastructure) | Audit writes with idempotency key | Audit service | Audit failure (surfaced) |
| `UserHttp` (http) | C3 routes, DTO mapping, RFC 7807 problems, metrics | Application | Request boundary |

## Failure domains and blast radius

- **Postgres unavailable** → all identity operations return `503`; blast radius: the whole Identity API.
- **Gate unavailable** → provisioning refused; existing users still resolve. Blast radius: first authentications only.
- **Audit unavailable** → writes surface the failure; reads unaffected.
- **Process panic** → the Identity API replica restarts; blast radius: that replica, not the data.

## Shared resources

| Resource | Shared with | Isolation |
|---|---|---|
| Identity schema | u2, u3, u4, u5 | Separate tables per unit; no cross-unit table access |
| Audit service | All services | HTTP with its own budget |
