# Infrastructure Specification — u9-notification

Inputs: `nfr-design/*`, `functional-design/`, `inception/domain-design/components.md`, C9/C3/C10/C11.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute | `notification` (2 replicas, 0.2 vCPU / 192 MiB) and `notification-worker` (1 replica, 0.25 vCPU / 256 MiB) | API stateless; one leased worker for delivery/retention |
| Networking | No published port; gateway → `internal`; C3/C10/C11 internal; `public` for the email provider and Postgres | Provider is external |
| Storage | Supabase Postgres schema `notification`, role `notification_svc` | One schema and role |
| Environments | Production only | Team decision |
| IaC | `infra/swarm-stack.yml`: both services, networks, health checks, update config; migrate is a release-job one-shot | Infrastructure is code |
| Sizing | API pool 10; worker 8; ceiling 4 | 500k alerts/month |
| Health | API `/healthz` 8099, worker 8100; `/readyz` Postgres only | Provider/C3 outages must not fail readiness |
| Update | Rolling, `start-first` API, `stop-first` worker, `failure_action: rollback` | One delivery executor |
| Scaling | Manual `docker service scale notification=N` on p95 > 100 ms for 10 min | No autoscaler |
| Wake | Durable `wake_requested_at` flag on the singleton delivery row; worker polls every 5 s | No queue; mandatory ≤30 s |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| Supabase Postgres `notification` | Database | preferences, alerts, delivery_attempts, delivery_control (wake flag/lease); alerts/attempts pruned after 12 months by the worker; keyset index `(user_id, created_at desc, id)`; daily backup, PITR | RPO ≤24 h, RTO ≤4 h |
| `notification_migrator` | Migration role | CREATE schema/tables/roles/grants, used only by the migrate job via `notification_migrate_database_url` | Not available to the running services |
| Email provider | External | HTTPS API, tier 240 calls/min, 5 s call deadline, concurrency 20 | TLS only |
| Docker secrets | Bootstrap | `notification_database_url`, `notification_migrate_database_url`, `notification_smoke_token`, `notification_outbound_token` (C3/C10), `notification_email_key` | `*_FILE` |
| Service-token registry | Auth | gateway, Identity, Account, Portfolio, Reporting, smoke-reader; rotation insert-then-delete | No plaintext stored |
| `internal` overlay | Network | Encrypted; gateway, Identity, Account, Portfolio, Reporting, Audit, Job Orchestration | — |
| `public` overlay | Network | Email provider and Postgres | TLS |
| Loki/Prometheus/Tempo/Grafana | Observability | Existing stack | — |

## Shared Infrastructure

| Resource | Owner | Consumers | Boundary |
|---|---|---|---|
| API Gateway | u1 | u2, u9 | C1 feed/preferences |
| Identity (C3) | u3 | u9 | verified contact + erasure caller |
| Audit (C10) | u12 | u9 | write-only |
| Job Orchestration (C11) | u11 | u9 | fallback delivery run |
| Observability | u1 | all | ingestion |

## Cost and recovery

- Incremental: 2 × 0.2 + 1 × 0.25 vCPU on existing capacity; the email provider is priced per 1,000
  emails with the launch tier (TBD with the operator).
- Cluster rebuild from the stack + Supabase backup, RTO ≤4 h; rehearsal before launch.

## Non-resources

- No queue (the alerts table is the queue), no cache, no SMS/push channel at launch.
