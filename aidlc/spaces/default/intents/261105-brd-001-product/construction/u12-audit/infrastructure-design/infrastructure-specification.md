# Audit — Infrastructure Specification

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute model | Two Docker Swarm services from two images: `cpb/audit-api` (3 replicas) and `cpb/audit-retention` (1 replica, scheduled-driven) | The retention credential must not live in the process that serves appends (NFR19.2) |
| Networking | Both services join the existing `internal` overlay and nothing else. `audit-api` publishes no port; only the gateway reaches it. `audit-retention` is reachable by nothing — it is a scheduled worker with no inbound listener, calling `audit-api` outbound when it needs to write an event. Service-to-service calls use TLS with the project's internal CA (rustls), not the overlay's IPsec: NFR9.3 says TLS, and overlay encryption is not TLS | The audit API is an internal boundary (NFR9.1, NFR9.3) |
| Credential delivery | The pipeline reads each role's credential from its Supabase Vault path and creates a Docker secret (`audit_app_dsn`, `audit_retention_dsn`); the Swarm task mounts the secret as a file at `/run/secrets/…`, exactly as the walking skeleton mounts `registry_database_url`. A stack file cannot reference a Vault path itself, so Vault is the source of truth and Docker secrets are the delivery | The existing stack file's pattern; no credential in an env var or an image |
| Retention scheduling | `audit-retention` runs as a Swarm service in `replicated` mode with one replica and a supervisor loop that sleeps to the next slot rather than a Swarm cron (Swarm has none); Job Orchestration triggers it through `audit-api`'s run endpoint, and the supervisor is the fallback if that trigger is missed | The heartbeat alert covers a missed trigger |
| Storage | No local volume. Exports stream into the HTTP response; nothing is written to disk | Removes replica-local state and the download-routing problem |
| Environments | Production only. No staging tier — the team's affirmed deployment rule | Team memory `## Deployment`; a staging copy of the audit trail would be a second copy of security data with weaker controls |
| IaC approach | One `infra/swarm-stack.yml` beside the other services, digest-pinned images, health checks, `failure_action: rollback`; SQL migrations as versioned sqlx up/down pairs | Project rule: every service deploys as a replicated Swarm service with a rollback path |
| Resource sizing | `audit-api`: 0.5 vCPU / 512 MB per task, limits equal to reservations. `audit-retention`: 0.25 vCPU / 256 MB | Sized to the append path's measured budget, not to a guess |
| Restart policy | `condition: any`, `max_attempts: 0`, health check `GET /healthz` every 10 s (shallow, same path as the gateway and asset-registry); `GET /readyz` checks the database and the migrations table and is what the deploy smoke test calls before traffic is considered good | NFR1.1, NFR1.2; matches the existing services' paths |
| Migration execution | A dedicated `migrate` step runs the sqlx up/down against the schema using a **DDL-holding migration credential** (`audit_migrator`), created for the purpose and used by nothing else. `audit_app` has no DDL rights and cannot be the migration credential | The reviewer's point: `docker stack deploy` runs no SQL, and `audit_app` cannot create partitions |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| Supabase Postgres — schema `audit` | database | One schema, three roles (`audit_app`, `audit_readonly`, `audit_retention`), declarative partitioning by `recordedAt`, partitions created three months ahead by the deploy migration | The instance shares with every other schema; the connection budget is the isolation |
| Role credentials | secrets | Three Supabase Vault paths plus the migrator path. At deploy the pipeline reads each path and creates a Docker secret; the task mounts it as a file. Rotation writes the new value to Vault, then the pipeline recreates the secret and forces a service update | NFR19.4; no credential in an env file or an image |
| Bootstrap | secrets | The first deploy creates the `audit_migrator` role and the three runtime roles through a Vault-run SQL bootstrap, then stores each credential in its Vault path. No credential is ever printed, and the bootstrap runs once | Nothing to rotate on day one that is not already in Vault |
| Partition horizon job | job | Runs at the start of each deploy migration (using the migrator credential) and daily inside `audit-retention` (as its fallback) | Keeps appends independent of the retention service |
| Prometheus `/metrics` | metrics | Scraped by the shared Prometheus | Low-cardinality labels only |
| Loki | logs | Structured JSON on stdout | Shared stack |
| Tempo | tracing | OTLP from `tracing` spans | Shared stack |
| Grafana | dashboards | One board, four rows | Shared stack |

## Shared Infrastructure

| Shared Resource | Owner Unit | Consumer Units | Access Boundary |
|---|---|---|---|
| Supabase Postgres instance | Platform (U1) | Every service, each in its own schema | No cross-schema grants; HTTP only between services |
| Supabase Vault | Platform (U1) | Every service | Each service reads only its own paths |
| Swarm ingress and TLS termination | U1 (gateway) | All services | No service publishes a port except the gateway |
| Loki / Prometheus / Tempo / Grafana | Platform (U1) | All services | Write-only from services; read by operators |
| GitHub Actions runners | Platform (U1) | All services | Deploy credentials scoped to the stack, not the platform |

## Migrations

Each release ships `migrations/000N_*.up.sql` / `.down.sql`. The up migration creates the next
three months of partitions and the indexes named in the performance design; the down migration
drops the indexes and the partitions it created **in that release only** — never a partition that
holds rows inside the retention window, which is why the down path checks the window before it
drops anything.

## Sources

- `construction/u12-audit/nfr-design/logical-components.md` — component inventory and budgets
- `construction/u12-audit/nfr-design/security-design.md` — role matrix and credential separation
- `construction/u12-audit/nfr-design/scalability-design.md` — partitioning and capacity
- `construction/u12-audit/nfr-design/reliability-design.md` — health, readiness, rollback
- `construction/u12-audit/functional-design/functional-spec.md` — WF1 to WF7
- `inception/domain-design/components.md` — the Audit component
- `inception/contract-design/contract-summary.md` — C10, C11
