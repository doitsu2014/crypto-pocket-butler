# Infrastructure Specification — u2-authorisation

**Upstream:** `performance-design.md`, `security-design.md`, `scalability-design.md`, `reliability-design.md`, `observability-design.md`, `logical-components.md`, `components.md`, `functional-spec.md`, `contract-summary.md` (C3, C4, C9).

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute model | **No new container.** The decision is a module of the existing Identity service; the C4 route is an Axum handler on the same binary | The unit owns no deployable and no schema |
| Networking — ingress | Edge → API Gateway → Identity `/v1`. The C4 `/internal/authorize` route is **not** in the gateway's route table; it is reached directly over the internal network by allowlisted callers | Deny-by-default; the gateway's table covers external `/v1` routes only |
| Networking — internal | Cluster-internal HTTP to the C4 route, the audit service (C9), and Supabase Auth | Existing service mesh; TLS in production |
| Storage | Identity's existing Postgres schema (`identity_app` role) for the status read only; no new table, no new role | Least privilege; the unit stores nothing |
| Environments | Production Swarm only; `deployment/localhost` compose for development | Production is the only environment (project rule) |
| IaC approach | `infra/swarm-stack.yml`: **one change** — the C4 caller wiring below. No new service, no new container | See "What must be provisioned" |
| Resource sizing | Identity API 3 replicas × 0.5 CPU / 512 MB, as configured | Existing stack; the decision adds no measurable load to a replica |
| Migration | None | No schema change |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| Supabase Postgres (identity schema) | database | Existing; one indexed status read per request | `identity_app` role, own schema only |
| Audit service (C9) | internal API | **250 ms ceiling per write** — this is the governing value; `reliability-design.md` is aligned to it | Every denial; two entries per role change |
| Supabase Auth | third-party API | Admin API for the role write; JWKS for validation | The only store of `app_metadata.role` |
| Docker secrets / Supabase Vault | secrets | The per-service C4 tokens (see below); the Identity service's Supabase admin credential | Never an env var, never in source |
| Prometheus + Alertmanager | metrics and alerts | **Not declared in the repository.** The scrape job is specified in `monitoring-design.md` and lands with the platform work | `authorisation_*` metrics are emitted but not yet collected |
| Loki / Tempo / Grafana | logs, traces, dashboards | Same platform gap as Prometheus | One new dashboard definition, armed when the stack lands |

## What must be provisioned (the one stack change)

| Item | Where | Change kind |
|---|---|---|
| `identity_service_tokens` entry for `account-api` | `infra/swarm-stack.yml`, the identity service's secrets block | Stack change + secret material |
| `identity_service_tokens` entry for `portfolio` | same | Stack change + secret material |
| Each caller's own token entry (the matching `*_service_tokens` entry the caller reads) | the caller service's secrets block | Stack change + secret material |
| `IDENTITY_URL` for `account-api` | the account service's environment | **Missing today** — must be added |
| `IDENTITY_URL` for `portfolio` | the portfolio service's environment | **Wrong today: `http://identity-api:8089`.** Identity listens on **8083**; a C4 call would hit a closed port. Corrected in this change |

Service tokens are held inside each service's `*_service_tokens` secret, not in a standalone
secret — the provisioning above follows that existing pattern rather than inventing a new one.

## Shared Infrastructure

| Shared Resource | Owner Unit | Consumer Units | Access Boundary |
|---|---|---|---|
| Identity service process | Identity (all units) | u1, u2, u3, u4, u5, u6 | In-process modules; no network call inside the service |
| C4 `/internal/authorize` | u2 | u3, Account service, Portfolio service | Service token + per-operation allowlist + the user's JWT |
| Audit service (C9) | Audit service | every service | Per-service token, write-only |
| Supabase Auth admin API | platform | Identity service | One credential, held by Identity |
| `identity_app` role and schema | u1 owns the tables | u2 reads status | Same schema, same service; no cross-schema grant |

## Why no new container

The unit is business logic plus adapters inside an existing service. A new container, cache tier,
or queue would add an operational surface with no load to justify it: the status read rides the
service's existing pool, the outage cache is in-process memory, and the two external writes go to
services that already exist. What is *not* free is the caller wiring above — two service-token
entries, one missing base URL, and one wrong port — and this design records it as the change it
is rather than folding it into "no change".
