# Logical Components — u5-account-lifecycle

**Upstream:** `security-design.md`, `reliability-design.md`, `performance-design.md`, `functional-spec.md`, `components.md` (AccountLifecycle).

## Component inventory

| Component | Layer | Responsibility | Depends on |
|---|---|---|---|
| `TombstoneLookup` | infrastructure (identity registry) | The precondition this design records: `find_by_subject_hash` / `find_by_auth_subject` consult `users.erased_subject_hash` before concluding the subject is unknown, and return `SubjectErased` rather than `None` — `resolve_with_origin` maps that to **403**, never to provisioning | `users` |
| `StatusMachine` | domain | The permitted edges and the refusal of any other (BR5.1) | none |
| `TransitionAccount` (use case) | application | Admin disable/restore and the guarded `pending→active`; one transaction | `StatusRepository` (port), `TransitionLog` (port), `AuditOutbox` (port), `ObservationSink` (port) |
| `RequestDeletion` (use case) | application | Open a request, set `pending_deletion`, record the deadline | the same four ports |
| `RestoreAccount` (use case) | application | Owner cancels within the window; admin restore is W1 | the same four ports |
| `RunErasure` (use case) | application | One account: transition, anonymise, cascade, mark erased; idempotent | `StatusRepository`, `TransitionLog`, `AuditOutbox`, `ErasureTargets` (port), `Clock` |
| `StatusRepository` (port) | application | Read the current status; write it inside a transaction | implemented by `PgStatusRepository` |
| `TransitionLog` (port) | application | Append a transition; read the newest per user | implemented by `PgTransitionLog` |
| `AuditOutbox` (port) | application | Enqueue an audit intent inside the caller's transaction | implemented by `PgStatusAuditOutbox` |
| `ErasureTargets` (port) | application | Anonymise the tombstone; delete `user_personal`; destroy the key; delete the Auth user; delete consent/preferences; call each service's erasure endpoint | implemented by `IdentityErasureTargets` (identity) and `ServiceErasureClient` (other services) |
| `SweepWorker` | infrastructure worker | Claim due requests with the lease, call `RunErasure`, record attempts/backoff/terminal state | `RunErasure`, Postgres |
| `StatusAuditDrainer` | infrastructure worker | Claim outbox rows, post to C9 with an idempotency key, mark delivered/dead | Postgres, C9 |
| `AccountsRoute` | http | C3/C7 routes, DTOs, status codes | the use cases |

## Import direction

Every arrow points inward. `application` sees only its ports; `infrastructure` implements them;
`http` maps DTOs and calls use cases. The workers are started by the composition root.

## Boundaries and failure domains

- **One failure domain** inside the Identity process for the request paths.
- **The sweep is a separate unit of work**: a stuck cascade cannot block a user operation, because
  it holds a lease rather than a lock.
- **The audit drainer is separate again**: a C9 outage grows the outbox and alerts; it never blocks a
  transition.

## Blast radius

| Failure | Blast radius |
|---|---|
| Status machine bug | Every transition |
| Sweep stopped | Nothing erased; the liveness alert fires |
| Cascade step down | Deletions for that step only; other steps continue per account |
| Postgres down | Everything in the service |

## Shared resources

| Resource | Shared with | Isolation |
|---|---|---|
| Identity Postgres (`identity_app`) | The rest of the Identity service | Own schema; `0006` tables and columns |
| `identity_retention` | u4 | Delete-only, used by the cascade |
| Supabase Auth admin API | The platform | One credential, held by Identity |
| C9 | Every service | Outbox-mediated |

## What this unit deliberately does not own

No new container, no queue. The sweep and the drainer are tasks in the existing `identity-worker`
and `identity-api` loops.
