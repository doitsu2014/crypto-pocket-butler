# Logical Components — u3-identity

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`,
`scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`,
`tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

This is the component-level view of where the NFR patterns apply, bridging to Infrastructure
Design.

## Component inventory

| Logical component | Responsibility | Scaling | Failure domain |
|---|---|---|---|
| Identity API | `resolvePrincipal`, consent, preferences, sessions, export/deletion requests, sign-up gate, support grants | 3 stateless replicas | One task; the gateway routes around it |
| Identity worker | Export assembly and deletion execution; lease-based | 1+ workers | Separate process/loop; a dead worker's lease is re-claimed |
| Identity Postgres schema | `User`, `ConsentRecord`, `SupportAccessGrant`, `ExportRequest`, `DeletionRequest`, `JurisdictionRule`, `ShreddingKey`, `DeviceSession` | Supabase managed | Service-owned schema; no cross-schema access |
| Vault key store | Per-subject `ShreddingKey` material | Supabase managed | External dependency; a key failure fails deletion, never silently |
| Export bucket | Private `identity-exports` archive | Supabase Storage | External dependency; owner-only signed URL |
| Audit client | Writes C10 events | per request | External dependency; support reads refuse on failure |
| Supabase Auth client | Validate JWT, revoke sessions, delete users, pre-sign-up hook | per request | External dependency; degraded as designed |

## Boundaries

- The API and the worker share the schema but not a process; both are the same service unit.
- Identity is the only writer of its schema; no other unit reads it directly.
- Outbound dependencies: Audit (C10), Account/Ledger/Portfolio privacy operations (C5/C7/C8
  change requests), Supabase Auth, Vault, Storage, and Job Orchestration (C11 run endpoint).

## Blast radius

- A failed Identity API task affects request routing for its replica only; the gateway retries
  another task.
- A failed worker affects only the records it holds; the lease lets another worker resume.
- A compromised JWT exposes only that user's own records and consent; it cannot grant admin or
  support capability.
- A failed Vault read fails deletion at the key step and leaves it retryable; it never deletes data
  without the key.

## Shared resources

- Postgres connection pool per replica (20), total 60 under the Supabase budget.
- The gateway's rate limiter and the Audit service are shared with every other unit; Identity
  treats them as bounded dependencies.
- No shared cache or message queue.

NFR references: NFR1.1, NFR13.1, NFR20.1.
