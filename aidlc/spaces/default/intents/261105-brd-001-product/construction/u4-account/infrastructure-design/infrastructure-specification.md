# Infrastructure Specification — u4-account

Inputs: `nfr-design/performance-design.md`, `security-design.md`, `scalability-design.md`,
`reliability-design.md`, `observability-design.md`, `logical-components.md`,
`inception/domain-design/components.md`, `functional-design/functional-spec.md`, and
`inception/contract-design/contract-summary.md`.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute model | Two replicated Docker Swarm services: `account` (3 replicas, 0.25 vCPU / 256 MiB each) and `account-worker` (1 replica, 0.5 vCPU / 512 MiB) | The API is stateless and scales with reads; the worker owns sync, import processing, and the run lease and must be a single writer per account |
| Import handoff | The API creates the `ImportBatch` row (`status = pending`, updated to `processing`/`complete`/`failed` by the worker) and writes the uploaded bytes to `account.import_upload` (`batch_id` PK/FK, `bytes bytea`, `byte_size`, `created_at`, capped at 10 MiB) in 64 KiB chunks; `ImportBatch.id` is returned in the 202. The worker polls `status = 'pending'`, reads the file in one bounded 10 MiB buffer, and streams 500-row batches. The staging row is deleted when the batch completes or after 90 days | No message queue and no object store are allowed or needed; the service's schema is the only buffer, and the file survives a worker restart. The 10 MiB cap is the memory bound, and an oversized upload is rejected before staging |
| Networking topology | Edge CDN/WAF → API Gateway → `internal` overlay → `account`. Both `account` and `account-worker` join `internal` (for C3, C4, C9, C10) and `public` (for Binance, OKX, EVM/Solana RPC, Supabase JWKS, Supabase Postgres through pgbouncer, and the Supabase Auth admin API) | The SPA never calls Account directly; the worker takes no inbound traffic but must reach both the internal shared services and the external providers; the API needs the gateway inbound and provider egress for credential verification |
| Storage strategy | One Supabase Postgres schema `account` and one role `account_svc`, holding the domain tables, the `ImportBatch` rows, and the `import_upload` staging rows; Vault references for per-user exchange credential secrets; no shared store with another service | One schema and one role per service, no cross-schema grants |
| Environments | Production only (`production` Swarm stack); no staging tier | Team decision: production is the only environment |
| IaC approach | `infra/swarm-stack.yml` declares both services, replicas, resource limits, networks, health checks, and update config; sqlx migrations ship with the service image | Infrastructure is code; rolling updates are health-gated |
| Resource sizing | API 0.25 vCPU / 256 MiB × 3; worker 0.5 vCPU / 512 MiB × 1; Postgres pool 12 per API replica and 8 in the worker | Launch bound of 10,000 users and ≤ 50,000 accounts; C5 reads at 50 rps |
| Ingress health | API: `GET /healthz` (liveness) and `GET /readyz` (Postgres + Vault) on port 8091. Worker: the same two endpoints on port 8092, with `/readyz` also failing while draining. The Swarm health check uses `/healthz` (so a dependency blip cannot restart-storm the tasks); the release job gates promotion on a `/readyz` smoke step | A new image that cannot reach its dependencies must not be promoted, and a dependency outage must not cause restart churn |
| Update strategy | Swarm rolling update, defaults, `start-first` for the API and `stop-first` for the worker with `failure_action: rollback`; each step waits for the task to be healthy, then the release job's `/readyz` smoke gates promotion | Worker overlap would break the single-writer lease; API overlap is safe |
| Scaling mechanism | Manual on-call action (`docker service scale account=N`), triggered by the C5 p95 alert or pool in-use above 80% of 12 for 10 minutes; ceiling 4 API replicas. Docker Swarm has no native autoscaler | The trigger is carried from the NFR design; the ceiling bounds cost |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| Supabase Postgres (schema `account`) | Database | Shared Supabase instance; schema and role `account_svc`; connections through pgbouncer; daily backup, PITR per plan. Holds `ImportBatch` and its `import_upload` staging row | RPO ≤ 24 h, RTO ≤ 4 h; migrations applied by the release job |
| Supabase Vault | Secrets | Per-user exchange credential references; one token per calling service (`identity_service_token`, `portfolio_service_token`, `ledger_service_token`, `job_orchestration_service_token`). The caller identity is resolved from `account.service_token_registry` (token hash → caller name → per-operation allow-list) | Used at runtime, after the service connects with its Docker-secret DSN; no bootstrap credential is stored here |
| Docker secrets | Bootstrap configuration | `account_database_url`, `account_supabase_service_role`, and `account_migrate_database_url` created at `docker stack deploy` from the operator's secret store, mounted as `*_FILE` into the services and the migration job; the DSN requires TLS (`sslmode=require`) | Reconciles with the existing stack's Docker-secret convention (`*_supabase_service_role` naming); only per-user exchange keys live in Vault |
| Docker Swarm overlay `internal` | Networking | `internal: true`; gateway, `account`, and `account-worker` attach | No external connectivity; carries C3, C4, C9, and C10 calls |
| Docker Swarm overlay `public` | Networking | External-connectivity overlay; `account` and `account-worker` attach; provider and Supabase egress | The gateway and the `web` service publish ports; Account publishes none |
| CDN/WAF + API Gateway | Load balancer / edge | TLS termination, rate limits per user/IP, JWT forwarding with the `aal` claim unchanged | Account validates the JWT against Supabase JWKS and requires `aal2` to store an exchange credential |
| Self-hosted Loki / Prometheus / Tempo / Grafana | Observability | Metrics scrape, log aggregation, OTLP traces, dashboards and alerts | Every service emits logs, metrics, and traces |
| Supabase Auth admin API (C15) | External identity | Outbound HTTPS; service-role key from a Docker secret | Used for `app_metadata.has_exchange_credentials`, retried and audited |

## Shared Infrastructure

| Shared Resource | Owner Unit | Consumer Units | Access Boundary |
|---|---|---|---|
| API Gateway | u1-walking-skeleton | u2-web-app, u4-account, all services | Gateway owns routing and rate limits; Account exposes C1 routes only |
| Supabase Postgres instance | externally managed (Supabase) | every service (own schema only) | No cross-schema grants; a service reads another only through its HTTP API |
| Self-hosted observability stack | u1-walking-skeleton | every service | OTLP/metrics ingestion; dashboards read-only for the team |
| Audit service (C10) | u12-audit | u4-account | Account writes events; it never reads the audit schema |
| Identity service (C3) | u3-identity | u4-account | Grant check over internal HTTP only |
| Asset Registry (C4) | u1-walking-skeleton | u4-account | Supported-chains and token resolution over internal HTTP only |
| Notification service (C9) | u9-notification | u4-account | Sync and credential alerts over internal HTTP only |

## Security posture

- TLS terminates at the edge/CDN and gateway; the `internal` overlay is encrypted
  (`encrypted: true`), the `public` overlay is not. Postgres uses a TLS DSN (`sslmode=require`) and
  all provider and Supabase egress is HTTPS.
- Data minimisation: the schema stores the credential's Vault reference, never the key; audit and
  logs carry ids and labels only, never a token or full address.
- Retention: anonymised rows keep their retention label in the `account` schema; no dedicated
  infrastructure resource is needed beyond the schema and the scheduled sweep.

## Cost and recovery

- Launch-bound cost estimate: 3 × 0.25 vCPU API tasks + 1 × 0.5 vCPU worker on existing Swarm
  capacity (no new node), the existing Supabase plan, and the existing observability stack; the
  only incremental cost is compute share on the cluster. A priced figure is TBD with the platform
  owner before launch.
- Cluster-level recovery: a lost node re-schedules replicas on the surviving nodes; a lost cluster
  is rebuilt from `infra/swarm-stack.yml` and the Supabase backup, with the same RTO ≤ 4 h. A
  restore rehearsal before launch covers both the schema and the stack.

## Explicit non-resources

- No message queue: sync runs are claimed from the `SyncRun` table on the run lease; CSV uploads
  are staged in `account.import_upload` and polled by the worker.
- No Redis or other cache service: chain metadata is cached in process for 5 minutes.
- No object storage: staged CSV bytes live in the service's own Postgres schema.
- No separate read replica at launch; add one only if the C5 read SLO is missed for two weeks.
