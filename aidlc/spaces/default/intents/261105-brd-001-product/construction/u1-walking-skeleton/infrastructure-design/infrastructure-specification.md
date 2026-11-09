# Infrastructure Specification — u1-walking-skeleton

Designs the infrastructure behind `nfr-design/performance-design.md`,
`security-design.md`, `scalability-design.md`, `reliability-design.md`,
`observability-design.md`, and `logical-components.md`, for the ApiGateway and AssetRegistry
components in `inception/domain-design/components.md`, serving the workflows in
`functional-design/functional-spec.md` under the contracts in
`inception/contract-design/contract-summary.md`.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute model | Docker Swarm services, one image per component | Fixed by the project rules; rolling updates are native |
| Swarm topology | **Single node at launch** (one manager, also a worker) | The interview chose single-node for now; the three-manager target remains the growth path |
| Networking | Swarm routing mesh on 443 via the edge; internal service network for service-to-service | Only the gateway is publicly reachable |
| Storage | Supabase Postgres, schema `registry`; no local volume state | Per-service schema and role; stateless replicas |
| Environments | Production only | No staging tier, by the practices decision |
| IaC | Swarm stack files committed to the repository, applied by the pipeline | Infrastructure is code; no console changes |
| Resource sizing | Gateway and registry: 512 MiB / 0.5 vCPU each, 2 replicas minimum, 4 at launch capacity | Matches the NFR design |
| Edge | CDN/WAF in front of the gateway; TLS terminated at the edge | TLS everywhere, HSTS |

**Risk recorded:** a single-node Swarm has no node-failure tolerance; the 99.9% availability
target depends on the host staying up. Moving to three managers is the first infrastructure
change when the target is enforced.

## Infrastructure services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| Supabase Postgres | database | Schema `registry`, one role, PITR where configured | No cross-schema grants |
| Supabase Vault | secrets | Holds the registry role credential and JWKS configuration | Rotatable |
| Supabase Auth / JWKS | identity | External; JWKS cached in the gateway for 10 minutes | Fails closed after the window |
| Docker Swarm routing mesh | load-balancer | 2–4 replicas per service, health-gated rolling update | Single node at launch |
| Private/public image registry | artifact store | Public registry with digest-pinned references and a scan gate | The interview chose public; digest pinning and scanning still apply |
| Observability stack | metrics/logs/traces | Loki, Prometheus, Tempo, Grafana on the same Swarm | One stack, self-hosted |

## Shared infrastructure

| Shared Resource | Owner Unit | Consumer Units | Access Boundary |
|---|---|---|---|
| Supabase Postgres instance | platform | U1 and every later service unit | One schema and role per service; no cross-schema grants |
| Supabase Vault | platform | U1, U3, U4 and later | Per-service secret paths; never in env files |
| Swarm cluster and routing mesh | platform | every unit | Only the gateway is public |
| Observability stack | platform | every unit | Services push metrics, logs, and traces |
| Container registry | platform | every unit | Digest-pinned pulls; scan gate |
