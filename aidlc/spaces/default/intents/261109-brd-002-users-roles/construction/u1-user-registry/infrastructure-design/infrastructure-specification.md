# Infrastructure Specification — u1-user-registry

**Upstream:** `performance-design.md`, `security-design.md`, `scalability-design.md`, `reliability-design.md`, `observability-design.md`, `logical-components.md`, `components.md`, `functional-spec.md`, `contract-summary.md`.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute model | Containers on the existing Docker Swarm; the registry is a module of the Identity service | No new deployable |
| Networking — ingress | CDN/WAF → edge → API Gateway; C3 routes are internal, undeclared at the edge | Deny-by-default |
| Storage | Supabase Postgres, Identity schema (`users`, `shredding_keys` already exist) | One schema and role per service |
| Environments | Production Swarm only; `deployment/localhost` for development | Production is the only environment |
| IaC approach | `infra/swarm-stack.yml` (no change for this unit) | Existing conventions |
| Resource sizing | Identity API replicas 3 at 0.5 CPU / 512 MB (as configured) | Current stack |
| Migrations | `0004_user_registry_extensions.{up,down}.sql` — add `disabled` and `pending_deletion` to the status check, add display-name/email override markers | Pair rule; 0003 is taken by u6 |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| Supabase Postgres (identity schema) | database | Indexes on subject hash, id, email | `identity_app` role |
| pgsodium | encryption | Deterministic subject ciphertext; per-user shredding keys | Key material in Vault |
| Docker secrets | secrets | None new for this unit | — |
| Audit service | internal HTTP | 300 ms + one retry | Evidence |
| Jurisdiction gate (u6) | internal HTTP | Gate's own 1.5 s cap | Provisioning precondition |
| Prometheus/Loki/Tempo/Grafana | observability | Existing stack | — |

## Shared Infrastructure

| Shared Resource | Owner Unit | Consumer Units | Access Boundary |
|---|---|---|---|
| `users` table | u1 | u2 (role read), u3 (target/requester), u5 (status transitions) | In-schema via the repository; cross-unit through C3 |
| Identity schema role `identity_app` | Identity service | all identity units | Schema-local |

## Rationale

The registry extends the existing identity tables rather than introducing new infrastructure: one migration pair, no new service, no new secret.
