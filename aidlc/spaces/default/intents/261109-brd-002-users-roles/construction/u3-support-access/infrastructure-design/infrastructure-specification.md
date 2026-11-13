# Infrastructure Specification — u3-support-access

**Upstream:** `nfr-design/*` (performance, security, scalability, reliability, observability,
logical-components), `functional-design/functional-spec.md`, `inception/contract-design/contract-summary.md`,
`inception/domain-design/components.md` (the SupportAccess component and its dependencies),
`infra/swarm-stack.yml`, `memory/team.md` (`## Deployment`).

**Headline: this unit provisions no new infrastructure service.** It adds one migration, one table,
two internal metadata routes, one gateway field, and one alert file, all on infrastructure the
platform already runs. It also carries two deployment prerequisites it does not own — one
corrected value (`ACCOUNT_URL`) and one missing secret mount on `portfolio` — recorded below
rather than assumed away.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute model | The existing Docker Swarm services. `identity-api` (3 replicas) serves the four routes, the metadata read **and hosts the existing audit-outbox drainer**: `AuditOutboxDrainer` is built and spawned in `build_api_state` (`services/identity/src/lib.rs:291,350`), so all three replicas drain concurrently under `FOR UPDATE SKIP LOCKED`. `identity-worker` (1 replica) runs `PrivacyWorker` only and drains nothing | The support outbox joins the drainer that already exists; no service moves and no second drain loop is added |
| Networking topology | Existing `internal` overlay network. `identity-api` reaches the peer services by their real service names — `account-api:8091` and `portfolio:8101`; no new network, no new ingress, no egress rule | The two hops are service-to-service over the network that already carries `IDENTITY_URL` and `AUDIT_URL`. Only `ACCOUNT_URL`'s shipped default is wrong (`http://account:8091`), and it is corrected at deploy |
| Storage strategy | Supabase Postgres `identity` schema: the existing `support_access_grants` plus the new `support_audit_outbox`, applied by `0007_support_access.{up,down}.sql` through the one-shot `identity-migrate` service | One schema per service is the platform rule; the outbox is a table, not a broker |
| Environments | Production only (the Swarm). `deployment/localhost` remains developer convenience and gains the same migration through its provision job | `memory/project.md` — no staging tier, no promotion step |
| IaC approach | `infra/swarm-stack.yml` for services and secrets; versioned sqlx migration pairs for schema; `infra/prometheus/*.yml` for alerts | These are the repository's three infrastructure surfaces; no new tool is introduced |
| Resource sizing | Unchanged: `identity-api` 0.5 CPU / 512 MB per replica, `identity-worker` 0.5 CPU / 512 MB; the new read adds two bounded outbound calls per request | The read is bounded at 500 ms and 20,000/day; a sizing change would be speculation |
| Release gate | The existing `identity-api` health check on `/healthz` with `start-first`, `parallelism: 1`, `failure_action: rollback`, `monitor: 30s` | `/healthz` must not depend on Account or Portfolio, or their outage would pull every replica out of an update (NFR1.3) |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| Supabase Postgres — `identity` schema | database | Existing instance; the migration adds a table, a partial unique index, two partial indexes and its grants | `identity_app` gets `select/insert/update/delete` on the new table — the drainer needs delete for the 24 h cleanup; `identity_migrator` owns DDL |
| `identity-migrate` | one-shot migration job | `restart_policy: none`, `identity_migrate_database_url` secret; unchanged shape | Runs `0007` once before the API update; the down pair is the rollback path |
| `identity-api` | compute | Already receives `ACCOUNT_URL`, `PORTFOLIO_URL`, `AUDIT_URL`, `SERVICE_TOKENS_FILE` and the JWT settings; hosts the drainer | **One deploy value must change**: the shipped default `ACCOUNT_URL: ${ACCOUNT_URL:-http://account:8091}` names no service — the service is `account-api` — so the deploy must set `ACCOUNT_URL=http://account-api:8091` (or the stack default is corrected). `PORTFOLIO_URL`'s default `http://portfolio:8101` already matches. No new environment variable and no new secret |
| `identity-worker` | compute | Already receives `AUDIT_URL` and the database URL | **Runs `PrivacyWorker` only; it drains no outbox.** This unit adds nothing to it — the support sink joins the drainer in `identity-api` |
| `account-api` / `portfolio` | compute (metadata owners) | Both listen for internal traffic (8091 and 8101) and both verify an inbound service token by hashing it and looking the caller up in their own `*.service_token_registry` table — `PgServiceTokenRegistry`, not a mounted token file. The `identity` caller is already registered on both, holding `privacy_export`/`privacy_anonymise` only | Owned by their units; this unit is a client only. The new metadata operation is the **prerequisite** recorded in "Configuration this unit depends on" |
| API Gateway | edge | The existing limiter — an in-process fixed-window map, and the gateway runs **2 replicas**, so a configured budget is per replica and the fleet-wide ceiling is twice it. A new optional `max_requests_per_minute` on `Route`, with the token-hash bucket in its own map bounded separately from the IP keys | No new route table entry beyond the four support paths; no config file change. The per-replica doubling and the token-hash bound are stated in `nfr-design/security-design.md` |
| Audit service (C10) | external service | Existing synchronous and outbox delivery paths | Append-only, insert-only role; this unit only posts events |
| Supabase Vault (pgsodium) | secrets store | Existing | The `identity` service token is provisioned there; no new secret |
| Prometheus + Grafana | observability | The new rule file `infra/prometheus/support-rules.yml` beside `job-orchestration-rules.yml`, same shape. **No scrape job is declared anywhere in the repository**, so the rules can only fire where the platform's Prometheus already scrapes `identity-api` and `identity-worker` | The collection gap is recorded in `monitoring-design.md`, not papered over |
| Loki / Tempo | observability | **No Loki, Tempo, Promtail or OTLP exporter exists in the repository.** Identity logs to stdout through `tracing`/`tracing-subscriber`; no `opentelemetry` dependency appears in any `Cargo.toml` or source file, and `infra/` holds only `prometheus/job-orchestration-rules.yml` | The new log fields reach stdout today. The span structure in `monitoring-design.md` is what the unit will emit once the platform lands an exporter — recorded as a platform task beside the scrape job, not claimed as existing |

## Shared Infrastructure

| Shared Resource | Owner Unit | Consumer Units | Access Boundary |
|---|---|---|---|
| `identity` schema and `identity_app` role | U1 (identity service) | u2, u3, u4, u5 and the worker units | One role, one schema; no cross-schema grant. The new table is granted to `identity_app`, `identity_readonly` and `identity_migrator` only |
| Identity's `PgPool` | U1 | u3 (read and mutations), the drainer | Shared pool, no second pool; statement timeouts bound every support statement |
| `OutboxDrainer::run_once` | U1 (consent units) | u3 adds one sink | The support sink has its own batch (5) and client (2 s); the consent and status sinks' settings are untouched |
| API Gateway route table and limiter | gateway unit | u3 (four routes), every other unit | The new field is optional; routes without it behave exactly as today |
| `audit-api` (C10) | audit unit | u1, u2, u3, every service posting events | Insert-only store; identifiers and enum codes only |
| `infra/prometheus/` | platform | u3 adds one file | Rule files are per-domain; the support alerts live in their own file |

## Configuration this unit depends on

Three items the unit needs and does not own. Each is small, each is a prerequisite of the metadata
read, and none of them provisions anything new.

1. **`ACCOUNT_URL` must resolve.** `infra/swarm-stack.yml` defaults it to `http://account:8091`,
   but the service is `account-api`; nothing is named `account`. The deploy sets
   `ACCOUNT_URL=http://account-api:8091`, or the stack default is corrected to match the service
   name. Until it does, the Account hop fails and every metadata read is refused.
2. **A new operation in both service-token registries.** Neither service verifies a mounted token
   file on the inbound side: each hashes the presented token and looks it up in its own
   `*.service_token_registry` table, then checks the caller's operation allow-list. So the two
   metadata routes need a new `Operation` variant on each side (for example
   `support_metadata_read`) and a registry row granting it to `identity`.
   `infra/sql/portfolio-service-tokens.sql` already registers `identity` for
   `privacy_export`/`privacy_anonymise`, so the operation joins that array; Account has **no
   `infra/sql/` seed file at all** — the local stack seeds it from
   `deployment/localhost/init/03-tokens.sql` — so the production Account seed is a gap this unit
   records rather than closes.
3. **The gateway route entries.** The four `/v1/support/...` paths are added to the deny-by-default
   route table by the gateway unit; this unit only supplies the field and the two boundary tests.

## What is deliberately not provisioned

No new database, cache, queue, broker, CDN, load balancer, or network. No autoscaling rule
(scalability-design.md records why). No new secret. No new CI platform — the pipeline section
records the gates and their intended home, which is an open platform decision the team has kept
open.