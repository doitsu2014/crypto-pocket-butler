# Audit — Tech Stack Decisions

The stack is fixed by the project rules; this file records what the fixed stack means for this
unit and the few choices it leaves open.

## Fixed by the project

| Concern | Choice | Note |
|---|---|---|
| Language and web framework | Rust, Axum, tower, tokio | .claude/rules/development.md — not substitutable |
| Data access | sqlx, no ORM | Versioned up/down migration pairs |
| Database | Supabase Postgres, one schema `audit` | No cross-schema grants, no Row Level Security. The one-role-per-service rule is relaxed here on purpose: the mandated insert-only audit role cannot be the same role that deletes past-window rows, so the schema carries three roles (see the decision below and NFR19.4) |
| Timestamps | `timestamptz`, stored in UTC | Retention is measured on `recordedAt` |
| Deployment | Docker Swarm, replicated service, digest-pinned images | One manifest entry beside the other services |

## Choices this unit makes

| Decision | Choice | Alternatives rejected |
|---|---|---|
| Storage roles | Three roles: `audit_app` (insert and select on the trail, column-scoped update on the workflow records), `audit_readonly` (select), `audit_retention` (delete on trail partitions only, used by a separate retention service) | A single role with application-level checks — rejected: the database is the last place the append-only rule can be enforced, and BR1.1 is worthless if a code path can bypass it. **This relaxes the one-role-per-service rule**, and the relaxation is recorded rather than implied: the mandated insert-only audit role and a role that deletes past-window rows cannot be the same credential, so the exception is written down in NFR19.4 and in the infrastructure design, with each role taking its own Vault credential |
| Table growth control | Declarative partitioning by `recordedAt` month from the first migration | Unpartitioned table with batched deletes — rejected: six months of data makes every sweep a table scan and a long transaction |
| Idempotency | `Idempotency-Key` header, unique `(emitting_service, idempotency_key)` pair, for the life of the row | Storing a dedupe table keyed by a payload hash — rejected: it duplicates the row and gives no stable id to return. A 24-hour replay window — rejected: nothing is deleted before retention, so the window could not be honoured without a delete grant the append role must not have |
| Export storage | Audit's own object storage path, served by the audit service, 24-hour expiry | Handing back a database dump — rejected: it would need database credentials outside the service |
| Contract shape | Extend contract C10 with query, export, support-session, and privacy-deletion operations | A second contract file — rejected: C10 is already the audit boundary and the consumer set is identical |
| Schema for support sessions | In the `audit` schema, owned by this service | A separate service — rejected: the requirements put support access under audit, and a second service would only re-import the same data over HTTP |
| Vector or text search over events | None | Event search is by action prefix, target, and time. An index over free text would invite payload data into the trail, against BR1.4 |

## Open dependencies

- The shared internal service-token scheme from contract C11 supplies append authentication; if
  that scheme changes, this unit's NFR9.1 changes with it.
- The observability stack (Loki, Prometheus, Tempo, Grafana) is shared and configured in the
  infrastructure design, not here.

## Sources

- `construction/u12-audit/functional-design/rules.md` — BR1.1, BR1.4, BR2.2, BR4.2
- `construction/u12-audit/functional-design/functional-spec.md` — section 3 integration points
- `inception/requirements-analysis/requirements.md` — NFR9, NFR14, NFR19, NFR20
- `inception/contract-design/contract-summary.md` — C10 and C11
