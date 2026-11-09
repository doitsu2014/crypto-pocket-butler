# Infrastructure Specification — u3-identity

Inputs: `nfr-design/performance-design.md`, `security-design.md`, `scalability-design.md`,
`reliability-design.md`, `observability-design.md`, `logical-components.md`,
`inception/domain-design/components.md`, `functional-design/functional-spec.md`, and
`inception/contract-design/contract-summary.md`.

The team deploys to Docker Swarm with Supabase managed Postgres; there is no AWS account and no
CDK/Terraform layer. The IaC is the Swarm stack file plus the sqlx migrations.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute model | Replicated Docker Swarm services: `identity-api` (3 replicas) and `identity-worker` (1 replica, internal wake endpoint) | Matches the project's fixed deployment rule; stateless API and lease-guarded worker |
| Networking topology | Swarm ingress overlay; all external traffic through the edge CDN/WAF and the API Gateway; the internal overlay is declared with `encrypted: true` in this unit's stack-file change (stacks named below); the worker exposes `POST /internal/wake` on the internal network only | NFR9.1 requires encrypted internal traffic; the current stack file does not set it, so this unit changes it |
| Storage strategy | Supabase Postgres `identity` schema; Supabase Storage private `identity-exports` bucket; Supabase Vault for the shredding key ids | Fixed by the project rules; no local volume |
| Environments | Production only; no staging tier | Team Deployment rule: production is the only environment |
| IaC approach | `infra/swarm-stack.yml` stack file and versioned sqlx up/down migrations | Fixed by the project rules; no CDK/Terraform |
| Resource sizing | API 0.5 vCPU / 512 MiB per replica; worker 0.5 vCPU / 512 MiB; 3 API replicas, 1 worker; API pool 12 per replica (36), worker pool 8, 16 of the 60-connection budget left as headroom | Fits the launch load projection (50 rps principal resolution) and leaves room for the worker and a fourth replica |
| Scale-out procedure | Manual `docker service scale identity-api=N`, ceiling N=4 (48 + 8 = 56 connections) | Docker Swarm has no replica autoscaler; the trigger is an operator action |
| Backup and recovery | Supabase PITR 7-day window plus daily backups retained 30 days; restore rehearsal before launch; the deletion record stores the destroyed key id, and the restore procedure re-destroys keys for every completed deletion before the schema is served | NFR18.1 (RPO ≤24 h, RTO ≤4 h) and the deletion guarantee after a restore |
| Health and rollback | Health check on `/readyz`; `failure_action: rollback` on the service update | Swarm rolling update with health-check gating, as the team requires |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| Supabase Postgres | database | `identity` schema, one service role, direct endpoint (or pooler) with a 60-connection limit; API pool 12/replica, worker 8; sqlx statement cache off behind a transaction pooler | No cross-schema grants; no RLS |
| Supabase Auth | external identity provider | AAL2 MFA, JWKS, sessions, pre-sign-up hook, admin user delete | Identity validates the JWT itself against JWKS |
| Supabase Vault | secrets | Per-subject `ShreddingKey` ids; `identity.protect`/`unprotect` SECURITY DEFINER functions use pgsodium; gate-token signing key | The service passes the key id only; key material never leaves Postgres/Vault |
| Supabase Storage | object storage | Private `identity-exports` bucket, encrypted at rest, 24 h signed download; the worker deletes objects after `downloadExpiresAt` (sweep every 60 s) | Object deletion failure alerts |
| identity-worker wake endpoint | internal HTTP | `POST /internal/wake` on the internal overlay, called by the API after it commits an export or deletion request | Degrades to the 60 s sweep if the wake fails |
| Audit service (C10) | audit sink | Synchronous append with a 100 ms p95 / 200 ms timeout on support reads | Insert-only role |
| API Gateway | ingress | Deny-by-default routing, rate limits, correlation IDs | Identity is reached only through it, except the sign-up gate call |
| Job Orchestration (C11) | scheduler | Calls `POST /v1/internal/run/audit-privacy-sweep` | Resumes export and deletion work |
| Observability stack | metrics, logs, traces | Prometheus, Loki, Tempo, Grafana (self-hosted) | OTLP from the service |
| No cache, no queue | — | Caching is in-process (JWKS only); work is status-driven plus the wake endpoint | No Redis, no broker |

## Shared Infrastructure

| Shared Resource | Owner Unit | Consumer Units | Access Boundary |
|---|---|---|---|
| Supabase Postgres cluster | Platform (managed Supabase) | every service | One schema per service; no cross-schema grants |
| Supabase Vault | Platform (managed Supabase) | Identity | Project-scoped secrets only |
| Supabase Storage | Platform (managed Supabase) | Identity (`identity-exports`) | Private bucket; owner-only signed URL |
| Swarm stack, internal encrypted overlay, observability wiring | U1 (walking skeleton) | every service | Owned in `infra/swarm-stack.yml` and the observability stack |
| Audit service | U12 Audit | Identity and every service | C10 HTTP API; insert-only role |
| Job Orchestration | U11 | Identity | C11 run endpoint with the shared service token |
