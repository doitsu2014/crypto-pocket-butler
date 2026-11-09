# Infrastructure Specification — u10-analytics

Inputs: `nfr-design/*`, `inception/domain-design/components.md`, `functional-design/functional-spec.md`, C12/C3/C10/C11.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute | `analytics` (2 replicas, 0.2 vCPU / 192 MiB) and `analytics-worker` (1 replica, 0.25 vCPU / 256 MiB) | Stateless API; one worker for aggregation and retention |
| Network | No published port; reached from the gateway (ingest/flags/aggregates) and Identity (erase) over `internal`; `public` for Postgres only | Analytics is internal |
| Storage | Supabase Postgres schema `analytics`, role `analytics_svc`; Vault salt via `read_analytics_salt` | One schema and role |
| Environments | Production only | Team decision |
| IaC | `infra/swarm-stack.yml`: both services, networks, health checks, update config; the migrate job is a release-job one-shot | Infrastructure is code |
| Sizing | API pool 10; worker 6; ceiling 4 | 1 M events/month, peak 50/s |
| Health | API `/healthz` 8095, worker `/healthz` 8096; `/readyz` checks Postgres; Swarm gate on `/healthz`, release smoke on `/readyz` | Dependency-safe |
| Update | Rolling, `start-first` API, `stop-first` worker, `failure_action: rollback` | Worker must not overlap aggregation |
| Scaling | Manual `docker service scale analytics=N` on p95 > 120 ms for 10 min | No autoscaler |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| Supabase Postgres `analytics` | Database | events, consent_checks, metric_aggregates, feature_flags, service_token_registry; ≈7.5 M rows at the launch model; retention sweep deletes events > 6 months and consent_checks > 90 days; Supavisor transaction pooling; statement cache off; daily backup, PITR | RPO ≤24 h, RTO ≤4 h |
| Supabase Vault + `read_analytics_salt` | Secrets | `SECURITY DEFINER`, owned by the migration role with `SELECT` on `vault.decrypted_secrets`, EXECUTE to `analytics_svc`; every salt version retained 12 months | No cross-schema grant; a missing grant fails the migration loudly |
| Docker secrets | Bootstrap | `analytics_database_url`, `analytics_smoke_token`, `analytics_outbound_token` | Mounted `*_FILE`; the outbound token authenticates C3 getConsent and C10 audit, allow-listed to those two callers |
| Service-token registry | Auth | Hashes for the gateway, Identity (erase), and `smoke-reader`; the deploy job holds INSERT/UPDATE/DELETE on the table for rotation (insert-then-delete) | No plaintext stored |
| `internal` overlay | Network | Encrypted; gateway, Identity, C3, C10, C11, analytics | No external route |
| `public` overlay | Network | Postgres only | TLS |
| Loki/Prometheus/Tempo/Grafana | Observability | Existing stack | — |

## Shared Infrastructure

| Resource | Owner | Consumers | Boundary |
|---|---|---|---|
| API Gateway | u1 | u2, u10 | C12 ingest and C1 flag routes |
| Supabase Postgres | externally managed | every service | own schema only |
| Identity (C3) | u3 | u10 | consent + erase orchestration |
| Audit (C10) | u12 | u10 | write-only |
| Job Orchestration (C11) | u11 | u10 | aggregation/retention runs |
| Observability stack | u1 | all | ingestion |

## Cost and recovery

- Incremental: 2 × 0.2 vCPU + 1 × 0.25 vCPU on existing capacity; no new managed service.
- Cluster rebuild from `infra/swarm-stack.yml` + Supabase backup, RTO ≤4 h; rehearsal before launch.

## Non-resources

- No third-party analytics vendor, no OLAP store, no message queue, no cache service (a 30 s
  in-process flag cache only).
