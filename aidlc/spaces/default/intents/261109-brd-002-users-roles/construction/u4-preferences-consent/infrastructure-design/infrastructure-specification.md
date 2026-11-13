# Infrastructure Specification — u4-preferences-consent

**Upstream:** `performance-design.md`, `security-design.md`, `scalability-design.md`, `reliability-design.md`, `observability-design.md`, `logical-components.md`, `components.md`, `functional-spec.md`, `contract-summary.md`.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute model | **No new container.** The use cases are Identity modules; the outbox drainer is a background task started by the service's composition root | The unit owns no deployable |
| Networking — ingress | Edge → Gateway → Identity `/v1/me/...`; the C6 `/internal/...` routes are internal-only | Deny-by-default |
| Storage | Identity schema: `user_preferences` and `consent_audit_outbox` created, `consent_records` altered by migration `0005_preferences_consent` | One schema, one role |
| Environments | Production Swarm only; `deployment/localhost` for development | Project rule |
| IaC approach | `infra/swarm-stack.yml`: **no change** — no new service, secret, or environment variable | The drainer runs inside the existing Identity process |
| Resource sizing | Identity API 3 replicas × 0.5 CPU / 512 MB | Existing stack |
| Migration | `0005_preferences_consent.up.sql` / `.down.sql` — an ALTER of `consent_records` plus two new tables, ten DDL steps, and the role/trigger changes listed in `tech-stack-decisions.md` | The table already exists (see that file) |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| Supabase Postgres (identity schema) | database | Existing; the three tables plus two indexes each on consent history and the outbox | `identity_app` for the application; `identity_retention` (NOLOGIN) for erasure only |
| Audit service (C9) | internal API | 5 s connect / 10 s read, three jittered attempts, `Idempotency-Key` header (a contract amendment, see `security-design.md`) | The drainer's only outbound call |
| Prometheus + Alertmanager | metrics and alerts | **Not declared in the repository**; the scrape job is specified in `monitoring-design.md` and lands with that platform work | `consent_*` metrics are emitted but not collected until then |
| Loki / Tempo / Grafana | logs, traces, dashboards | Same platform gap | One dashboard definition |
| Docker secrets / Vault | secrets | None new: the existing Identity service token covers C9 | — |

## Roles and privileges

| Role | Grants | Why |
|---|---|---|
| `identity_app` | `SELECT, INSERT` on `consent_records`; `SELECT, INSERT, UPDATE` on `user_preferences`; `SELECT, INSERT, UPDATE, DELETE` on `consent_audit_outbox` | The application appends consent; the drainer marks and deletes outbox rows |
| `identity_erasure` | **`LOGIN`** (its own secret, materialised from Vault), `SELECT (user_id)` and `DELETE` on `consent_records`, `user_preferences`, and `consent_records_legacy_archive` | The erasure job **connects as this role directly** — no `SET ROLE`, no membership. That is deliberate: a membership in `identity_app` would hand it `DELETE` by inheritance and undo the revoke. PostgreSQL needs `SELECT` on any column a `DELETE … WHERE` reads, so `SELECT (user_id)` is granted beside `DELETE` |
| `identity_migrator` | already holds `ALL` on the Identity schema tables from `0001` — including `DELETE` on `consent_records` | Stated rather than omitted: the trigger refuses `identity_app` and `identity_erasure`'s write paths, but the migration role keeps its privileges because migrations must be able to alter the table. It is the one role that could delete outside the erasure path, and it is used only by `identity-migrate` |
| Trigger rule | `BEFORE UPDATE OR DELETE` raises for `identity_app`; permits `identity_erasure` on a delete; permits `identity_migrator` | The application can never rewrite history; the erasure job and the migration role are the two sanctioned writers |

## Where the drainer runs

The drainer is a `tokio` task in the **`identity-api`** binary (started by its composition root, not by
`identity-worker`). That is why the scrape target is `identity-api:8083` and why its metrics are
collected with the API's. `identity-worker` (8085) is not involved and needs no scrape entry for
this unit.

## Shared Infrastructure

| Shared Resource | Owner Unit | Consumer Units | Access Boundary |
|---|---|---|---|
| Identity service process | Identity (all units) | u1–u6 | In-process modules |
| C6 consent-state query | u4 | the Account service (source connection), U7 via the gateway | JWT plus service-token allowlist |
| Audit service (C9) | Audit service | every service | Per-service token, write-only |
| `identity_erasure` login role | u4 (schema owner) | u5's erasure job | Its own credential; delete-only on the three tables |
| `consent_records_legacy_archive` | u4 | the erasure job | Holds `risk`/`age` events copied by a down migration; erased with the account, retained until then |

## Why no new container

The drainer is a `tokio` task in the existing process with its own interval and its own metrics.
A separate worker container would add a deployable, a health check, and a failure domain for one
periodic `SELECT … UPDATE` loop — and the backlog alert already catches a wedged drainer.

## What must be provisioned before release

1. The **C6 amendment**: consent-state endpoint, its response shape, the 404/409/422/503 statuses, and `account-api` as a consumer.
2. The **C9 amendment**: an `Idempotency-Key` header honoured by `/internal/events`, and the outbox-completion wording.
3. The Prometheus service and scrape job (see `monitoring-design.md`).
