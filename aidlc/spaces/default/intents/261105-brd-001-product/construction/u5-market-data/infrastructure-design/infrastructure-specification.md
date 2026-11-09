# Infrastructure Specification — u5-market-data

Inputs: `nfr-design/*`, `inception/domain-design/components.md`, `functional-design/functional-spec.md`, and C4/C6/C11.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute model | Two Swarm services: `market-data` (2 replicas, 0.25 vCPU / 256 MiB) and `market-data-worker` (1 replica, 0.5 vCPU / 512 MiB) | The API is stateless. C11 runs execute inline in the API replica that receives the call, fenced by the per-job lease; the worker owns the lease sweep and 90-day retention |
| Networking topology | No published port: `market-data` is reached only from Portfolio, Reporting and Job Orchestration over `internal` (the SPA reaches Portfolio through the gateway). Both services join `public` for provider and Supabase Postgres egress only. C4, C10 and C11 are reached over `internal` | Market Data is not a user-facing service; C6 callers are internal |
| Storage | Supabase Postgres schema `market_data`, role `market_data_svc`; Vault for provider keys read through a `SECURITY DEFINER` function | One schema and role per service; no cross-schema grants |
| Environments | Production only | Team decision |
| IaC | `infra/swarm-stack.yml` with both services, networks, health checks and update config; the migrate and smoke one-shot tasks are created by the release job, not declared in the stack | Infrastructure is code; jobs run once per release |
| Sizing | API 12 pool/replica; worker 8; ceiling 4 API replicas | Launch bound of 2,000 assets and 50 rps |
| Ingress health | API `/healthz` 8093 and `/readyz`; worker `/healthz` 8094 and `/readyz`; Swarm health check `/healthz`, release gate on `/readyz` | Dependency-safe liveness |
| Update strategy | Rolling, `start-first` API, `stop-first` worker, `failure_action: rollback` | API replicas may overlap safely because the per-job lease fences runs; the worker stays single |
| Scaling | Manual `docker service scale market-data=N`; the worker stays at 1 unless the tail job misses its deadline | No Swarm autoscaler |

The C11 caller's timeout must exceed the run deadline (180 s top100 / 600 s tail), and
`market-data-api` sets `stop_grace_period: 660s` so an in-flight run is not killed mid-write; the
lease sweep still recovers an interrupted run.

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| Supabase Postgres `market_data` | Database | Observations, FX, runs, tiers, tokens, rate buckets; 90-day pruning of observations and FX except the newest per asset/source and pair; Supavisor transaction pooling; sqlx statement cache disabled; daily backup, PITR | RPO ≤ 24 h, RTO ≤ 4 h |
| Supabase Vault + `read_provider_secret` | Secrets | `SECURITY DEFINER` function in `market_data` granted to `market_data_svc` | No cross-schema grant |
| Docker secrets | Bootstrap | `market_data_database_url` (role `market_data_svc`, used by the API, worker, and the migrate job) and `market_data_smoke_token` (the release job's smoke credential) | Mounted as `*_FILE`. The C11 shared run token is read from Vault at startup through the same `SECURITY DEFINER` function. No Supabase service-role key: this service never calls Supabase Auth |
| Service-token registry | Runtime auth data | `market_data.service_token_registry` rows are seeded by the deploy job from operator-supplied hashes (Portfolio, Reporting, Job Orchestration, plus a read-only `smoke-reader` caller); rotation inserts the new hash and deletes the old row | No plaintext token is stored; distribution is out-of-band. The smoke token is a Docker secret held only by the release job |
| Smoke task | Release gate | A one-shot task on the `internal` overlay runs `getPrices` and `getFxRate` with the `smoke-reader` token, before the release is promoted | Proves the C6 path and the token registry without publishing a port |
| `internal` overlay | Networking | Encrypted; gateway, `market-data`, `market-data-worker`, C4, C10, C11 | Internal calls |
| `public` overlay | Networking | Price/FX providers and Supabase Postgres | TLS only |
| CDN/WAF + gateway | Edge | TLS, rate limits, JWT | Serves Portfolio, not Market Data directly |
| Loki/Prometheus/Tempo/Grafana | Observability | Metrics, logs, traces, dashboards | Existing stack |

## Shared Infrastructure

| Shared Resource | Owner Unit | Consumer Units | Access Boundary |
|---|---|---|---|
| API Gateway | u1-walking-skeleton | u2-web-app, all services | Gateway routes and rate-limits |
| Supabase Postgres | externally managed (Supabase) | every service (own schema) | No cross-schema grants |
| Asset Registry (C4) | u1-walking-skeleton | u5-market-data | Token-authenticated internal HTTP |
| Audit (C10) | u12-audit | u5-market-data | Write-only events |
| Job Orchestration (C11) | u11-job-orchestration | u5-market-data | Runs the two jobs |
| Observability stack | u1-walking-skeleton | every service | OTLP/metrics ingestion |

## Cost and recovery

- Incremental cost: 2 × 0.25 vCPU API and 1 × 0.5 vCPU worker on existing Swarm capacity; no new
  managed service. Priced figure TBD with the platform owner before launch.
- Cluster recovery: rebuild from `infra/swarm-stack.yml` and the Supabase backup, RTO ≤ 4 h;
  restore rehearsal before launch.

## Explicit non-resources

- No Redis or cache: the source list is read from C4 per request.
- No message queue: the two jobs are claimed from `refresh_runs`.
- No time-series database: Postgres with 90-day pruning.
