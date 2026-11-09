# Infrastructure Specification — u7-ledger

Inputs: `nfr-design/*`, `inception/domain-design/components.md`, `functional-design/`, C5/C7/C10.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute | `ledger` (2 replicas, 0.25 vCPU / 256 MiB) and `ledger-worker` (1 replica, 0.5 vCPU / 512 MiB) | API stateless; worker owns reconcile and the anonymise job |
| Networking | No published port; gateway → `internal`; C5/C4/C10 over `internal`; `public` for Postgres only | Ledger is internal |
| Storage | Supabase Postgres schema `ledger`, roles `ledger_svc` (runtime) and `ledger_anonymiser` (one-shot job) | Append-only grants plus a narrow anonymiser |
| Environments | Production only | Team decision |
| IaC | `infra/swarm-stack.yml`: both services, networks, health checks, update config; migrate/anonymise jobs are release-job one-shots | Infrastructure is code |
| Sizing | API pool 12; worker 8; ceiling 4 | 1 M transactions/month |
| Health | API `/healthz` 8097, worker 8098; `/readyz` Postgres only; Swarm gate `/healthz`, release smoke `/readyz` | C5 outage must not remove tasks |
| Update | Rolling, `start-first` API, `stop-first` worker, `failure_action: rollback` | Reconcile must not overlap |
| Scaling | Manual `docker service scale ledger=N` on p95 > 400 ms for 10 min, with the latency ticket alert and a runbook step | No autoscaler |
| Service tokens | `ledger.service_token_registry` rows seeded by the deploy job for gateway, Portfolio, Reporting and `smoke-reader`, each with an operation allow-list; rotation insert-then-delete | Internal auth |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| Supabase Postgres `ledger` | Database | transactions, corrections, transfer_pairs, cost_basis_lots, pnl_snapshots, ledger_subjects; partitioned by user hash at 24 M rows; Supavisor transaction pooling; statement cache off; daily backup, PITR | RPO ≤24 h, RTO ≤4 h |
| `ledger_svc` | Runtime role | SELECT/INSERT on append-only tables; SELECT/INSERT/UPDATE on `cost_basis_lots`, `pnl_snapshots`, `transfer_pairs`; **no UPDATE on `ledger_subjects`**; no DELETE except lots prune | No UPDATE/DELETE on transactions/corrections |
| `ledger_anonymiser` | Anonymise-only role | UPDATE on `ledger_subjects` only | The API opens a short-lived connection with this DSN **only** inside the C7 anonymise handler; the runtime pool uses `ledger_svc` |
| `ledger_migrator` | Migration role | CREATE schema/tables/roles/grants, applied only by the migrate job | Not available to the running services |
| Docker secrets | Bootstrap | `ledger_database_url`, `ledger_migrate_database_url`, `ledger_anonymiser_database_url`, `ledger_smoke_token`, `ledger_outbound_token` (C5/C4/C10) | `*_FILE` done |
| `internal` overlay | Network | Encrypted; gateway, Account, Asset Registry, Audit, Portfolio, Reporting, Job Orchestration, ledger | — |
| `public` overlay | Network | Postgres only | TLS |
| Loki/Prometheus/Tempo/Grafana | Observability | Existing stack | — |

## Shared Infrastructure

| Resource | Owner | Consumers | Boundary |
|---|---|---|---|
| API Gateway | u1 | u2, u7 | C7 routes |
| Account (C5) | u4 | u7 | transaction pages |
| Asset Registry (C4) | u1 | u7 | asset resolution |
| Audit (C10) | u12 | u7 | write-only |
| Job Orchestration (C11) | u11 | u7 | reconcile run |
| Observability stack | u1 | all | ingestion |

## Cost and recovery

- Incremental: 2 × 0.25 + 1 × 0.5 vCPU on existing capacity; no new managed service. Priced figure
  TBD with the platform owner.
- Cluster rebuild from `infra/swarm-stack.yml` + Supabase backup, RTO ≤4 h; rehearsal before launch.

## Non-resources

- No message queue, no cache, no separate OLAP store.
