# Infrastructure Specification — u6-jurisdiction-gate

**Upstream:** `performance-design.md`, `security-design.md`, `scalability-design.md`, `reliability-design.md`, `observability-design.md`, `logical-components.md`, `components.md`, `functional-spec.md`, `contract-summary.md`.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute model | Containers on the existing Docker Swarm; the gate is a module of the Identity service | No new deployable; the team's stack is fixed |
| Networking — ingress | CDN/WAF → edge → API Gateway (deny-by-default routes) | Only route declarations change; `/internal/*` stays unreachable from the edge |
| Networking — egress | Outbound HTTPS from the Identity service to the geolocation provider | Provider called over TLS with a bounded client |
| Storage | Supabase Postgres, Identity schema (`jurisdiction_list_versions`, `jurisdiction_rules`) | One schema and role per service |
| Environments | Production Swarm only; `deployment/localhost` for development | Production is the only environment |
| IaC approach | `infra/swarm-stack.yml` (service definition) + `infra/sql/identity-schema.sql` | Existing repository conventions |
| Resource sizing | Identity API replicas: start at 2, scale with check throughput; no dedicated volume | Stateless; list reloaded from Postgres |
| Migrations | One versioned sqlx up/down pair adding the two tables and indexes | Required pairing rule |

### Migration outline

| Migation | Purpose |
|---|---|
| `0004_jurisdiction_list.up.sql` / `.down.sql` | Create `jurisdiction_list_versions` and `jurisdiction_rules`, the version foreign key, and the `(version, country_code)` primary key |

The exact next migration number is reserved per the parallel-unit rule; U4 and U5 also add pairs, so the number is allocated before any of the three starts.

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| Supabase Postgres (identity schema) | database | 300 rules per version, versioned snapshots | Read by the API role; written by the publish role |
| In-memory list | cache (in process) | ≤ 30 KB per replica; refresh every 10 s | No external cache service is introduced |
| Geolocation provider | third-party API | HTTPS, 800 ms attempt timeout, one retry, circuit breaker | Contractual SLA recorded on the provider-dependency SLO |
| Audit service | internal HTTP | 300 ms timeout, one retry, idempotency key | Append-only store |
| Docker secrets | secrets | Provider API key and publish credential as `*_FILE` Docker secrets, materialised from Supabase Vault | Never in env values, source, or logs |
| Prometheus/Loki/Tempo/Grafana | observability | Existing self-hosted stack | Scrapes `/metrics`; ships logs and traces |

## Shared Infrastructure

| Shared Resource | Owner Unit | Consumer Units | Access Boundary |
|---|---|---|---|
| Identity schema | Identity service (shared by U1–U5) | None for these tables | The gate's tables are namespaced `jurisdiction_*`; other units never read them |
| Audit service | Audit service | U6, all services | HTTP API only |
| API Gateway route table | Gateway service | U6 declares its routes | Deny-by-default; `/internal/*` not exposed |

## Rationale

No AWS services are used; the platform is the team's Docker Swarm with self-hosted Supabase. The only new infrastructure is two tables, one migration pair, and a Docker secret for the provider key — deliberately minimal, because the gate is a module inside a service that already exists.
