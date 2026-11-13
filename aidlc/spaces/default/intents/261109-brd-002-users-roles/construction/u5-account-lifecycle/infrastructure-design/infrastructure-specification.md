# Infrastructure Specification — u5-account-lifecycle

**Upstream:** `performance-design.md`, `security-design.md`, `scalability-design.md`, `reliability-design.md`, `observability-design.md`, `logical-components.md`, `components.md`, `functional-spec.md`, `contract-summary.md`.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute model | **No new container.** The transition use cases are Identity modules; the sweep and the audit drainer are tasks in the existing `identity-worker` loop | The unit owns no deployable |
| Networking — ingress | Edge → Gateway → Identity `/v1/me/...`; the C3/C7 `/internal/...` routes are internal-only | Deny-by-default |
| Storage | Identity schema: `0006_account_lifecycle` adds `users.erased_subject_hash`, alters `deletion_requests`, creates `status_transitions` and `status_audit_outbox`, drops `0001`'s status-keyed unique index | One schema, one role |
| Environments | Production Swarm only; `deployment/localhost` for development | Project rule |
| IaC approach | `infra/swarm-stack.yml`: **no change** — no new service, secret, or variable. The sweep reuses `IDENTITY_SWEEP_INTERVAL_SECONDS` | Existing loop |
| Resource sizing | Identity API 3 replicas, `identity-worker` 1 replica (as configured) | Existing stack |
| Migration | `0006_account_lifecycle.{up,down}.sql`, applied statement-wise through the runner u4 built (nullable columns, backfill, `CONCURRENTLY` indexes, `NOT VALID` + `VALIDATE`) | The live-table constraint `0005` already solved |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| Supabase Postgres (identity schema) | database | The four new/altered objects plus the indexes in `scalability-design.md` | `identity_app`; `identity_retention` for the consent/preference delete |
| Supabase Auth | third-party API | Admin delete of one user per erased account, **first in the cascade** | One credential, held by Identity |
| Audit service (C9) | internal API | Outbox-mediated, idempotency key, 5 s / 10 s, bounded retries | Delivery lag alerts at 5 minutes |
| Other services' erasure endpoints | internal API | Called where one exists; a missing one is recorded, never assumed | Bounded by the contract's timeouts |
| Docker secrets / Vault | secrets | None new: the retention DSN comes from u4's `identity_retention_database_url` | — |
| Prometheus + Alertmanager | metrics and alerts | **Not declared in the repository**; the scrape job for `identity-api:8083` and `identity-worker:8085` is specified in `monitoring-design.md` | Until then alerts are defined but unarmed |

## Roles and privileges

| Role | Grants | Why |
|---|---|---|
| `identity_app` | `SELECT, INSERT` on `users` (its existing grants), `SELECT, INSERT` on `status_transitions` (**insert-only**, with the `0005` trigger pattern), `SELECT, INSERT, UPDATE` on `deletion_requests`, `SELECT, INSERT, UPDATE` on `status_audit_outbox` | The transitions and the sweep's bookkeeping; the log is evidence, so no `UPDATE` |
| existing `identity_app` grants the cascade relies on | `DELETE` on `user_personal`; `EXECUTE destroy_shredding_key` (both from `0001`) | Named so a reader can see the cascade is permitted |
| `identity_retention` | `DELETE` on `consent_records`, `user_preferences`, the archive (u4's grants) | The consent/preference step only; u5 adds nothing to this role |

## Shared Infrastructure

| Shared Resource | Owner Unit | Consumer Units | Access Boundary |
|---|---|---|---|
| Identity service process | Identity (all units) | u1–u6 | In-process modules |
| `users` row and its status | u1 (column) / u5 (transitions) | u2 reads status; u5 writes it through the transition path | In-schema |
| C3/C7 transition surface | u5 | u2 (admin checks), U10 via the gateway | JWT plus service-token allowlist |
| `identity_retention` | u4 | u5's erasure cascade | Delete-only |
| Supabase Auth admin API | platform | Identity service | One credential |
| C9 | Audit service | every service | Per-service token, write-only |

## Why no new container

The sweep is a `tokio` interval in a worker that already runs one; a second worker container would
add a deployable, a health check, and a failure domain for one `SELECT … LIMIT 100` loop whose
liveness is already a metric.

## What must be provisioned before release

1. The **u2 precondition**: the read-versus-write predicate for `pending_deletion` and the
   `account:connect` operation name.
2. The **u1 lookup change**: `find_by_subject_hash`/`find_by_auth_subject` consult
   `users.erased_subject_hash` and return `SubjectErased`; `resolve_with_origin` denies it.
3. **C3 gains a tombstone response** — `403` (or `410`) for an erased subject on
   `/internal/users/by-subject/{subjectHash}`. Without it, every C3 consumer (U2, U3, U4, U7, U8,
   U10) reads an erased subject as "not provisioned" and is free to provision it; only Identity's
   in-process path is fixed by item 2.
4. The Prometheus service and the two scrape jobs.
5. **C7 gains the response codes it genuinely lacks**: `409` (illegal edge, second request) and
   `410` (restore after the window), plus `403`/`422` on the disable path. It already returns
   `recoveryEndsAt`.
6. A test that seeds an erased subject and asserts its token is denied rather than provisioned, cited
   from the CI stages so a build cannot ship with items 1–3 absent.
