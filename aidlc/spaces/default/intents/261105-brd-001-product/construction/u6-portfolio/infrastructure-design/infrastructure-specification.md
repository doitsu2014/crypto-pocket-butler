# Infrastructure Specification — u6-portfolio

Inputs: `nfr-design/*`, `functional-design/`, `inception/domain-design/components.md`, C1/C5–C10.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute | `portfolio` (2 replicas, 0.25 vCPU / 256 MiB) and `portfolio-worker` (1 replica, 0.5 vCPU / 512 MiB); the worker holds a 900 s lease renewed every 60 s and runs the hourly snapshot prune at two years | API stateless; one leased valuation worker |
| Networking | No published port; gateway → `internal`; C5–C10 are all reached over `internal`; `public` carries Postgres only | Portfolio is internal |
| Storage | Supabase Postgres schema `portfolio`, roles `portfolio_svc` and `portfolio_migrator` | Insert-only snapshots; migration separation |
| Environments | Production only | Team decision |
| IaC | `infra/swarm-stack.yml`: both services, networks, health checks, update config; migrate is a release-job one-shot | Infrastructure is code |
| Sizing | API pool 12; worker 8; ceiling 4 | 10k users, 20k portfolios |
| Health | API `/healthz` 8101, worker 8102; `/readyz` Postgres only | C5/C6 outages must not fail readiness |
| Update | Rolling, `start-first` API, `stop-first` worker, `failure_action: rollback` | One valuation executor |
| Scaling | Manual `scale portfolio=N` on snapshot read p95 > 300 ms or fresh read p95 > 800 ms for 10 min | No autoscaler |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| Supabase Postgres `portfolio` | Database | portfolios, membership, targets, guardrails, snapshots (JSONB inputs/lines), overlays, intents, trades; indexes per NFR; hourly snapshot uniqueness; daily backup, PITR | RPO ≤24 h, RTO ≤4 h |
| `portfolio_svc` | Runtime role | SELECT/INSERT on snapshots; full DML on mutable tables; no DELETE on snapshots | Immutability |
| `portfolio_migrator` | Migration role | DDL only via the migrate job | — |
| Docker secrets | Bootstrap | `portfolio_database_url`, `portfolio_migrate_database_url`, `portfolio_smoke_token`, `portfolio_outbound_token` (C5–C10) | `*_FILE` |
| Service-token registry | Auth | Reporting (C8 snapshots), Identity (privacy export/anonymise), Job Orchestration (C11 `POST /v1/internal/run/portfolio-valuation`), smoke-reader; Portfolio's outbound token is allow-listed at C5–C10 | Seeded and rotated by `infra/sql/portfolio-service-tokens.sql` (insert-then-delete, deploy job) |
| `internal` overlay | Network | Encrypted; gateway, Account, Market Data, Ledger, Notification, Audit, Identity, Job Orchestration | — |
| `public` overlay | Network | Postgres | TLS |
| Loki/Prometheus/Tempo/Grafana | Observability | Existing stack | — |

## Shared Infrastructure

| Resource | Owner | Consumers | Boundary |
|---|---|---|---|
| API Gateway | u1 | u2, u6 | C1 routes |
| Account (C5), Market Data (C6), Ledger (C7), Notification (C9), Audit (C10), Identity (C3 privacy) | their units | u6 | service tokens |
| Observability | u1 | all | ingestion |

## Cost and recovery

- Incremental: 2 × 0.25 + 1 × 0.5 vCPU on existing capacity; snapshots ≈ 0.35–0.7 TB at two years
  within the Supabase plan (TBD with the operator).
- Cluster rebuild from the stack + Supabase backup, RTO ≤4 h; rehearsal before launch.

## Non-resources

- No cache service, no queue, no external order integration.
