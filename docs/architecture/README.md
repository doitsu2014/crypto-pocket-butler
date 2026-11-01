# Crypto Pocket Butler — C4 Architecture

Architecture documentation for **Crypto Pocket Butler**, a read-only crypto portfolio
aggregation and decision-support platform.

The model of record is the Structurizr DSL workspace in [`workspace.dsl`](./workspace.dsl).
This README explains that model: how to open it, which views exist, what each container and
service owns, how the Clean Architecture rules are applied, how the system is deployed, and
which business requirements each part satisfies. If this document and the DSL disagree about
structure, the DSL is authoritative. If they disagree about a business rule, the
[BRDs](../brds/README.md) are authoritative.

The model describes a **Supabase-only managed infrastructure**: the Rust services and the
React single-page application run on a container host, while authentication, relational data,
queues, object storage, secrets, scheduling and observability are consumed from Supabase.

| Property | Value |
| --- | --- |
| Model file | [`workspace.dsl`](./workspace.dsl) |
| Requirements baseline | BRD-001 … BRD-012, Draft v1.0 |
| Reviewed against the DSL | 2026-11-01 |
| People | 2 |
| External software systems | 14 (7 Supabase services + 7 third-party integrations) |
| Containers | 10 (1 React SPA + 9 Rust services) |
| Components | 318 |
| Views | 16 |
| Deployment environments modelled | 1 (`Production`), one `Container Host` node |

---

## 1. Purpose and architecture goals

### 1.1 Purpose

This documentation exists to give product, engineering, security and operations one shared,
diagram-backed description of the system before and during implementation. It answers four
questions:

1. **What is the system boundary?** Which capabilities the platform owns and which belong to
   Supabase, exchanges, blockchains, price providers, OpenClaw, Notion or the alerting channel.
2. **Who owns which data?** Every service owns its own schema and Postgres role, and no service
   reads another service's tables.
3. **How do the parts communicate?** Synchronous request/response is used only where a caller
   needs an answer; all state propagation, job dispatch and audit flow use Supabase Queues
   (pgmq).
4. **Why was it built this way?** The decisions and their costs are recorded in section 13 so
   they can be challenged rather than re-learned.

The model covers C4 levels 1–3, one deployment view and four dynamic views. It is not a class
diagram, an OpenAPI specification or an implementation plan. Those artefacts live with the code.

### 1.2 Architecture goals

| Goal | What it means in practice | Where it is visible |
| --- | --- | --- |
| **G1 — Read-only by construction** | No private keys, seed phrases, withdrawal rights or trade execution exist in the MVP. Exchange credentials are read-only. | Account Service, `Read-Only Credential Policy`, Supabase Vault credential encryption, OKX connector |
| **G2 — Trustworthy valuation** | Quantities and values use decimal arithmetic; prices carry source and observation time; unpriced and unmapped assets are labelled and excluded from the valued total, never invented. | Portfolio Service `Valuation Calculator` and `Valuation Line Status`, Market Data Service `Price Freshness`, Asset Registry `Asset Identity Resolver` |
| **G3 — Clear service ownership** | Nine Rust services; each of the eight domain services owns one schema and one Postgres role in Supabase Postgres, and the gateway owns only a cache schema. They communicate through APIs or events. | `Containers` view, section 3.3, section 6 |
| **G4 — Human-in-the-loop advice** | Recommendations are deterministic, explainable and advisory; accept/reject is a recorded decision, not an execution. | Portfolio Service `Recommendation`, section 11 |
| **G5 — Auditability and privacy** | Security- and financially-material events are append-only and hash-chained; exports and erasure are coordinated across services, with erasure by crypto-shredding rather than mutating the chain. | Audit Service, Supabase Storage, section 11 |
| **G6 — Deterministic operations** | UTC schedules survive restarts; runs are single-flight, recorded, retried with bounds and detected when missed. | Job Orchestration Service, Supabase Cron (pg_cron), section 11 |
| **G7 — No secret exposure** | Credentials are encrypted with Supabase Vault (pgsodium), masked, never returned, never logged and never placed in briefs or events. | Supabase Vault, Account Service, Reporting Service, section 11.2 |
| **G8 — Evolvable boundaries** | Providers, chains and suppliers are behind adapters; the registry is data-driven; projections are rebuildable. | Ports and adapters in every service, section 8 |
| **G9 — MVP-sized delivery** | One deployment environment, one exchange, USD only, single-tenant ownership, no premature multi-tenancy or execution engine. | Deployment view, section 10, section 13 |

### 1.3 Non-goals of this documentation

- Class-level design, crate-by-crate APIs or database DDL.
- A committed runtime topology for development or staging; only `Production` is modelled.
- Vendor selection for CI/CD, backup tooling or disaster recovery beyond what the DSL states.
- Restating business rules. The BRDs remain the source of truth for **what** the business needs;
  this document describes **how** the architecture supports it.

---

## 2. How to view the model

### 2.1 Tooling status (verified 2026-11-01)

| Tool | Status in this environment | Consequence |
| --- | --- | --- |
| Docker CLI | Installed | The command is available |
| Docker daemon | Not running | Structurizr containers cannot start until Docker Desktop is started |
| Java | Not on `PATH` | `structurizr-cli` cannot run directly |
| `structurizr-cli` | Not installed | No direct export without Java or Docker |
| Generated diagrams | None committed | The DSL is the only render input in this folder |

To render the model today, start Docker Desktop. To use the CLI export path, install a Java 17+
runtime and `structurizr-cli`, or use the Docker-based CLI command below.

### 2.2 Interactive viewing with Structurizr Local

The bind mount must point at the **directory** containing `workspace.dsl`, because Structurizr
Local looks for `/usr/local/structurizr/workspace.dsl` inside the container.

PowerShell:

```powershell
docker run -d --rm --name structurizr `
  -p 8080:8080 `
  -v "${PWD}\docs\architecture:/usr/local/structurizr" `
  structurizr/structurizr local
```

Bash:

```bash
docker run -d --rm --name structurizr \
  -p 8080:8080 \
  -v "$(pwd)/docs/architecture:/usr/local/structurizr" \
  structurizr/structurizr local
```

Then open <http://localhost:8080>. The left-hand view list shows the 16 views in section 4.
Structurizr Local reads the DSL on start and writes a generated `workspace.json` plus a
`.structurizr/` log directory; both are ignored by `docs/architecture/.gitignore` and are not
source artifacts. Restart the container after editing the model.

### 2.3 Exporting diagrams with Structurizr CLI

When Java 17+ and `structurizr-cli` are available:

```bash
# Validate the model first
structurizr-cli validate -w docs/architecture/workspace.dsl

# Export every view to Mermaid
structurizr-cli export \
  -w docs/architecture/workspace.dsl \
  -f docs/architecture/export \
  -t mermaid
```

Other export formats supported by the CLI (for example PlantUML) can be selected with `-t`.
Exports are build artefacts; they are not part of the model of record.

### 2.4 Docker-based CLI alternative

If Docker is running but Java is not installed:

```bash
docker run --rm \
  -v "$(pwd)/docs/architecture:/usr/local/structurizr" \
  structurizr/cli validate \
  -w /usr/local/structurizr/workspace.dsl
```

```bash
docker run --rm \
  -v "$(pwd)/docs/architecture:/usr/local/structurizr" \
  structurizr/cli export \
  -w /usr/local/structurizr/workspace.dsl \
  -f /usr/local/structurizr/export \
  -t mermaid
```

On PowerShell, replace `$(pwd)` with `${PWD}` and use backslashes in the mount path.

### 2.5 What to check after rendering

- Every application container in the `Containers` view has a justified owner and a data
  decision: each backend service owns one Supabase Postgres schema and role, the gateway owns a
  cache schema only, and the SPA owns none.
- Every Rust service opens a component view with exactly four groups: `Transport`,
  `Application`, `Domain`, `Infrastructure`.
- Asynchronous relationships are dashed: every Supabase Queues consumer, publisher and
  coordinator relationship, plus the account SSE progress-stream edge, is tagged `Async`.
- The deployment view contains the `Container Host` and every service container instance.
  Supabase services are external systems, not deployment nodes.
- No view is empty; `autolayout` may still need manual adjustment in the Structurizr UI.

---

## 3. Architecture principles

### 3.1 Rust services

All nine backend services are Rust binaries built with Axum and `tower`. Persistence uses
`sqlx`; money-like values use `rust_decimal`; the event backbone is Supabase Queues (pgmq)
accessed through Postgres; background work uses Tokio.

- One deployable binary per service. Portfolio Service runs as two process groups on the
  container host — an interactive process group and a separate batch worker process group —
  while the other eight services run one process group each.
- Synchronous internal calls use `reqwest` over TLS on the container host's private network,
  authenticated by a signed internal principal assertion (section 11.1).
- DTOs and error types stay in the transport layer; domain types do not serialize themselves
  for HTTP.
- Blocking or CPU-heavy work is isolated (the Portfolio Service advisory worker pool is the
  explicit example).
- Services own their schema and migrations inside the shared Supabase Postgres project; there
  is no shared migration repository and no cross-schema grant.

### 3.2 Standard Clean Architecture and the dependency rule

Every service is described with the same four layers:

| Layer | Responsibility | May depend on | Must not contain |
| --- | --- | --- | --- |
| **Domain** | Aggregates, entities, value objects, policies, calculators, invariants | Rust standard library, domain-only crates such as `rust_decimal` | Axum, SQL, HTTP clients, Supabase Queues, Supabase Vault, Supabase Storage, transport DTOs |
| **Application** | Use cases, orchestration, transaction boundaries, ownership checks, idempotency; declares **ports** as traits | Domain, port definitions, pure application types | SQL strings, HTTP framework types, provider SDKs |
| **Infrastructure** | Repository adapters, provider clients, Supabase Vault client, Supabase Queues publishers and consumers, Supabase Postgres caches, Supabase Storage client | Application ports, Domain | Business decisions, authorization policy |
| **Transport** | Axum routers, middleware, extractors, SSE handlers, Supabase Queues consumers, request/response mapping | Application use cases | Direct database or provider calls, duplicated domain rules. Named exception: edge adapters may call Infrastructure directly for health, readiness and operational probes; for Supabase Queues consumer projection/cache writes; for SSE ticket verification; and for any route guard or handler (public, admin, privacy, internal or retry) that invokes its `Principal Assertion Verifier` (section 8.2 item 6). |

The dependency rule is about **source-code imports**:

```text
Transport ──────► Application ──────► Domain
                       ▲
                       │ implements ports declared by Application
Infrastructure ────────┘
```

- Domain depends on nothing outward.
- Application depends on Domain and on the port traits it declares; it does not import
  infrastructure crates.
- Infrastructure depends on Application (to implement ports) and Domain (to map types).
- Transport depends on Application; it invokes use cases and maps errors, but owns no rules.

The C4 component diagrams show **runtime and data-flow** arrows, not import arrows. A
relationship such as `accountSyncAccountUseCase -> accountRepository` means the use case calls a
repository port; the Supabase Postgres adapter implements that port. The source dependency of
the adapter still points inward.

### 3.3 Microservices with logical database-per-service

Services are split by business capability, not by technical layer. All nine Rust services share
one managed Supabase Postgres project, but each one connects with its own Postgres role and
reads or writes only its own schema. This is **logical database-per-service**, not physical
database-per-service: isolation is enforced by schema ownership and role grants, not by separate
database servers.

| Service | Owned Supabase Postgres schema and role | Data owned |
| --- | --- | --- |
| Identity Service | `identity` | Internal users, Supabase `auth.users.id` mapping, stable owner UUID, roles, status, retention state, outbox and inbox dedup |
| Account Service | `account` | Accounts, credential ciphertext and versions, wallet chains, holdings, sync runs, registry projection, outbox and inbox dedup |
| Asset Registry Service | `registry` | Assets, contracts, decimals, EVM chains, EVM/Solana token registries, lookup candidates, quarantine, affected-account projection, registry audit, outbox and inbox dedup |
| Market Data Service | `market` | Partitioned price observations, history, provider mapping and health, freshness state, last-known-price cache, outbox and inbox dedup |
| Portfolio Service | `portfolio` | Portfolios, links, targets, guardrails, projections, valuations, monthly snapshot partitions, recommendations, decisions, outbox and inbox dedup |
| Reporting Service | `reporting` | Preferences, schedules, briefs, narratives, delivery log, idempotency keys, outbox and inbox dedup |
| Job Orchestration Service | `job` | Job definitions, UTC schedules, runs, heartbeats, advisory-lock state, alert state, trigger audit, outbox and inbox dedup |
| Audit Service | `audit` (insert-only role plus a separate audited privileged retention role) | Hash-chained audit events, retention and legal hold, export and erasure state; export and retention archives live in Supabase Storage private buckets |
| API Gateway | `gateway` (cache schema only) | JWKS cache, principal status and roles, rate-limit counters — never a system of record |

Rules:

- No service reads or joins another service's tables, and no service is granted access to
  another service's schema. Cross-service reads go through documented internal REST endpoints
  over TLS on the container host's private network.
- Internal endpoints require a signed internal assertion minted by the gateway; each service
  enforces owner scoping and re-checks admin roles in its own admin use cases. Every route
  guard — public, admin, privacy and internal — runs a `Principal Assertion Verifier` adapter
  that resolves the gateway and Job Orchestration public keys from Supabase Vault and caches
  them. User-originated service-to-service hops forward the inbound assertion unchanged.
  Scheduled and recovery hops carry a run-scoped System assertion obtained by the owning
  service from the Job Orchestration run-assertion exchange over mTLS; it never travels inside
  an event payload.
- **First sighting and lookup.** The SPA signs in through Supabase Auth. The gateway validates
  the Supabase JWT locally against the Supabase Auth JWKS. On a principal-cache miss it calls
  Identity's internal ensure-user endpoint over mTLS with the Supabase access token. Identity
  validates the token and idempotently upserts the internal user keyed by `auth.users.id`,
  returning the stable owner UUID, status and roles. The gateway caches that mapping and mints
  its signed internal principal assertion for service-to-service calls.
- No distributed transaction spans services. A service commits its own state and publishes
  events from a transactional outbox.
- Data another service needs is either fetched synchronously for recovery or projected locally
  from events; projections are explicitly rebuildable.
- Shared Supabase infrastructure carries messages, cache entries or isolated schemas, not
  shared business records. The gateway cache is cache-only and never authoritative.
- All reads use Supabase Postgres, which guarantees read-your-writes inside the project.
  Optional Supabase read replicas are a future scaling option only and are not modelled as
  current elements; they would serve read-heavy paths within a bounded lag budget while
  valuation, snapshot capture and recommendation computation would keep reading the primary.
- Every service exposes health and readiness endpoints; asynchronous event envelopes carry the
  correlation and trace context originated at the gateway.

**Trade-off.** Logical database-per-service keeps one managed Postgres project, one connection
pooler and one backup/PITR policy, which is far cheaper to operate than eight physically
separate databases. The cost is a shared blast radius: a project-level outage, storage
exhaustion or bad migration can affect every service. Per-service roles, schema ownership,
connection quotas, no cross-schema grants and per-service migration discipline contain the
logical blast radius, but they do not give independent restores or independent engine
upgrades. Physical database-per-service would trade operational cost for stronger isolation.
This decision is revisited in section 13 and section 15.

### 3.4 React single-page application

`React Web Client (SPA)` is built with React 18, TypeScript 5, Vite, TanStack Query,
`@supabase/supabase-js` and Recharts.

- Authentication uses Supabase Auth with PKCE: email/password, magic link, OAuth and MFA. The
  client keeps access and refresh tokens in memory only and never persists them in
  `localStorage`. It refreshes sessions directly against Supabase Auth.
- The SPA calls **only** `API Gateway (BFF)` for application data. It does not call services,
  Supabase Postgres, Supabase Queues, Supabase Vault or external providers directly. The one
  exception is interaction with Supabase Auth endpoints for sign-up, sign-in, token refresh,
  MFA and sign-out; these are the only direct browser-to-Supabase calls.
- Server state is owned by TanStack Query; the SPA does not become the source of truth for
  holdings, valuations or recommendations.
- Bulk sync progress is consumed through the gateway's SSE proxy.
- The SPA is modelled at container level only; no component view for the client is claimed.

### 3.5 Synchronous versus asynchronous integration

| Interaction | Rule | Examples |
| --- | --- | --- |
| Browser to platform | Synchronous HTTPS only, through the gateway, except Supabase Auth | Dashboard reads, account creation, snapshot capture |
| Browser to Supabase Auth | Direct HTTPS for sign-up, sign-in, refresh, MFA and sign-out | Supabase Auth only |
| Gateway to service | Synchronous REST over TLS on the private network for the user's request | Account sync request, valuation read, admin chain update |
| Service to service, synchronous | Only for an answer needed now, for recovery, for a consistent read or for an ownership check | Identity ensure-user and lookup, registry snapshot, holdings rebuild, brief data fetch, account ownership validation, run-assertion exchange |
| State propagation | Asynchronous event on Supabase Queues (pgmq) | Holdings replaced with aggregate version, registry changed, price observed, account deleted |
| Job dispatch | Asynchronous command on Supabase Queues with a logical run key and per-attempt id; the owning service exchanges the command for a short-lived run-scoped assertion over mTLS | `job.run.requested`, run-assertion exchange |
| Security and delivery audit | Asynchronous event to Audit Service | Access denied, brief published, registry change, manual job trigger |

There are no synchronous write chains across services and no service-to-service call cycle. A
user action completes inside one owning service and the rest of the system converges through
events. Between Account and Registry the only synchronous direction is Account fetching a
registry snapshot on cold start; Registry maintains an event-fed affected-account projection
instead of calling Account before chain deactivation. Portfolio's synchronous reads of Account,
Registry and Market are recovery-only snapshot fetches for projection rebuild, not write chains.
Every internal synchronous edge has a timeout, bounded retry and circuit breaker and documents
its degraded mode.

---

## 4. C4 view index

`workspace.dsl` defines 16 views. View keys are stable identifiers; titles are the descriptions
shown by Structurizr.

| View key | Title | Level | What it shows | Primary audience |
| --- | --- | --- | --- | --- |
| `SystemContext` | System Context | Level 1 | The two personas, Crypto Pocket Butler as a single system, the seven Supabase services and the seven third-party integrations with their protocols. | Product, leadership, security, new joiners |
| `Containers` | Containers | Level 2 | All ten containers — React SPA and nine Rust services — plus their synchronous and asynchronous relationships to Supabase and the external providers. | Engineering, architecture, security |
| `ApiGatewayComponents` | API Gateway Components | Level 3 | Public router, health, JWT and admin middleware, SSE proxy, identity-event consumer; token, principal, role, route and stream-ticket use cases; principal/status/role/limit policy and signed assertion; cached JWKS, Supabase Postgres principal cache and rate limiter, Identity client, assertion signer, stream-ticket signer, Supabase Vault client and fail-closed audit channel. | Backend and security engineers |
| `IdentityComponents` | Identity Service Components | Level 3 | Public and internal REST with Supabase bearer and admin guards, internal ensure-user handler, Supabase Auth hook handler, auth-hook verification and privacy/job consumers; ensure-user, role-projection, session-record, revoke-sessions, status, deletion, retention, export and lookup use cases; internal user aggregate, role assignment, token/provisioning/auth-hook/privacy/admin policy; repository, Supabase Admin API client, principal-cache eviction signal, assertion verifier, publishers and health. | Backend, security, identity engineers |
| `AccountComponents` | Account Service Components | Level 3 | Account, internal with principal guard and operations rebuild, SSE with stream-ticket verifier, job-command, registry-event and privacy surfaces; create, update, activate, sync, sync-status, all-accounts fan-out, bulk-sync, delete, internal-query, projection-rebuild, export and erase use cases; account aggregate, holding, balance normaliser, read-only credential, holdings-event and lifecycle-event policies; repository, per-account sync lock, assertion verifier, registry snapshot client, Supabase Vault credential cipher and OKX/EVM/Solana connectors, event and run-outcome publishers. | Backend, integrations, security |
| `AssetRegistryComponents` | Asset Registry Service Components | Level 3 | Asset and admin handlers with admin guard, internal snapshot with principal guard, unmapped/account-event/job/privacy consumers; query, register, map, chain, import, catalogue-sync, quarantine, deactivation-evaluation, snapshot, export and erase use cases; asset, contract, EVM chain, identity resolver, secret-reference, admin-authorization and event-versioning policy; repositories, assertion verifier, lookup client, affected-account projection, Supabase Vault reader and publishers. | Backend, data, operations |
| `MarketDataComponents` | Market Data Service Components | Level 3 | Market and admin handlers with admin guard, internal price/registry/privacy/job surfaces; collect, latest, freshness, provider-update, registry-apply, export, erase and retention use cases; observation, freshness, fallback, provider-status, admin-authorization and secret-reference models; price repository, assertion verifier, CoinGecko/CoinPaprika clients, Supabase Vault reader and Supabase Postgres last-known-price cache. | Backend, data, operations |
| `PortfolioComponents` | Portfolio Service Components | Level 3 | Portfolio, valuation/history, recommendation/decision, internal-brief with rebuild endpoint, job, account-event, price-and-registry-event and privacy surfaces; lifecycle, delete, ownership validation, valuation, snapshot, recommendation generation and decision, expiry, scheduled fan-out, reconciliation, projection rebuild, query, retention, export and erase use cases; aggregate, calculator, valuation-line status, snapshot, recommendation and immutability/supersession/configuration/admin policy; repository, projections, assertion verifier, advisory and batch worker pool, account/registry/market clients, Supabase Vault client and publishers. | Backend, finance, product |
| `ReportingComponents` | Reporting Service Components | Level 3 | Reporting, brief-dispatch, domain-event, internal-brief, retry and privacy surfaces; preferences, history, compose, publish, dispatch, retention, export and erase use cases; brief, composer, delivery record, idempotency, retry and payload-contract policies; repository, assertion verifier, portfolio client, OpenClaw client, Supabase Vault reader and publishers. | Backend, operations |
| `JobOrchestrationComponents` | Job Orchestration Service Components | Level 3 | Admin with list/schedule/trigger/cancel/backfill/run-history, internal-guard, run-outcome-consumer, scheduler-tick loop, missed-run watchdog, internal health, run-assertion exchange and health surfaces; tick, manual-trigger, outcome-recording, stale-run expiry, retry, cancel, backfill, list and schedule-update use cases; job definition, schedule, run with logical run key, attempt id and lease epoch, retry, single-flight, run-lease and admin policy; definition and run repositories, Supabase Postgres advisory lock, command publisher, System assertion signer, assertion verifier, alert webhook client and Supabase Vault client. | Backend, operations, SRE |
| `AuditComponents` | Audit Service Components | Level 3 | Admin with admin guard, privacy with owner guard, internal guard, audit-event, export-part and job consumers; append, query, export-request, assemble, retention and erasure use cases; audit event, hash chain, chain-writer, export request, retention, PII-pseudonymisation and admin policy; append-only and privileged repositories, per-partition chain writer lock, assertion verifier, privacy coordinator, Identity lookup client, Supabase Storage client, Supabase Vault reader and publisher. | Security, compliance, backend |
| `ProductionDeployment` | Production Deployment | Deployment (supplementary) | The `Production` environment with a single `Container Host` node holding the SPA and every Rust service process group; Supabase managed services and third-party providers remain external. | Operations, SRE, platform |
| `AccountSyncToValuation` | Account Sync to Valuation | Dynamic (flow, software-system scope) | SPA sync request to gateway to Account Service, atomic holdings replacement in Supabase Postgres, versioned `account.holdings.replaced`, Portfolio projection update by aggregate version, and the post-sync valuation from Supabase Postgres. | Engineering, product |
| `AccountSyncProgress` | Account Sync Progress | Dynamic (flow, gateway container scope) | Gateway components only: stream-ticket issuance and signing from a Supabase Vault-resolved key, and assertion minting for the routed sync request. | Engineering, security |
| `AccountSyncProjection` | Account Sync Projection | Dynamic (flow, Portfolio container scope) | Portfolio components only: account-event consumption, version-checked projection update, and the post-sync valuation read from Supabase Postgres. | Engineering, product |
| `RecommendationToReporting` | Recommendation to Reporting | Dynamic (flow, software-system scope) | Recommendation persistence, `recommendation.generated`, Reporting Service brief composition, redacted payload validation, OpenClaw publication to Notion, delivery outcome, and the audit record in Supabase Postgres. | Engineering, product, operations |

The SPA has no component view. The deployment view covers only `Production`; development and
staging environments are intentionally out of scope.

---

## 5. System context summary

`Crypto Pocket Butler` is one software system and one deployment boundary. It is a single-tenant
MVP that aggregates read-only holdings, values portfolios in USD, computes drift, proposes
human-reviewed recommendations and publishes scheduled briefs.

### 5.1 Actors

| Actor | Type | Responsibilities in the model |
| --- | --- | --- |
| **Portfolio Owner** | Person | Authenticated retail investor. Connects read-only exchange and wallet accounts, builds portfolios, reviews valuation, allocation, drift, snapshots and recommendations, and configures reporting. |
| **Admin** | Person | Holder of the Supabase `app_metadata.role` admin claim projected by the Custom Access Token Hook. Manages the chain and token registry, job schedules, audit queries and privacy requests. An Admin is also a Portfolio Owner for their own portfolios. |

The **System** actor described in BRD-002 is not a C4 person: it is represented by Job
Orchestration commands and by service-to-service calls with no interactive session.

### 5.2 External systems

The model has fourteen external software systems: seven Supabase services and seven third-party
integrations.

| External system | What it provides | Protocol in the model |
| --- | --- | --- |
| **Supabase Auth** (GoTrue) | Email/password, magic link, OAuth, MFA, JWT issuance and refresh, sessions, JWKS, a custom access-token hook projecting roles into `app_metadata.role`, and auth hooks/webhooks for user lifecycle. | HTTPS; Supabase Auth endpoints from the browser, JWKS and Admin API from services |
| **Supabase Postgres** | Managed Postgres with logical database-per-service: one schema and one role per service, no cross-schema grants, pgmq queues, outbox and inbox dedup tables, and the Supavisor connection pooler. | SQL over TLS via Supavisor |
| **Supabase Queues (pgmq)** | Durable Postgres-backed queues with visibility timeouts, archive and dead-letter handling and at-least-once delivery for domain events, job commands, run outcomes and privacy requests. | SQL/pgmq through Supabase Postgres; clients resolve queues through their service role |
| **Supabase Storage** | Private buckets for audit export and retention archives, accessed only through signed, access-controlled URLs. | Supabase Storage API over HTTPS with signed URLs |
| **Supabase Vault** (pgsodium) | pgsodium-backed secrets store for exchange credentials, provider keys and service credentials, with versioned encryption and no plaintext exposure to logs or events. | Supabase Vault (pgsodium) through the owning service's Postgres role |
| **Supabase Cron** (pg_cron) | Scheduled SQL that enqueues `job.tick` into Supabase Queues, runs retention triggers and hosts the dead-man watchdog that calls the alerting channel through pg_net. | SQL / pg_net |
| **Supabase Logs & Metrics** | Platform logs, metrics, log drains and alerts for every service, with correlation and trace context. | Log drain and metrics ingestion |
| **OKX Exchange API** | Read-only balances and holdings. | HTTPS REST with API-key, secret and passphrase HMAC signing |
| **EVM JSON-RPC Providers** | Primary and fallback JSON-RPC endpoints per active EVM chain; provider rotation, health, circuit breakers and shared request budgets; native and ERC-20 balances and token metadata. | JSON-RPC 2.0 over HTTPS |
| **Solana RPC Provider** | SOL and SPL balances plus mint metadata. | JSON-RPC over HTTPS |
| **Price Data Providers (CoinGecko / CoinPaprika)** | USD spot price, volume, market capitalisation, rank, 24-hour change, provider fallback, a shared per-provider request budget, and token lookup candidates for admin review. | HTTPS REST/JSON with API key where required |
| **OpenClaw Agent** | Composes the narrative brief from a versioned, redacted single-owner payload and publishes it to Notion under an idempotency key. | HTTPS JSON API |
| **Notion** | One-way destination for daily and weekly briefs. Reachable only through the OpenClaw Agent. | HTTPS REST/JSON with a scoped token |
| **Operational Alerting Channel** | Pager/on-call consumer for missed runs, failed schedules, error-rate and stale-price alerts. | Alert webhook |

### 5.3 Context relationships

- Both personas use the React SPA over HTTPS; the SPA calls the API Gateway for application
  data and Supabase Auth for sign-up, sign-in, MFA and token refresh.
- The SPA and gateway trust Supabase Auth for authentication. The gateway validates Supabase
  JWTs locally against a cached JWKS. On a principal-cache miss the gateway calls Identity's
  internal ensure-user endpoint over mTLS with the Supabase access token; Identity validates
  the token and idempotently upserts the internal user keyed by `auth.users.id`, returning the
  stable owner UUID, status and roles. Identity also receives Supabase Auth hooks for user
  lifecycle events. The gateway mints a signed internal principal assertion for every routed
  request; every internal endpoint verifies it against Supabase Vault-resolved, cached public
  keys. Scheduled and recovery hops use a run-scoped System assertion exchanged over mTLS with
  Job Orchestration.
- Account Service reads OKX, EVM RPC and Solana RPC with read-only credentials or public
  addresses only.
- Market Data Service and Asset Registry Service call the approved price/data providers.
- Reporting Service calls the OpenClaw Agent; the OpenClaw Agent alone writes to Notion and alone
  holds Notion credentials. There is no direct Notion write path in the platform.
- Every service uses Supabase Vault for its Postgres credentials and the secrets it owns. Account
  Service encrypts exchange credentials with Supabase Vault (pgsodium); Asset Registry and
  Market Data resolve RPC/provider secrets; Audit uses per-subject keys for crypto-shredding;
  Reporting resolves only the scoped OpenClaw token; Notion credentials stay with the OpenClaw
  Agent. The gateway owns no business schema and uses Supabase Vault only for internal-call
  credentials and assertion- and ticket-signing keys.
- Supabase Cron enqueues `job.tick` into Supabase Queues on each job schedule and runs the
  independent dead-man watchdog that raises an alert through pg_net when scheduler heartbeats
  stop.
- Every service emits structured logs, metrics and traces with correlation context to Supabase
  Logs & Metrics; Job Orchestration and the Supabase Cron watchdog route alerts to the
  Operational Alerting Channel.

---

## 6. Service catalogue

The model has ten containers. The React SPA is a container but not a backend service; it is
described in section 3.4 and included here so the catalogue matches the `Containers` view. The
remaining nine are Rust services. Event names in backticks are named explicitly in the DSL;
other entries are event families described in section 7.

| Container | Responsibility | Owned schema / data | Synchronous API | Published events | Consumed events |
| --- | --- | --- | --- | --- | --- |
| **React Web Client (SPA)** | Authenticated interface for dashboard, accounts, portfolios, valuation, drift, snapshots, recommendations, reporting preferences and admin screens. Uses Supabase Auth with `@supabase/supabase-js` for sign-up, sign-in, OAuth, magic link, MFA and PKCE; keeps tokens in memory; calls only the API Gateway for application data. | None | Calls the gateway's `/api/v1` routes and SSE progress stream; calls Supabase Auth directly for sign-up, sign-in, refresh, MFA and sign-out. | None | None |
| **API Gateway (BFF)** | Single public entry point and stateless edge BFF. Validates Supabase JWTs locally against the Supabase Auth JWKS with a cached JWKS in memory and Supabase Postgres, resolves principals from a Supabase Postgres cache and calls Identity's ensure-user endpoint only on a cache miss, enforces deny-by-default routing and admin-claim gating, applies rate limits, CORS, security headers and correlation IDs, mints a signed internal principal assertion, and routes to owning services. Owns no business data and holds no server-side session store. | `gateway` schema in Supabase Postgres: JWKS cache, principal status/roles, rate-limit counters. Never authoritative. | Public `/api/v1/*` routes, stream-ticket issuance, health and readiness, SSE sync-progress proxy. | `security.access_denied`, `security.rate_limited` (published synchronously before the request completes) | Identity user, role and status events (principal-cache eviction) |
| **Identity Service** | Internal identity and trust boundary. Validates Supabase JWTs as defence in depth, receives Supabase Auth hooks for sign-up, sign-in, sign-out and user-update, verifies the standard-webhooks signature and idempotency, lazily upserts the internal user keyed by the Supabase `auth.users.id` UUID through an ensure-user endpoint that accepts the Supabase access token over mTLS, denies disabled users, projects roles from the `app_metadata.role` claim, issues the stable owner UUID, records session events, revokes sessions through the Supabase Admin API, applies a documented retention period, and owns user deletion and anonymisation policy. Holds no passwords, MFA state or server-side session store. | `identity` schema: users, `auth.users.id` mapping, owner UUID, roles, status, retention state, outbox and inbox dedup. | `GET /api/v1/me`, logout, admin user-status and deletion endpoints; internal ensure-user and lookup over mTLS; Supabase Auth hook endpoint. | Identity user, role, session, status-change, `user.deleted`, export-part, retention-applied, audit and run-outcome events | Privacy export and erasure requests, `identity.retention` job commands |
| **Account Service** | Read-only account lifecycle and the complete holdings sync pipeline: type-specific validation, Supabase Vault-backed credential encryption and masking, public-address-only wallets, create/update/activate/deactivate/delete, idempotent single and bulk syncs, versioned holdings and lifecycle events, decimal normalisation, zero filtering, cross-chain merging, fault isolation, sync-status reporting and an admin-triggered registry-projection rebuild. Never accepts private keys or seed phrases. | `account` schema: accounts, credential ciphertext and versions, wallet chains, holdings, sync runs, registry projection, outbox and inbox dedup. | Account create, update, activation, deletion, single sync and sync status; bulk sync over SSE with stream tickets; internal owner account and holdings queries and the operations rebuild endpoint over mTLS. | Account lifecycle (`account.created/updated/activated/deactivated/deleted`), versioned holdings replacement (`account.holdings.replaced`), sync outcome, unmapped observation, export/erasure confirmation, audit and run-outcome events (the run outcome is published once by the fan-out; per-account progress is separate) | Registry events, `job.run.requested` (`accounts.sync`), privacy requests |
| **Asset Registry Service** | Single authority for canonical asset identity and admin configuration of chains and tokens; unique symbols; one contract or mint per asset per chain; decimals immutable after mapping; native assets resolved from the chain native symbol; EVM chains with primary/fallback RPC secret references; EVM and Solana token registries; review-before-activation lookup and import; catalogue-to-registry sync; quarantine of unmapped observations; event-fed affected-account index; versioned registry change audit. All administration is behind an admin guard and admin policy; the public asset handler serves catalogue queries only. | `registry` schema: assets, contracts, decimals, chains, token registries, lookup candidates, quarantine, affected-account projection, registry audit, outbox and inbox dedup. | Canonical asset catalogue and admin-guarded chain, token, lookup, import, catalogue-sync and chain-deactivation endpoints; internal registry snapshot over mTLS. | Versioned registry asset, contract, chain and token events; registry-change audit; export/erasure confirmation and run-outcome events | Unmapped observations, account lifecycle and wallet-chain events, `catalogue.refresh` commands, privacy requests |
| **Market Data Service** | Point-in-time USD market observations; deployment-configured primary provider with fallback to the approved secondary and then the last known stored price under a shared provider budget; source and retrieval time; retained history; freshness and provider health; maximum stale age after which an asset is unpriced; admin-guarded provider configuration; retention; export/erasure. | `market` schema: partitioned price observations, history, provider mapping, provider health and freshness state, last-known-price cache, outbox and inbox dedup. | Latest prices, batch prices and freshness; admin-guarded provider configuration; internal latest prices over mTLS. | Price observation, batch, freshness, provider-health, export/erasure confirmation and run-outcome events | Registry asset events, price-collection and retention commands (`prices.collect`, `prices.retention`), privacy requests |
| **Portfolio Service** | Portfolio lifecycle, links, targets and guardrails with unique-name, single-default and 100%-target invariants; account ownership and active-status validation at link time; deletion with cascade against retention and legal hold; valuation, allocation, weights and drift; explicit priced/stale/unpriced/unmapped valuation lines with exclusion from the valued total; first-write-wins manual and EOD snapshots idempotent on the logical run key plus portfolio, date and type; deterministic, superseded, decision-recorded and reproducible recommendations; holdings and price/registry projections with aggregate versioning, rebuild and scheduled reconciliation; scheduled all-portfolio fan-out with per-portfolio fault isolation and a partial-failure outcome; batch isolation and backpressure; export/erasure and retention. All reads use Supabase Postgres. | `portfolio` schema: portfolios, links, targets, guardrails, projections, valuations, monthly snapshot partitions, recommendations with recorded inputs, decisions, outbox and inbox dedup. | Portfolio lifecycle, deletion, links, targets, guardrails and oversight; valuation, drift, snapshot capture, history and trend; recommendation generation, history, accept/reject decisions; operations rebuild and internal brief data over mTLS. | Portfolio lifecycle, valuation, snapshot, recommendation (`recommendation.generated`), recommendation-decision, audit, export/erasure confirmation and run-outcome events | Versioned holdings and account lifecycle, price, registry, `job.run.requested` (`snapshots.eod`, `recommendations.expire`, `snapshots.retention`, `projections.reconcile`) and privacy events |
| **Reporting Service** | Reporting preferences and schedules; daily and weekly brief composition with owned structure and disclaimer; versioned redacted OpenClaw payload and response validation; least-privilege delegation of narrative and Notion publication; one delivery per owner, portfolio and period; bounded retry with pgmq archive/dead-letter and duplicate detection; brief history and delivery attempt log; retention; export/erasure and run outcomes. | `reporting` schema: preferences, schedules, briefs, narratives, delivery log, idempotency keys, outbox and inbox dedup. | Preferences, generate-now, brief history and delivery log; internal brief lookup by period; delivery retry restricted to the owner or an admin. | Brief generated, published, delivery-failure, audit, export/erasure confirmation, retention-applied and run-outcome events | Snapshot, valuation, recommendation, portfolio, `job.run.requested` (`briefs.dispatch`, `reporting.retention`) and privacy events |
| **Job Orchestration Service (Scheduler / Job Runner)** | Control plane and scheduler for non-interactive work: durable UTC schedules, `job.tick` signals enqueued by Supabase Cron, asynchronous commands carrying a logical run key (job + scheduled period), a per-attempt run id and a lease epoch used as a fencing token, heartbeat-renewed TTL leases, single-flight with Supabase Postgres advisory locks, run outcomes, stale-run expiry with epoch supersession and bounded retry, admin cancel and bounded backfill, a fixed-interval missed-run watchdog, alerts within 15 minutes through a webhook adapter, a run-assertion exchange over mTLS, and retention job definitions for identity, prices, snapshots, reporting and audit. Owns no business data. | `job` schema: job definitions, UTC schedules, run records with logical run key, attempt state and lease epoch, heartbeats, advisory-lock state, alert state, trigger audit, outbox and inbox dedup. | Admin job listing, schedule updates, manual triggers, cancel, backfill and run history with rate limiting and an admin guard; internal scheduler health over mTLS; run-assertion exchange over mTLS. | `job.run.requested` with logical run keys, attempt ids and lease epochs but never a credential or assertion; cancel and backfill commands; missed-run events; alerts; manual-trigger audit events | Run outcomes and heartbeats |
| **Audit Service** | Append-only compliance store and privacy operations: per-partition hash chains serialised by a Supabase Postgres advisory-lock writer lease so only one replica extends a chain at a time, insert-only application role, separate audited privileged retention role, archival/partition-drop retention, export assembly in Supabase Storage, erasure by crypto-shredding with pseudonymisation at ingestion and no post-hoc row mutation, coordinated export/erasure parts, queries and exports for authorised admins, owner-scoped privacy requests under a privacy guard. | `audit` schema with role separation plus private Supabase Storage buckets for export and retention archives. | Admin-guarded audit queries, export requests and retention administration; owner-guarded privacy export/erasure requests and processor disclosures. | Privacy requests, export-ready, retention-applied, erasure-confirmation and retention run-outcome events | Audit events, session and delivery events, per-service export and erasure parts, `audit.retention` commands |

---

## 7. Event and queue catalogue

Supabase Queues (pgmq) is the event backbone. It is a set of durable Postgres-backed queues
with visibility timeouts, archive and dead-letter handling and at-least-once delivery.
Consumers must be idempotent and deduplicate with their inbox table. Producers write to a
transactional outbox in the same transaction as the business change, and a dispatcher moves
outbox rows onto the queue, so a committed state change is never published twice or silently
lost. A message that exceeds its receive count or visibility window is archived; archive and
dead-letter depth is monitored and alerted. Supabase Queues does not provide an unlimited or
globally ordered log, so event design relies on aggregate versions and inbox dedup rather than
onstream position.

Several concrete subjects are fixed in `workspace.dsl`: the two security-audit subjects, the
`job.tick` signal, `job.run.requested`, `account.holdings.replaced`, the five account lifecycle
subjects (`account.created`, `account.updated`, `account.activated`, `account.deactivated`,
`account.deleted`), `recommendation.generated`, `recommendation.decided` and the portfolio
lifecycle, valuation and snapshot events (`portfolio.created/updated/deleted`,
`valuation.computed`, `snapshot.captured`). The job keys are fixed in the `Job Definition`
component: `catalogue.refresh`, `prices.collect`, `accounts.sync`, `snapshots.eod`,
`recommendations.expire`, `briefs.dispatch`, `projections.reconcile`, `audit.retention`,
`prices.retention`, `snapshots.retention`, `reporting.retention` and `identity.retention`. The
remaining rows are event families: where the DSL names both the publisher and the consumer, both
are fixed; where it only states that a service consumes job commands or run outcomes, the
producer is the command-issuing or job-executing service. Exact queue names are implementation
details. When a family stabilises, record its concrete subjects in the DSL descriptions so this
table and the model stay aligned.

| Event, command or family | Producer | Consumers | Purpose |
| --- | --- | --- | --- |
| `security.access_denied` | API Gateway | Audit Service | Record denied authentication, authorization and administrative access attempts. |
| `security.rate_limited` | API Gateway | Audit Service | Record rate-limit enforcement on public and security-sensitive routes. |
| `job.tick` | Supabase Cron (pg_cron) | Job Orchestration Service | Signal that a job schedule is due; the tick is idempotent per period and repeated ticks in the same period are no-ops. Supabase Cron is the trigger, not the scheduler of record. |
| `job.run.requested` | Job Orchestration Service | Identity, Account, Asset Registry, Market Data, Portfolio, Reporting, Audit | Dispatch a scheduled, manual, retry, cancel or backfill run to the owning service without coupling the scheduler to business logic. Each command carries a logical run key (job + scheduled period), a per-attempt run id and a lease epoch, and never a credential or assertion: the consumer exchanges the command for a short-lived run-scoped assertion over mTLS. Consumers deduplicate on (run key, attempt), so a redelivery is rejected while a bounded retry with a new attempt is accepted. |
| `account.holdings.replaced` | Account Service | Portfolio Service | Replace the holdings projection atomically after a sync; carries accountId, monotonic aggregate version and occurredAt so only newer state is applied. |
| `recommendation.generated` | Portfolio Service | Reporting Service | Make a new recommendation available to brief composition; a newer generation supersedes prior pending recommendations. |
| Identity user, role, session, status-change and deletion events | Identity Service | API Gateway, Audit Service, owning services | Evict or refresh cached principals and roles; record sign-in, sign-out and role changes; cascade user deletion. |
| Registry asset, contract, chain and token events | Asset Registry Service | Account Service, Market Data Service, Portfolio Service | Propagate versioned catalogue and registry changes to projections; consumers apply only a newer aggregate version. |
| Registry-change audit events | Asset Registry Service | Audit Service | Record administrative chain and token changes in the hash-chained compliance store; the registry's own audit table remains an operational record. |
| Account lifecycle, sync outcome, unmapped observation and audit events | Account Service | Portfolio Service, Asset Registry Service, Job Orchestration Service, Audit Service | Mark links to deactivated accounts inactive and excluded from valuation, remove links only on `account.deleted`, reactivate them on `account.activated`, feed the affected-account index, quarantine unmapped tokens, track run outcomes and preserve traceability. Lifecycle events carry the account aggregate version. |
| Price observation, batch, freshness and provider-health events | Market Data Service | Portfolio Service, Audit Service | Maintain the latest-price projection and make freshness and provider state visible; price events carry a per-asset monotonic observation sequence (compared separately from registry aggregate versions), and the projection consumer classifies stale, unpriced and unmapped lines. |
| Portfolio, valuation, snapshot, recommendation, decision and audit events | Portfolio Service | Reporting Service, Audit Service | Feed briefs and record material state changes in the audit trail: `portfolio.created/updated/deleted`, `valuation.computed`, `snapshot.captured`, `recommendation.generated` and `recommendation.decided`. |
| Brief generated, published, delivery-failure and audit events | Reporting Service | Audit Service | Make delivery outcomes traceable without exposing brief content or secrets. |
| Run outcome and heartbeat events | Account, Identity, Asset Registry, Market Data, Portfolio, Reporting, Audit | Job Orchestration Service | Record start, end, affected counts and liveness; detect missed or failed runs. Only the run owner (the job consumer or fan-out use case) publishes the run outcome; per-item use cases publish item-progress events on a distinct item-progress queue that the run owner aggregates. |
| Missed-run and alert events | Job Orchestration Service | Operational Alerting Channel | Raise an alert within 15 minutes of a missed or failed schedule. A Supabase Cron dead-man watchdog also alerts through pg_net if the scheduler itself stops. |
| Manual-job-trigger audit events | Job Orchestration Service | Audit Service | Record administrator-triggered runs in the compliance store as well as the local trigger audit. |
| Privacy export, erasure, export-part, export-ready, erasure-confirmation and retention-applied events | Audit Service, Identity, Account, Asset Registry, Market Data, Portfolio, Reporting | Audit Service and every owning service | Coordinate per-service export parts and erasure or anonymisation under legal hold, and confirm completion. |
| Session and delivery audit events | Identity Service, Reporting Service | Audit Service | Complete the security and reporting audit trail. |

Event design rules:

- Subject names describe business facts in the past tense, not commands, except explicit
  command subjects such as `job.run.requested` and `job.tick`.
- Events carry identifiers and business facts, never credentials, tokens, signed assertions or
  full brief content. A principal assertion is a reusable bearer credential, so it is exchanged
  over mTLS from Job Orchestration and is never placed in a command or event payload.
- State-changing events that update a mutable projection carry an aggregate id, a monotonically
  increasing version and an `occurredAt` timestamp; consumers apply a change only when the version
  is newer than the stored one, so reordering cannot overwrite newer state. This rule is stated
  and modelled for holdings and account lifecycle events (`Holdings Event Versioning Policy`,
  `Account Lifecycle Event Versioning Policy`), for registry events (`Registry Event Versioning
  Policy`, where token registry entries carry their own aggregate id and version) and for price
  events (`Price Event Versioning Policy`, a per-asset observation sequence compared separately
  from registry aggregate versions). Other event families are append-only business facts consumed
  with inbox dedup and carry identifiers and `occurredAt` where they are used for ordering.
- Every event envelope carries the correlation id and trace context originated at the gateway so
  asynchronous flows are traceable end to end.
- Consumers write to an inbox dedup table before applying a side effect. The audit consumer
  deduplicates on event id plus chain partition; the append-only store never relies on retry
  semantics alone.
- Producers write to a transactional outbox in the same transaction as the business change.
- Archive and dead-letter queues are operationally monitored and alerted; a message is not
  silently dropped. pgmq visibility timeouts return an unacknowledged message for redelivery,
  and exhausted messages are archived for inspection rather than discarded.
- Subject names, schemas and versions are documented when a family stabilises. The DSL names
  `security.access_denied`, `security.rate_limited`, `job.tick`, `job.run.requested`,
  `account.holdings.replaced`, the account lifecycle subjects
  (`account.created/updated/activated/deactivated/deleted`), `recommendation.generated`,
  `recommendation.decided`, `portfolio.created/updated/deleted`, `valuation.computed` and
  `snapshot.captured` explicitly; the other rows are families with fixed producers and consumers.

---

## 8. Clean Architecture blueprint

### 8.1 The four layers in every service

The model groups every Rust service's components into exactly four groups. The names are part of
the convention and should not drift.

| Layer | Typical contents | Dependency direction |
| --- | --- | --- |
| **Domain** | Aggregates, entities, value objects, policies, calculators, invariants | Depends on nothing outward |
| **Application** | Use cases, orchestration, ports (traits), transaction boundaries, authorization and idempotency | Depends on Domain and its own ports |
| **Infrastructure** | SQL repositories, provider clients, Supabase Vault client, Supabase Queues publishers/consumers, Supabase Postgres caches, Supabase Storage client | Depends on Application ports and Domain types |
| **Transport** | Axum routers, middleware, extractors, SSE handlers, Supabase Queues consumers, DTO mapping | Depends on Application |

### 8.2 Dependency rule

1. **Domain is pure.** It contains no `sqlx`, `axum`, `reqwest`, pgmq/Supabase Queues client or
   Supabase Vault imports. It can be unit-tested without a runtime.
2. **Application declares ports.** A use case calls a trait such as a repository, registry
   reader, price reader, secret reader, event publisher or clock. It does not know which adapter
   implements it.
3. **Infrastructure implements ports.** Supabase Postgres repositories, OKX/EVM/Solana
   connectors, CoinGecko/CoinPaprika clients, Supabase Vault encryption and secret readers,
   the OpenClaw client, Supabase Queues publishers and Supabase Storage clients are adapters.
4. **Transport maps and delegates.** Handlers validate shape, authenticate, map DTOs and call a
   use case. They do not contain business rules or direct SQL.
5. **Dependencies point inward.** Transport and Infrastructure may depend on Application;
   Application may depend on Domain; Domain depends on nothing. No arrow from Domain to
   Infrastructure is permitted.
6. **Named Transport-to-Infrastructure exception.** The DSL contains direct Transport-to-
   Infrastructure edges for a bounded set of edge-adapter cases: health, readiness and operational
   probes reading a repository or dependency client; Supabase Queues consumers writing their own
   projection or cache; SSE stream-ticket verifiers; and any route guard or handler (public,
   admin, privacy, internal or retry) calling its `Principal Assertion Verifier`. These are
   mechanics with no business rule; every other Transport edge goes through an application use
   case. The exception is named here, in section 3.2 and in the review checklist so the model and
   the rule cannot drift apart.

The DSL diagrams show runtime calls, so an application component may point at an infrastructure
component to show "calls the repository port implemented by this adapter". That is not an import
arrow and does not weaken the rule.

The eight domain services carry a `Principal Assertion Verifier` adapter; the gateway is the
assertion issuer and validates Supabase JWTs instead. Each verifier is an Infrastructure
component that resolves the gateway user-assertion and Job Orchestration System-assertion public
keys from Supabase Vault and caches them with a bounded refresh. Every route guard — public,
admin, privacy and internal — calls its service's verifier before the route runs. The verifier is
the only component that validates assertion signatures; use cases receive an already-verified
principal.

### 8.3 Canonical service layout

```text
service/
  src/
    domain/            # aggregates, value objects, policies, errors
    application/       # use cases, ports, command/query models
    infrastructure/    # sqlx repositories, clients, publishers, caches
    transport/         # Axum routers, middleware, SSE, Supabase Queues consumers, DTOs
  migrations/          # service-owned schema migration set, applied to the service role
  tests/               # unit and integration tests
```

A service may split `domain` and `application` into modules (the Portfolio Service has three
modules: lifecycle/configuration, valuation/snapshots and advisory recommendations), but the
dependency direction remains the same. Migrations are owned per service but applied against the
shared Supabase Postgres project; each migration set is scoped to the owning schema and must not
alter another service's objects.

### 8.4 Application per service

| Service | Domain core | Key application ports | Notable adapters | Transport surfaces |
| --- | --- | --- | --- | --- |
| API Gateway | Authenticated principal, principal status, role policy, rate-limit policy, signed internal principal assertion, first-sighting policy | JWKS reader, principal cache, identity client, rate limiter, assertion signer, stream-ticket signer, Supabase Vault secret client, audit channel, route policy | Supabase Postgres JWKS and principal caches, Supabase Postgres atomic-counter rate limiter, Identity Service client, principal assertion signer, stream-ticket signer, Supabase Vault client, fail-closed Supabase Queues audit channel | Public HTTP router, health, JWT middleware, admin guard, stream-ticket issuance, SSE proxy with ticket verification, identity-event consumer |
| Identity | Internal user aggregate, role assignment, token validation policy, provisioning policy, auth-hook verification policy, admin authorization policy, privacy policy | User repository, Supabase Admin API client, Supabase Vault secret client, principal-cache eviction signal, assertion verifier, event and run-outcome publishers | Supabase Postgres repository, Supabase Auth Admin API client, Supabase Vault client, Supabase Queues publishers | Public/internal REST with Supabase bearer and admin guards, internal ensure-user handler, internal lookup guard, Supabase Auth hook handler, privacy and job consumers, health |
| Account | Account aggregate, holding with mapped/unmapped state, balance normaliser, read-only credential policy, holdings event versioning policy, account lifecycle event versioning policy | Account repository, credential cipher, exchange/wallet connectors, registry snapshot client, event and run-outcome publishers | Supabase Postgres repository, Supabase Vault credential cipher, OKX/EVM/Solana connectors, registry snapshot client, stream-ticket verifier, assertion verifier, Supabase Queues publishers | REST, internal REST with principal guard and operations rebuild, SSE with ticket verifier, job consumer, registry-event consumer, privacy consumer, health |
| Asset Registry | Asset aggregate, asset contract, EVM chain, identity resolver, secret-reference policy, admin-authorization policy, registry event versioning policy | Asset repository, chain repository, lookup client, affected-account projection, event and run-outcome publishers | Supabase Postgres repositories, provider lookup client, Supabase Vault secret reader, assertion verifier, Supabase Queues publishers | Asset query REST, admin REST with admin guard, internal snapshot with principal guard, unmapped consumer, account-event consumer, job and privacy consumers, health |
| Market Data | Price observation with per-asset observation sequence, price freshness, fallback policy, provider status, admin-authorization policy, provider secret-reference policy, price event versioning policy | Price repository, provider clients, last-known-price cache, Supabase Vault secret reader, event and run-outcome publishers | Supabase Postgres repository, CoinGecko/CoinPaprika clients, Supabase Postgres cache, Supabase Vault reader, assertion verifier, Supabase Queues publishers | Market REST, admin REST with admin guard, internal price with principal guard, job consumer, registry and privacy consumers, health |
| Portfolio | Portfolio aggregate, valuation calculator, valuation line status, snapshot, recommendation, snapshot immutability policy, recommendation supersession policy, portfolio configuration policy, admin authorization policy | Portfolio repository, holdings and price/registry projections, ownership and snapshot clients, advisory/batch worker, Supabase Vault secret client, event and run-outcome publishers | Supabase Postgres repository, projection readers, Account/Registry/Market clients, isolated Tokio batch pool, assertion verifier, Supabase Vault client, Supabase Queues publishers | Portfolio/valuation/recommendation/query REST, internal brief and rebuild with principal guard, job, account-event, price-and-registry-event and privacy consumers, health |
| Reporting | Brief, brief composer, delivery record, idempotent delivery policy, delivery retry policy, brief payload contract, admin authorization policy | Brief repository, portfolio client, OpenClaw client, secret reader, event and run-outcome publishers; retry delivery use case for authorisation and bounds | Supabase Postgres repository, Portfolio client, OpenClaw client, Supabase Vault secret reader (OpenClaw token only), assertion verifier, Supabase Queues publishers | Reporting REST, internal brief with principal guard, retry, job, domain-event and privacy consumers, health |
| Job Orchestration | Job definition, job schedule, job run with logical run key, attempt id, lease epoch and fencing, retry policy, single-flight policy, run lease and idempotency policy, admin authorization policy | Job definition repository, run repository, advisory lock, command publisher, System assertion signer, alert webhook client, Supabase Vault secret client | Supabase Postgres repositories, Supabase Postgres advisory lock, Supabase Queues publisher, assertion signer and verifier, alerting webhook client, Supabase Vault secret client | Admin REST with a first-class admin guard for listing, scheduling, triggering, cancel and backfill, scheduler tick loop, missed-run watchdog, stale-run expiry, outcome consumer, run-assertion exchange, internal health, public health |
| Audit | Audit event, hash-chain policy, chain-writer policy, export request, retention policy, PII pseudonymisation policy, admin authorization policy | Append-only repository, privileged retention repository, Supabase Storage client, Identity lookup client, privacy coordinator, event and run-outcome publishers, Supabase Vault secret reader | Insert-only Supabase Postgres repository, privileged retention role for archival and partition drop only, per-partition advisory-lock writer lease, Supabase Storage adapter, Identity client, assertion verifier, Supabase Queues coordinator and publishers, Supabase Vault reader | Admin/privacy REST with admin and privacy guards, internal guard, audit-event consumer, export-part consumer, job consumer, health |

### 8.5 How the DSL encodes the pattern

- Every Rust service container has four groups named exactly `Transport`, `Application`,
  `Domain` and `Infrastructure`.
- Component identifiers are prefixed by service (`gateway…`, `identity…`, `account…`, `registry…`,
  `market…`, `portfolio…`, `reporting…`, `job…`, `audit…`) so the layer remains obvious even
  when a component appears in a cross-service dynamic view. Display names stay descriptive and
  are not prefixed.
- Relationship technologies state the mechanism: `in-process`, `repository port`, `mTLS REST
  client`, `Supabase Queues publisher`, `Supabase Queues consumer`, `Supabase Postgres client`,
  `Supabase Vault (pgsodium)`, `Supabase Storage client`, `advisory lock`, `SQL over TLS via
  Supavisor`.
- The `styles` block colours Domain green, Application blue, Infrastructure purple and
  Transport orange, so a misplaced component is visible immediately.
- A component in the wrong group is a modelling defect, even if the code still compiles.

---

## 9. Supabase platform mapping and shared responsibility

The platform is **Supabase-only** for managed infrastructure. Supabase provides identity,
relational storage, queues, object storage, secrets, scheduling and observability. It does
**not** host the Rust services or the React SPA: those run on the `Container Host` (section 10).
All service-to-Supabase traffic leaves the container host over TLS; Postgres traffic uses the
Supavisor pooler.

### 9.1 Service mapping

| Supabase service | Replaces | What the product still owns | Plan limits and operating notes |
| --- | --- | --- | --- |
| **Supabase Auth (GoTrue)** | A self-hosted OIDC identity provider | Sign-in UX, role projection through the Custom Access Token Hook (`app_metadata.role = "admin"`), the internal ensure-user mapping keyed by `auth.users.id`, session-event recording from auth hooks, admin-claim re-checks in every service, disabled-user denial, privacy deletion. | Monthly active users, Auth request rate limits, JWT expiry and refresh-token settings, hook timeout and URL configuration. Auth is on the sign-in and refresh path, so its availability and rate limits are product-visible. Supabase holds password hashes and MFA state; the product never does. |
| **Supabase Postgres** | Eight per-service database servers (or databases) and a separate cache tier | Schema design, migrations, repository queries, cross-service projection design, transaction boundaries, advisory-lock usage, per-service roles and grants, connection-pool sizing, RLS policy decisions. | Database size, compute add-on, connections through Supavisor, statement timeouts, storage. One project is the operational unit for backups, PITR and upgrades, so a project-level event is a shared blast radius. Services use transaction-mode pooling and transaction-scoped advisory locks; session-level lock usage must be verified against the pooler. RLS is not used by services (section 9.3). |
| **Supabase Queues (pgmq)** | A dedicated event-streaming broker | Queue and subject naming, outbox dispatch, inbox dedup, consumer idempotency, visibility-timeout tuning, archive/dead-letter monitoring, per-run-key dedup, event versioning. | Throughput is bounded by Postgres write capacity and queue table maintenance. There are no broker-style partitioned consumer groups or unbounded replay. Archive retention consumes database storage. Section 15 tracks throughput validation. |
| **Supabase Storage** | S3-compatible audit object storage | Bucket privacy, signed-URL lifetime, export assembly, archive naming and retention, access auditing, ensuring no public bucket exists. | Storage quota and egress. Only Audit Service reads or writes archives; URLs are signed and access-controlled, never public. |
| **Supabase Vault (pgsodium)** | An external secrets manager and KMS | Secret classification, secret-reference naming, least-privilege role access, rotation runbooks, break-glass procedure, ensuring plaintext never reaches logs, events, briefs or API responses. | Supabase Vault is read through the owning service's Postgres role; encrypt/decrypt happens inside the database. Key rotation and per-subject crypto-shredding must be exercised. The database project's security posture is now the secrets posture, so plan-level access control and PITR matter. |
| **Supabase Cron (pg_cron)** | A platform scheduler and independent dead-man switch | The real scheduler semantics: due-ness, logical run keys, single-flight, leases, fencing epochs, retries, outcomes and alert thresholds all remain in Job Orchestration. | Cron frequency and statement-duration limits apply. Cron only enqueues `job.tick`, runs retention triggers and hosts the dead-man watchdog; it is not the system of record for job state. |
| **Supabase Logs & Metrics** | A separate observability platform | Structured log schema, correlation and trace context, health/readiness endpoints, metric selection, alert thresholds and routing to the Operational Alerting Channel. | Log retention and query window follow the Supabase plan. Long-term retention, dashboards and SLO tooling may need an external sink. Section 15 tracks retention. |

### 9.2 Backups, PITR and recovery

Supabase performs encrypted backups and offers point-in-time recovery as a plan add-on. The
product must:

- confirm the project's backup and PITR configuration and retention window satisfy BRD-012
  NFR-AVL-04 (RPO ≤ 24 hours) and the agreed RTO (NFR-AVL-05);
- run restore drills and record the evidence, because a backup that has never been restored is
  not a verified control;
- verify that Supabase Storage archive buckets and Supabase Vault key material are covered by
  the recovery plan, not only the relational database;
- document that recovery is a project-level operation, consistent with logical
  database-per-service isolation.

Backup and restore verification is a **shared** obligation: Supabase operates the mechanism, the
product owns the configuration, the drills and the evidence. This is still an open item in
section 15.

### 9.3 Connection pooling, schemas and authorization

- Services connect through the Supavisor pooler with a per-service role. A service role has
  `USAGE` and object privileges only on its own schema; there are no cross-schema grants.
  Migrations run under the owning role and are scoped to that schema.
- **Row Level Security is not the service isolation mechanism.** Services authenticate to
  Postgres as themselves, not as an end user, so RLS would not distinguish one product user
  from another. Ownership and admin rules are enforced in application use cases against the
  signed internal principal assertion, and additionally by owner-scoped queries. RLS remains
  available if a future direct-client access path is introduced, but it must not be treated as
  a substitute for the application authorization matrix.
- Because all services share one Postgres project, a connection-pool and role-per-service
  design is mandatory. Pool sizes are budgeted per service; Portfolio's batch worker uses its
  own pool. Advisory locks are transaction-scoped where they are held inside a transaction, and
  the per-partition audit writer and per-account sync locks must be verified against
  transaction-mode pooling.
- The API Gateway's schema holds only caches and counters. Losing that schema degrades
  performance and forces re-resolution through Identity; it cannot lose a business record.

### 9.4 Authentication boundary and hooks

- Supabase Auth is the only external system the browser may call directly, and only for
  sign-up, sign-in, token refresh, MFA and sign-out.
- Services validate Supabase JWTs against the Supabase Auth JWKS endpoint. The gateway caches
  the JWKS in memory and Supabase Postgres with stale-while-revalidate and a bounded grace
  period.
- The Custom Access Token Hook projects `app_metadata.role = "admin"` into issued tokens.
  Services still re-check the claim, and never treat portfolio ownership as administrative
  rights.
- Auth Hooks/webhooks deliver sign-up, sign-in, sign-out and user-update events to Identity.
  Identity verifies the standard-webhooks signature, enforces the replay window and applies
  idempotency before recording a session event or applying a status change.
- Administrative actions such as disabling a user, updating `app_metadata` and deleting an auth
  user go through the Supabase Admin API from Identity, never from the browser.

### 9.5 Shared responsibility summary

| Area | Supabase owns | The product owns |
| --- | --- | --- |
| Managed compute, patching, HA of the managed services | Yes | Choosing the plan and verifying the SLA |
| Auth engine, password hashing, MFA factors, JWT signing | Yes | Role projection, admin gating, hook verification, user lifecycle policy |
| Postgres engine, backups, PITR mechanics | Yes | Schema/role isolation, migrations, queries, RLS decision, connection budgets, backup configuration and restore drills |
| Queue engine (pgmq) | Yes | Outbox/inbox, idempotency, visibility timeouts, archive and dead-letter monitoring, throughput validation |
| Storage engine and signing | Yes | Bucket privacy, signed-URL lifetime, archive retention and access review |
| Supabase Vault encryption primitives | Yes | Secret classification, access via roles, rotation, crypto-shredding, no plaintext leakage |
| Cron execution | Yes | The real job semantics in Job Orchestration, tick idempotency, watchdog thresholds |
| Log/metrics collection | Yes | Structured logging, correlation, health, dashboards, alert thresholds, retention choice |
| Application security | No | Authentication flow, authorization matrix, ownership checks, input validation, rate limiting, secret handling in code |
| Rust services and SPA runtime | No | Build, deploy, scaling, health, rolling releases on the container host |

Supabase is a processor and infrastructure provider, not a substitute for the product's own
authorization, data-minimisation and audit obligations.

---

## 10. Deployment topology

### 10.1 Production environment

The only modelled environment is `Production`, shown by the `ProductionDeployment` view. It
contains one deployment node, `Container Host` (Fly.io / Render / Cloud Run), with managed TLS,
health checks and rolling deploys. It hosts the SPA static assets and every Rust service as a
separate process group:

```text
Internet
   │ HTTPS                         │ HTTPS
   ▼                               ▼
Supabase Auth               Container Host (Fly.io / Render / Cloud Run)
(browser sign-in/refresh)     ├─ React Web Client (SPA) static assets + edge TLS
                              ├─ API Gateway (BFF)
                              ├─ Identity Service
                              ├─ Account Service
                              ├─ Asset Registry Service
                              ├─ Market Data Service
                              ├─ Portfolio Service (interactive + batch worker)
                              ├─ Reporting Service
                              ├─ Job Orchestration Service
                              └─ Audit Service
                                   │ TLS on the host private network
                                   ▼
                    Supabase Postgres / Queues / Storage / Supabase Vault / Logs & Metrics
                    Supabase Cron → pgmq + pg_net watchdog
```

There are no database, queue, cache or storage deployment nodes. Those are managed Supabase
services and remain external to the deployment view.

### 10.2 Service process groups

| Process group | Container | Scaling and isolation |
| --- | --- | --- |
| SPA static assets | React Web Client (SPA) | Static shell; managed TLS at the host edge |
| API Gateway | API Gateway (BFF) | Stateless processes, horizontal scaling |
| Identity | Identity Service | Horizontal scaling |
| Account | Account Service | Horizontal scaling; sync work bounded per process |
| Asset Registry | Asset Registry Service | Horizontal scaling |
| Market Data | Market Data Service | Horizontal scaling; provider limits respected |
| Portfolio interactive | Portfolio Service | Horizontal scaling, own connection pool |
| Portfolio batch worker | Portfolio Service | Separate process group with its own connection pool, bounded concurrency, per-owner fairness and backpressure for sharded EOD snapshots and recommendation expiry |
| Reporting | Reporting Service | Processes for brief composition and dispatch |
| Job Orchestration | Job Orchestration Service | Replicas coordinate through Supabase Postgres advisory locks so only one runs a given job; run leases have heartbeat-renewed TTLs and stale runs are expired |
| Audit | Audit Service | Audit and privacy processes; replicas coordinate through a per-partition advisory-lock writer lease so only one writer extends a chain |

### 10.3 Supabase managed services

| Managed service | Deployment characteristic |
| --- | --- |
| Supabase Postgres | One managed project with one schema and role per service; Supavisor pooler; encrypted storage; backup/PITR per plan |
| Supabase Queues (pgmq) | Queue tables inside Supabase Postgres with visibility timeouts, archive and dead-letter handling |
| Supabase Storage | Private buckets for audit export and retention archives with signed, access-controlled URLs |
| Supabase Vault | pgsodium-backed secrets inside Supabase Postgres, read by the owning service role |
| Supabase Auth | Managed GoTrue with JWKS, Custom Access Token Hook, Auth Hooks/webhooks and the Admin API |
| Supabase Cron | pg_cron schedules that enqueue `job.tick`, run retention triggers and host the dead-man watchdog through pg_net |
| Supabase Logs & Metrics | Platform log drain, metrics and alerts |

Every service schema defines a transactional outbox and an inbox dedup table. Market Data, Job
and Audit in particular must never rely on retry semantics alone: all three consume
at-least-once events, and duplicate appends would be especially harmful to the insert-only audit
store. The audit inbox deduplicates on event id plus chain partition.

Blast radius: because all services share one managed Supabase Postgres project, a project-level
outage, storage exhaustion or migration error affects every service. Per-service roles, schema
ownership, connection quotas and no cross-schema grants contain the logical blast radius, but
they do not give independent restores or independent engine upgrades. This is the deliberate
cost of logical database-per-service (section 3.3, section 13).

### 10.4 Observability and alerting

- **Supabase Logs & Metrics** collects structured logs, metrics and traces from every service
  through the log drain, with correlation and trace context. The product chooses the plan and
  therefore the retention window.
- **Job Orchestration** records run outcomes, runs missed-run detection from a fixed-interval
  watchdog loop independent of health probes, and raises alerts within 15 minutes through its
  `Alerting Webhook Client`.
- **Supabase Cron** hosts the independent dead-man watchdog. It checks scheduler heartbeats and
  last-success state and calls the `Operational Alerting Channel` through pg_net when they stop.
  This removes the scheduler as the sole detector of its own absence without adding another
  runtime service.
- Every service exposes health and readiness endpoints that report its repository and
  critical-dependency readiness.

### 10.5 Not in the deployment view

- Supabase Auth, Postgres, Queues, Storage, Supabase Vault, Cron and Logs & Metrics (external systems).
- All third-party providers (OKX, EVM RPC, Solana RPC, price providers, OpenClaw, Notion,
  alerting channel).
- CI/CD, registry, backup tooling, disaster recovery and development/staging environments.
- Any service mesh, sidecar proxy or per-service mTLS certificate authority. TLS on the
  container host's private network plus signed internal principal assertions provide the
  transport and authorization controls; a service mesh is not part of the Supabase-only stack
  (section 15).

---

## 11. Cross-cutting concerns

### 11.1 Authentication and authorization

**Authentication path.**

1. The SPA signs up or signs in with Supabase Auth using email/password, magic link, OAuth or
   MFA under PKCE, through `@supabase/supabase-js`.
2. The SPA keeps access and refresh tokens in memory only and refreshes sessions directly
   against Supabase Auth. It never persists tokens in `localStorage`.
3. The SPA calls only the API Gateway for application data. Supabase Auth endpoints are the
   only direct browser-to-Supabase exception.
4. The gateway validates signature, issuer, audience and expiry locally against the Supabase
   Auth JWKS, cached in memory and Supabase Postgres with a small documented clock-skew
   allowance and a bounded stale-key grace period.
5. The gateway looks up the principal in its Supabase Postgres principal cache by Supabase user
   UUID. The cache holds the stable owner UUID, status and roles with a short TTL.
6. **On a first sighting** there is no internal user yet, so the gateway calls Identity's
   internal ensure-user endpoint over mTLS with the Supabase access token and the gateway's
   service identity. Identity validates the token and idempotently upserts the internal user
   keyed by `auth.users.id`; concurrent requests for the same UUID return the same mapping.
   Identity returns the owner UUID, status and roles.
7. **On a later cache miss** the gateway presents its signed internal principal assertion
   carrying the Supabase user UUID and owner UUID to Identity's internal lookup endpoint.
8. Identity projects Portfolio Owner, Admin and System roles from the `app_metadata.role` claim
   issued through the Custom Access Token Hook, records verified auth-hook session events, and
   denies disabled users even when the Supabase token is valid.
9. The gateway rejects disabled principals and enforces a deny-by-default route policy; the
   admin guard applies only where the admin claim is required.
10. On every routed request, public or internal, the gateway mints a short-lived signed
    principal assertion carrying the owner UUID, roles, status and correlation id. Each of the
    eight domain services has a `Principal Assertion Verifier` that resolves the gateway
    user-assertion and System-assertion public keys from Supabase Vault, caches them, validates
    the signature, issuer, expiry, audience and correlation id, and enforces owner scoping on
    every owner-scoped read and write. The gateway is the assertion issuer and validates
    Supabase JWTs instead.
11. User-originated service-to-service hops forward the inbound assertion unchanged. Scheduled
    and recovery hops do not carry an assertion in the event: the owning service exchanges the
    run command for a short-lived, run-scoped System assertion at Job Orchestration over mTLS,
    bound to the run key, lease epoch, target audience and permitted owner set. The exchange
    confirms the mTLS caller is the run's owning service, so a different service cannot obtain
    an assertion for that run. Every service re-checks the admin role inside its own admin use
    cases, and an admin guard is present at the edge of every admin surface.

**Authorization rules.**

- Deny by default: a capability not explicitly granted to a role is denied.
- A Portfolio Owner may act only on resources they own. Every service checks ownership on read
  and write against the signed assertion; another user's resource returns a non-disclosing
  not-found or forbidden response.
- Internal endpoints require a valid signed assertion and run on the host's private network with
  TLS; they are not exposed through the public ingress. There is an endpoint-by-endpoint
  internal authorization matrix in the owning service's code and tests.
- Admin capability comes from the Supabase `app_metadata.role` admin claim, not from portfolio
  ownership, and is re-checked in each owning service's admin use case rather than only at the
  gateway. Identity, Asset Registry, Market Data, Portfolio, Job Orchestration and Audit each
  expose admin routes behind their own admin guard and admin-authorization policy; Audit privacy
  requests additionally use an owner-scoped privacy guard, and Identity public routes use the
  Supabase JWT bearer guard.
- A System-role assertion is not unscoped. It is bound to the run key, the lease epoch, the
  owning service/audience and, where applicable, the permitted owner set, and the owning
  service's internal authorization matrix states endpoint by endpoint what a System caller may
  request. An assertion bearing any other run key, audience or epoch is rejected.
- System jobs act on behalf of no interactive user and must never place one user's data in
  another user's response.
- Token and credential values are never written to logs.

### 11.2 Security and credential handling

| Concern | Control in the architecture |
| --- | --- |
| Exchange credentials | Supabase Vault (pgsodium) envelope encryption through a per-credential DEK stored in Supabase Vault; versioned ciphertext; re-encrypt on edit; masked in the UI; never returned, logged or placed in events |
| Wallet connections | Public address only; private keys and seed phrases are rejected by the read-only credential policy |
| Exchange permissions | Read-only API keys; no withdrawal, transfer or trading capability in the MVP |
| Internal traffic | TLS on the container host's private network plus a signed internal assertion validated by every route guard's `Principal Assertion Verifier` against Supabase Vault-resolved and cached public keys. User hops forward the assertion unchanged. Scheduled and recovery hops obtain a run-scoped System assertion from the Job Orchestration exchange over mTLS; it is never placed in an event payload. A service mesh, sidecar proxy or per-service certificate authority is not part of the stack |
| External traffic | TLS for Supabase Auth, Supabase Postgres (via Supavisor), Supabase Queues, Supabase Storage, Supabase Vault, OKX, RPC providers, price providers, OpenClaw and Notion; managed TLS at the container host edge |
| RPC and provider secrets | RPC URLs, provider API keys and the OpenClaw token are Supabase Vault secret references, resolved at runtime, masked in API responses and never logged; Notion credentials never enter the platform and stay with the OpenClaw Agent |
| Abuse control | Per-user and per-IP rate limits at the gateway, implemented with the Postgres atomic-counter rate limiter; stricter limits on admin and security-sensitive routes; manual job triggers rate-limited |
| Secret supply | Supabase Vault for runtime secret references and Postgres credentials; Supabase project secrets for deployment-time configuration such as signing-key references and service endpoints; no secrets in source control |
| Encryption at rest | Supabase encrypts managed Postgres, Storage and backups; Supabase Vault protects credential material inside the database. The product owns secret classification, access roles and rotation |
| Audit | Access denied, rate limited, credential changes, registry changes, admin configuration, recommendation decisions, job triggers, sync outcomes and deliveries are recorded; registry-change and manual-trigger records are published to the hash-chained Audit Service. Gateway security-audit events are published synchronously and acknowledged by Supabase Queues before the request completes, so the gateway fails closed rather than returning a response whose audit record was not durably accepted. pgmq visibility timeouts and archive provide dead-letter handling |
| Data at rest | Supabase PITR and encrypted backups, verified through restore drills; Audit export and retention archives use private Supabase Storage buckets with signed URLs. Backup and restore evidence is a shared-responsibility item in section 9.2 and section 15 |

### 11.3 Observability

Observability is split into two distinct concerns:

- **Audit** is a business record. Audit Service stores hash-chained, insert-only events and is
  queryable by authorised administrators. It is not a debugging tool.
- **Operational telemetry** is logs, metrics, traces and alerts. The model requires correlation
  and trace context, health and readiness on every service, job run records, heartbeats and
  missed-run detection. Supabase Logs & Metrics collects the telemetry, and the `Operational
  Alerting Channel` receives alerts routed by Job Orchestration and by Supabase Cron.

Job Orchestration is the operational heartbeat of scheduled work: it records run outcomes, runs
missed-run detection from a fixed-interval watchdog loop independent of health probes or
external polling, and raises alerts within 15 minutes. Health and readiness handlers are
side-effect free and only report state. Because the scheduler cannot detect its own absence, the
Supabase Cron dead-man watchdog watches the same heartbeat and last-success signals from outside
and calls the alerting channel through pg_net. Market Data Service exposes provider health and
freshness so stale prices are visible rather than silent.

Log and metric retention follows the Supabase plan. If the chosen plan does not retain logs long
enough for incident review, the product must add an external log sink; this is recorded in
section 15.

### 11.4 Resilience

- **Fault isolation per account.** A bulk sync runs each account independently; one failure does
  not abort siblings, and the SSE stream reports per-account outcomes. Bulk size, per-request
  duration and worker concurrency are bounded. Per-account outcomes are recorded in sync runs, so
  an interrupted bulk sync can be re-run account by account without duplicate side effects; it is
  not resumable mid-run, and SSE authenticates with a short-lived stream ticket rather than a URL
  token.
- **Internal call resilience.** Every synchronous internal edge (gateway to services, Portfolio
  to Account/Registry/Market, Reporting to Portfolio, Audit to Identity, run-assertion exchange)
  has a timeout, bounded retry and circuit breaker with a documented fallback or fail-closed
  behaviour.
- **Provider fallback and rotation.** Price collection tries the configured primary, then the
  approved fallback, then the last known stored price under a shared provider budget. Each chain
  has primary and fallback RPC endpoints with health, rotation and backoff. An asset with no
  usable price, or older than the maximum stale age, is marked unpriced rather than valued at a
  guess.
- **Supabase Vault degradation.** Supabase Vault is on the credential read/write path; clients
  use bounded retry and circuit breakers, cache unwrapped DEKs in memory only for the shortest
  permitted TTL, and fail closed with alerts and a documented break-glass path for syncs.
- **Supabase Auth degradation.** Supabase Auth is expected to be highly available. JWKS uses
  stale-while-revalidate with a bounded grace period; existing sessions continue until token
  expiry, new sign-ins fail clearly, and the gateway serves cached principals until TTL expiry
  if Identity is unavailable.
- **Supabase Postgres and Supabase Queues degradation.** A Postgres outage stops writes across
  services and therefore stops queue consumption as well; outboxes retain work until the
  database returns, so recovery is replay from the outbox rather than message loss. Consumers
  must treat transient Postgres errors as retryable and must not acknowledge a queue message
  before the business transaction commits.
- **Retries and backoff.** External clients retry with bounded backoff; RPC reads are batched.
- **Messaging durability.** Supabase Queues visibility timeouts return an unacknowledged message
  for redelivery, and exhausted messages are archived. Producers use a transactional outbox so a
  committed change is not lost when the queue publish fails. The gateway's security-audit channel
  is a bounded in-memory channel with synchronous publish-and-acknowledge: a request is not
  completed until Supabase Queues acknowledges its audit record, and the gateway fails closed if
  the publish cannot be acknowledged within the bounded timeout. Gateway processes hold no
  durable local state.
- **Scheduler safety, retries and liveness.** Durable UTC schedules, Supabase Postgres advisory
  locks, single-flight policy and leases prevent overlapping runs. Each run carries a logical run
  key (job + scheduled period) and a per-attempt run id; consumers deduplicate on (run key,
  attempt), so redelivery of an attempt is rejected while a bounded retry with a new attempt is
  accepted. Each attempt also carries a monotonically increasing lease epoch used as a fencing
  token: consumers compare it immediately before each write and reject a command or outcome
  bearing an older epoch, so a heartbeat-less executor and its replacement cannot both write. The
  lease has a bounded TTL renewed by heartbeats; a run whose lease expires without a heartbeat is
  marked failed, its epoch is superseded, its single-flight is released and a bounded retry is
  scheduled. An admin cancel marks the run cancelled, releases single-flight and asks consumers
  to stop between shards, and a bounded backfill covers a closed period window. Exhausted retries
  raise an alert. A fixed-interval watchdog loop detects missed runs within 15 minutes without
  depending on health polling. The Supabase Cron dead-man watchdog alerts if the scheduler itself
  is down or partitioned.
- **Scheduled fan-out.** `accounts.sync`, `snapshots.eod`, `recommendations.expire` and
  `projections.reconcile` run through explicit all-owner fan-out use cases that enumerate or shard
  active accounts and portfolios, apply per-owner fairness and backpressure, reuse an isolated
  worker pool and enforce idempotency on (run key, attempt). Per-item failure is isolated: a
  failed account or portfolio is recorded as failed and the remaining shards continue, producing
  a partial-failure run outcome, so one large owner or one bad portfolio cannot starve or abort
  the batch. The retention jobs (`snapshots.retention`, `prices.retention`, `reporting.retention`,
  `identity.retention`, `audit.retention`) run as direct per-run commands without a fan-out use
  case. Only the run owner publishes the run outcome and heartbeat; per-item use cases publish
  item-progress events that the run owner aggregates.
- **Safe failure.** A failed sync, provider or delivery does not corrupt existing data; partial
  writes are avoided or explicitly recorded.
- **Read consistency.** All reads use Supabase Postgres, which gives read-your-writes for the
  project. Valuation, snapshot-capture and recommendation computation read the same store, so
  stale read state cannot be frozen into an immutable snapshot. Optional Supabase read replicas
  are only a future scaling option (section 15).
- **Capacity and isolation.** The container host scales process groups horizontally; the
  Portfolio batch worker has its own connection pool; worker pools are bounded; per-owner batch
  fairness, backpressure and sharded EOD batches keep load within the schedule interval.
- **Shared-infrastructure dependence.** Supabase Postgres, Queues and Storage are single managed
  dependencies. Loss of Postgres stops writes and event flow while outboxes retain events for
  replay and reconciliation. Loss of Supabase Auth blocks new sign-ins and refreshes; cached
  sessions and the cached JWKS carry existing traffic for a bounded grace period.

### 11.5 Data consistency

- Each service commits locally. There is no distributed transaction and no cross-service join.
- A business change and its outgoing event commit together through a **transactional outbox**.
- Consumers apply events **at least once** and deduplicate through an **inbox table**.
- Derived state is explicit: Account Service keeps a registry projection; Asset Registry keeps
  an affected-account projection; Portfolio Service keeps holdings and price/registry
  projections. Each projection is rebuildable from events or from an internal snapshot endpoint
  on cold start, schema change, or event-retention gap, through a dedicated rebuild use case and
  snapshot client with timeout, circuit breaker and a degraded read-only mode. A rebuild or a
  backfill is a privileged operation triggered by an admin-authorized command or a scheduled run
  issued by Job Orchestration; the receiving service re-checks the asserted admin role or the
  run-scoped System assertion before it starts, so an unauthenticated caller cannot force a
  rebuild.
- Account links to deactivated accounts are kept, marked inactive and excluded from valuation and
  recommendations; they are reactivated on `account.activated` and removed only on
  `account.deleted`, so user configuration is never lost on a temporary deactivation. When an EVM
  chain is deactivated, Account marks that chain's holdings stale and excludes the chain from
  subsequent syncs, and Portfolio classifies those lines stale and excludes them from the valued
  total; the registry record and its token entries are retained, so reactivating the chain
  restores valuation without a re-import.
- Holdings events carry `accountId`, a monotonically increasing aggregate version and
  `occurredAt`; account lifecycle events carry the account aggregate version too. Price events
  carry a per-asset monotonically increasing observation sequence so price freshness and
  registry-derived state are ordered independently and never compared across families. Portfolio
  applies an event only when its version or sequence is newer than the stored one, and consumers
  are partitioned by account key. Inbox dedup prevents duplicates, not reordering. A scheduled
  `projections.reconcile` run compares stored projection versions with Account's internal holdings
  API and reports divergence in the run outcome, which Job Orchestration alerts on.
- Portfolio configuration enforces three invariants before persistence: a unique portfolio name
  per owner, exactly one default portfolio per owner, and target weights summing to 100% within
  decimal tolerance. The repository backs the first two with a unique `(owner, name)` constraint
  and a partial unique index on the default portfolio.
- Valuation, drift, snapshot capture and recommendation generation for a portfolio commit in one
  service and one transaction against its own projections.
- Snapshots are immutable and store the exact inputs, per-line status and calculated values so
  they remain reproducible after accounts or prices change. The rule is **first write wins**: one
  snapshot per portfolio, date and type, and a duplicate capture is rejected and reported rather
  than overwriting the original. The one exception is retry idempotency: a redelivery of the same
  (run key, attempt) returns the already-written snapshot instead of failing, so a queue
  redelivery after a committed write cannot produce a second record or a spurious error. A
  different attempt or a manual re-run for an existing key is still rejected, and an EOD re-run
  does not mutate an existing snapshot. This deliberately deviates from BRD-007
  FR-VAL-09/AC-VAL-05 and BRD-011 BR-JOB-03/AC-JOB-04, which require a repeat to update the
  record; the conflict is recorded in section 15 and must be resolved with product before those
  Must requirements are called implemented.
- Recommendations are versioned; a new generation supersedes prior pending recommendations while
  preserving history links. Each recommendation stores the recorded inputs needed to reproduce
  it: holdings projection version, price observation ids, target and guardrail versions and the
  calculation timestamp.
- The audit store is append-only. Each partition is extended under a per-partition Supabase
  Postgres advisory-lock writer lease, so only one replica appends to a given chain at a time and
  concurrent consumers cannot fork it. Erasure is implemented by **crypto-shredding only**:
  personal data is pseudonymised on the ingestion path before an audit record is chained, and a
  per-subject key held outside the chain is destroyed to make any residual encrypted value
  unreadable. Chained rows are never `UPDATE`d or rewritten after ingestion, so the hash chain
  cannot be invalidated by an erasure. Retention is archival or partition drop, never row
  `UPDATE`/`DELETE` on the append-only role.
- The system is eventually consistent across services. Freshness is part of the UI and brief
  contract: price timestamp, last sync time, and a per-line valuation status of priced, stale,
  unpriced or unmapped. Unpriced and unmapped lines are labelled and excluded from the valued
  total rather than valued at a guess.

### 11.6 Idempotency

| Operation | Mechanism | Guarantee |
| --- | --- | --- |
| Account holdings sync | Atomic replacement of the account's holdings set per run | Repeating a sync does not create duplicate holdings |
| Manual or EOD snapshot | Unique key of portfolio, snapshot date and type; first write wins; a redelivery of the same (run key, attempt) returns the existing snapshot | One snapshot per key; a redelivery of the committed attempt is idempotent, while a repeat with a different attempt or run is rejected and reported, never an overwrite. This deviates from BRD-007 FR-VAL-09 and BRD-011 BR-JOB-03 (section 15). |
| Brief delivery | Idempotency key of owner, portfolio and period; delivery log and external reference | Retry does not create a duplicate brief for the period |
| Job execution | Logical run key (job + scheduled period, or job + manual/backfill occurrence) plus a per-attempt run id, a monotonically increasing lease epoch used as a fencing token, heartbeat-renewed TTL lease, Supabase Postgres advisory lock and single-flight policy | Only one run of a job executes at a time; redelivery of the same attempt is rejected while a bounded retry with a new attempt is accepted; a write bearing an older lease epoch is rejected |
| Stale run recovery | Lease TTL with heartbeat renewal; the watchdog marks a heartbeat-less attempt failed, supersedes its lease epoch, releases single-flight and schedules a bounded retry | A crashed executor cannot block the next scheduled run indefinitely, and its late write is fenced out by the newer epoch |
| Cancel and backfill | Admin-guarded cancel command with per-shard cancellation checks; bounded backfill under a distinct run key | A cancelled run stops between shards and a backfill cannot overlap an active run |
| Event consumption | Inbox dedup table per service; audit dedups on event id plus chain partition | Redelivery does not repeat a side effect |
| Account holdings and lifecycle projection | Aggregate version in the event; apply only if newer | Reordering cannot overwrite newer holdings or link state |
| Manual job trigger | Admin role, rate limit, audit, single-flight | No operational abuse and no concurrent duplicate run |
| Registry import | Review-before-activation; no silent overwrite | A candidate cannot become active or replace a mapping without admin action |
| Price collection | Append-only, timestamped observations | A repeat collection is a new observation, not a duplicate of a past fact |

Identifiers and keys are generated by the owning service. Clients cannot choose an owner UUID or
an internal identifier.

---

## 12. BRD traceability matrix

| BRD | Business area | Primary containers and services | Key model evidence |
| --- | --- | --- | --- |
| **BRD-001** | Product vision, scope and success | React Web Client, API Gateway, Account Service, Market Data Service, Portfolio Service, Reporting Service | Read-only connectors; USD valuation; drift; recommendations; scheduled briefs; no execution or custody; Supabase-managed authentication and data |
| **BRD-002** | Users, roles and permissions | API Gateway, Identity Service, all domain services, Audit Service, Supabase Auth | `JWT Authentication Middleware`, `Admin Role Guard Middleware`, `Role Policy`, `Signed Principal Assertion`, `Principal Assertion Verifier`, `Internal Principal Guard`, `Internal User Aggregate`, `Role Assignment`, `Set User Status Use Case`, `Enforce Admin Role Use Case`, `Admin Authorization Policy` (identity, registry, market, portfolio, job and audit); Supabase `app_metadata.role` admin claim; ownership checks in every service |
| **BRD-003** | Authentication and session management | React Web Client, API Gateway, Identity Service, Supabase Auth, Supabase Postgres (gateway cache) | Supabase Auth with PKCE; JWKS cached in memory and Supabase Postgres; principal cache and identity-event eviction; ensure-user on first sighting; `Ensure User Use Case`; `Project Roles Use Case`; `Record Session Event Use Case`; `Revoke Sessions Use Case`; `Supabase Auth Hook Handler`; `Auth Hook Verification Policy`; Custom Access Token Hook; session-cookie and disabled-user propagation deviations recorded in section 15 |
| **BRD-004** | Account management | Account Service, `account` schema in Supabase Postgres, Supabase Vault, OKX Exchange API, EVM JSON-RPC Providers, Solana RPC Provider | `Account Aggregate`, `Holding` with mapped/unmapped state (price-derived priced/stale/unpriced status belongs to Portfolio's `Valuation Line Status`), `Balance Normaliser`, `Read-Only Credential Policy`, `Supabase Vault Credential Cipher`, OKX/EVM/Solana connectors; `Update Account Use Case`, `Set Account Active Use Case`, `Sync Account Use Case` (on-demand and scheduled), `Get Sync Status Use Case`, `All-Accounts Sync Fan-Out Use Case` |
| **BRD-005** | Asset catalogue and market data | Asset Registry Service, `registry` schema in Supabase Postgres, Market Data Service, `market` schema in Supabase Postgres, Price Data Providers | `Asset Aggregate`, `Asset Contract`, `Asset Identity Resolver`, `Quarantine Unmapped Observation Use Case`, `Price Observation`, `Price Fallback Policy`, maximum stale age; `Valuation Line Status` excludes unpriced and unmapped assets from the valued total; projections in Account and Portfolio |
| **BRD-006** | Portfolio management | Portfolio Service, `portfolio` schema in Supabase Postgres, Account Service | `Portfolio Aggregate` with unique-name, single-default and 100%-target invariants, `Portfolio Configuration Policy`, account links with active/excluded state, targets, guardrails; repository uniqueness constraints; `Manage Portfolio Use Case`, `Delete Portfolio Use Case` with cascade; `Validate Account Ownership Use Case` calls Account's internal ownership/active-status API at link time |
| **BRD-007** | Valuation, allocation and snapshots | Portfolio Service, `portfolio` schema in Supabase Postgres, Market Data Service | `Valuation Calculator`, `Valuation Line Status`, `Snapshot` with per-line status, `Snapshot Immutability Policy` (first write wins, with the repeat conflict recorded as a deviation in section 15), holdings and price projections with aggregate versions, decimal arithmetic and freshness markers; all reads use Supabase Postgres |
| **BRD-008** | Rebalancing recommendations | Portfolio Service, `portfolio` schema in Supabase Postgres, Reporting Service | `Recommendation` with recorded reproducible inputs (holdings projection version, price observation ids, target and guardrail versions, calculation timestamp), `Generate Recommendation Use Case`, `Record Recommendation Decision Use Case` (accept/reject with actor and audit event), `Expire Recommendations Use Case`, `Recommendation Supersession Policy`, batch worker pool, guardrails and disclaimer |
| **BRD-009** | Reporting and agent integration | Reporting Service, `reporting` schema in Supabase Postgres, Portfolio Service, OpenClaw Agent, Notion, Supabase Vault | `Brief Composer`, `Idempotent Delivery Policy`, `Delivery Retry Policy`, `Brief Payload Contract`, `Apply Reporting Retention Use Case`, `OpenClaw Agent Client`, `Retry Delivery Use Case`, `Supabase Vault Secret Reader` scoped to the OpenClaw agent token only; Notion credentials stay inside OpenClaw and there is no Notion write path or Notion secret in the platform; retention periods still need product/legal sign-off (section 15) |
| **BRD-010** | Admin configuration | Asset Registry Service, `registry` schema in Supabase Postgres, API Gateway, Account Service, Audit Service | `Admin Role Guard Middleware` (registry), `Admin Authorization Policy` (registry), `Registry Admin Handlers`, `Manage Chain Use Case`, `Import Lookup Use Case`, `Sync Contracts from Catalogue Use Case`, `Evaluate Chain Deactivation Use Case`, `Secret Reference Policy`, `Registry Event Versioning Policy`; `Asset Identity Resolver` resolves a chain's native asset from the chain native symbol when it has no contract or mint, so native holdings map without a token entry; affected-account projection, chain-deactivation effect on holdings and registry-change audit. Decimals are immutable after mapping; the BRD-010 BR-ADM-03 change-with-confirmation requirement is a recorded deviation in section 15. |
| **BRD-011** | Background jobs and data refresh | Job Orchestration Service, `job` schema in Supabase Postgres, Supabase Queues (pgmq), Supabase Cron (pg_cron), Account, Registry, Market Data, Portfolio, Reporting, Audit | `Job Definition` (including reporting.retention and identity.retention), `Job Schedule`, `Job Run` with logical run key, attempt id, lease epoch and lease state, `Retry Policy`, `Retry Failed Run Use Case`, `Expire Stale Runs Use Case`, `Cancel Run Use Case`, `Request Backfill Use Case`, `Single-Flight Policy`, `Run Lease and Idempotency Policy` with lease-epoch fencing, `Missed-Run Watchdog Loop`, `job.tick`, `job.run.requested`, per-service job consumers, `All-Accounts Sync Fan-Out Use Case`, `Scheduled Portfolio Fan-Out Use Case` with per-account and per-portfolio fault isolation and a partial-failure run outcome, run outcomes and heartbeats published only by the run owner, Supabase Cron dead-man watchdog; the `snapshots.eod` repeat semantics deviate from BR-JOB-03/AC-JOB-04 and are recorded in section 15 |
| **BRD-012** | Security, compliance and NFRs | Audit Service, `audit` schema in Supabase Postgres, Supabase Storage, Supabase Vault, Supabase Queues, Supabase Auth, Supabase Logs & Metrics, API Gateway, Identity Service, all services | Hash-chained append-only audit with role separation and a per-partition advisory-lock writer lease, crypto-shredding-only erasure, Supabase Vault encryption and secret references, rate limiting, signed assertions verified by every route guard with Supabase Vault-resolved keys, run-scoped System assertions exchanged over mTLS for scheduled and recovery hops, per-service admin guards and admin-authorization policies on every admin surface, an owner-scoped privacy guard, privacy export/erasure coordination, retention jobs for reporting and identity, run records, health on every service, a gateway security-audit channel acknowledged by Supabase Queues before the response with fail-closed behaviour, Supabase Logs & Metrics, and the Supabase Cron dead-man watchdog; backup/PITR is a shared-responsibility item, retention periods need sign-off and the gateway audit durability residual window is recorded (section 15) |

Coverage note: BRD-012 is cross-cutting. No single container owns all of its obligations.
Credential handling is concentrated in Account Service and Supabase Vault, identity in Identity
Service and Supabase Auth, audit and privacy in Audit Service, and rate limiting in the gateway.
Operational telemetry is provided by Supabase Logs & Metrics, and alert routing and the dead-man
watchdog are shared between Job Orchestration and Supabase Cron. Backup, PITR and restore drills
are a shared-responsibility arrangement rather than a modelled container, and are recorded in
section 9.2 and section 15. Open items still requiring product sign-off are listed in section 15.

---

## 13. Key architecture decisions and trade-offs

| # | Decision | Rationale | Trade-off / cost |
| --- | --- | --- | --- |
| 1 | **Supabase-only managed infrastructure** | One vendor provides identity, Postgres, queues, storage, secrets, scheduling and observability; no separate broker, cache, secrets manager or orchestration platform to operate | Single-vendor dependency and plan limits; the managed database is a shared blast radius; fewer platform primitives than a bespoke stack; migration away from Supabase would touch every service |
| 2 | Microservices with **logical database-per-service** | Independent ownership, clear boundaries and independent scaling, while keeping one managed Postgres project and one backup/PITR policy | No physical isolation: a project-level outage, storage exhaustion or bad migration affects every service, and services cannot be restored or upgraded independently. Schema and role isolation contain the logical blast radius but not the physical one |
| 3 | API Gateway (BFF) as the only public entry point | One authentication, authorization, rate-limit and routing policy; services stay private | Extra network hop; risk of the gateway accumulating business logic if discipline slips |
| 4 | **Supabase Queues (pgmq)** as the event backbone instead of a dedicated event-streaming broker | No extra broker to operate; queues live in the same Postgres project and participate in the same transactional outbox; visibility timeouts and archive provide dead-letter handling | Throughput is bounded by Supabase Postgres capacity; no broker-style partitioned consumer groups, unbounded replay or stream retention; queue tables consume database storage and need maintenance |
| 5 | **Supabase Postgres caches** instead of a separate in-memory cache tier | There is no separate cache service to deploy or secure; JWKS, principals, rate-limit counters and last-known prices are durable across restarts | Cache traffic adds load to the same Postgres project as business data; every cache read is a database round trip; TTL and event eviction must be tuned, and the gateway cache is only as available as Postgres |
| 6 | **Supabase Vault (pgsodium)** instead of an external secrets manager or cloud KMS | Secrets and encryption live next to the data, with no extra secret service on the credential path; versioned encryption and per-credential DEKs are available | Secret access is mediated by the Postgres role and shares the database blast radius; envelope-encryption semantics differ from a dedicated KMS; rotation, break-glass and per-subject crypto-shredding must be exercised |
| 7 | **Container Host** (Fly.io / Render / Cloud Run) instead of an orchestrated cluster | Simpler operations, managed TLS, health checks and rolling deploys; one deployment node matches MVP scale | Fewer platform primitives: no built-in service identity, network policy engine, autoscaler tuning or PodDisruptionBudget equivalent; scaling and private networking depend on the host; a service mesh is not available |
| 8 | **Supabase Auth (GoTrue)** as the managed identity provider | No password, MFA, lockout, session or token implementation in the product; standard OIDC-style JWTs and an admin API | Auth is on the sign-in and refresh path; customisation is limited to hooks and `app_metadata`; Auth rate limits and plan limits become product constraints |
| 9 | Event-maintained projections with explicit rebuild endpoints | Avoids distributed transactions and cross-service reads while keeping local queries fast | Duplicate data; projection lag; rebuild paths must be tested; event-retention gaps must be handled |
| 10 | One Portfolio Service for lifecycle, valuation, snapshots and recommendations | These concerns share one consistency boundary; valuation, drift, snapshot and recommendation can commit atomically | A larger service; reads and writes scale together; module boundaries inside the service must be defended |
| 11 | OpenClaw Agent indirection instead of direct Notion writes | Keeps Notion credentials and narrative composition outside the platform; one scoped integration | Extra hop and trust boundary; agent behaviour must be constrained and observable |
| 12 | Job Orchestration as a control plane over Supabase Queues | Durable schedules, single-flight, retries and missed-run detection live in one place; business services own execution; Supabase Cron only supplies the tick | More moving parts than in-process cron; command and outcome contracts must be versioned; a queue delivery is not a run guarantee, so leases and fencing remain necessary |
| 13 | Append-only, hash-chained audit with an insert-only database role | Tamper evidence and separation from application data; supports compliance queries | Corrections must be appended, not edited; a write-only role complicates operational fixes |
| 14 | Rust with Clean Architecture in every service | Memory safety, predictable performance, explicit boundaries, testable domain logic | Higher initial design and compile-time cost; more boilerplate than a layered CRUD service |
| 15 | Decimal arithmetic for quantities and money | Reproducible valuation and snapshots; no binary floating-point drift | Slower than native floats; developers must use the decimal type consistently |
| 16 | Single-tenant MVP with owner UUIDs everywhere | Smaller surface for launch; avoids premature tenant, billing and sharing models | Multi-tenant and shared portfolios remain post-MVP work; the data model must not hard-code assumptions that block them |
| 17 | Supabase Postgres principal cache at the gateway with identity-event eviction | Removes Identity from the per-request hot path; survives Identity outages for cached principals | Eventual status propagation up to the TTL: a disabled or deactivated user keeps the cached access until the identity event evicts it or the TTL expires (section 15). A cache miss never grants access; it resolves through the ensure-user path and fails closed if that lookup fails |
| 18 | Signed internal principal assertion plus per-service authorization | Prevents a compromised process or routing bug from reading any owner's data or performing admin operations | Every route guard — public, admin, privacy and internal — must validate the assertion through the verifier and keep an endpoint authorization matrix; more work than trusting the private network alone |
| 19 | Versioned, ordered holdings events | Prevents an older at-least-once delivery from overwriting newer state | Producers must assign and persist aggregate versions; consumers must compare versions |
| 20 | **Supabase Cron dead-man watchdog** outside Job Orchestration | A scheduler cannot reliably detect its own downtime, and pg_cron plus pg_net needs no extra runtime service | The watchdog is SQL-based and must be tested; pg_net availability, Cron limits and alert-route monitoring become operational concerns |
| 21 | Crypto-shredding-only erasure in an append-only audit store | Preserves hash-chain integrity while satisfying erasure obligations | Per-subject keys must be managed outside the chain; personal data must be pseudonymised on the ingestion path, because chained rows are never rewritten after ingestion |
| 22 | First-write-wins immutable snapshots | Reproducibility and audit consistency; no silent mutation of historical values | A repeat or re-run by a different writer is rejected and handled operationally rather than overwritten; only a redelivery of the same (run key, attempt) is treated as already captured. This conflicts with BRD-007 FR-VAL-09/AC-VAL-05 and BRD-011 BR-JOB-03/AC-JOB-04 and is recorded as a deviation in section 15 |
| 23 | **No service mesh**; TLS on the private network plus signed assertions | The container host does not provide mesh identity, and the Supabase-only stack avoids another control plane; the consumed-interface risk is addressed by short-lived assertions | No automatic mTLS identity rotation or automatic traffic policy from a mesh; certificates, private-network exposure and per-endpoint authorization matrices are the product's responsibility |

### 13.1 Logical versus physical database isolation

The central scalability and resilience trade-off is decision 2. Logical isolation buys a single
connection pooler, a single backup and PITR policy, one set of plan limits and far less
operational surface. Physical database-per-service would buy an independent blast radius,
independent restores and independent engine upgrades, at the cost of eight projects (or
instances), eight backup policies, more connection management and a much larger operational
budget. The MVP chooses logical isolation deliberately and records the residual risk in section
15. The architecture keeps the option open: because every service already owns a schema and a
role and never joins across schemas, moving a service to its own Supabase project later is a
connection-and-migration change, not a domain redesign.

---

## 14. How to change the model and keep it consistent

### 14.1 Source of truth

- `workspace.dsl` defines structure: people, systems, containers, components, relationships,
  deployment nodes and views.
- The BRDs define business intent: actors, rules, requirements and acceptance criteria.
- This README explains and indexes both. It must be updated in the same change as the DSL when a
  service, view, event family, deployment node or traceability row changes.
- Generated exports are not sources of truth. Do not hand-edit them.

### 14.2 Common changes

| Change | Steps |
| --- | --- |
| Add or rename a component | Add or edit the `component` inside the correct layer group of its container; add relationships with a verb phrase and technology; ensure the owning component view includes it; update section 8.4 if the layer contents change. |
| Add a service | Add a `container` with identifier, description, technology and a nested four-group block; declare its owned Supabase Postgres schema and role in the description and this README; add a component view and its row in section 4; add a `containerInstance` on `Container Host`; add it to the `Containers` view; update sections 3.3, 6, 7, 8.4, 10, 12 and 13; add dynamic views if it participates in a new flow. |
| Add a Supabase-backed data concern | Add the table or type to the owning service's schema and role; never add a separate database container. Update section 3.3, section 6, section 9 and section 12. If it is a cache, document TTL, eviction and fallback. |
| Add an external system | Add a `softwareSystem` tagged `External`; connect it to the owning container with protocol and trust details; include it in the context view; update section 5.2; add Supabase Vault or Supabase project-secret handling if it needs credentials. |
| Add a synchronous flow | Add a relationship to the owning container or component; label the mechanism (`mTLS REST client`, `HTTPS`, `JSON-RPC`); if it is a user-visible sequence, add a dynamic view and update section 4. |
| Add an event family | Add the publisher and consumer relationships tagged `Async`; use a past-tense business fact; update section 7; define the queue, outbox and inbox dedup strategy in the owning service. |
| Add a deployment node | Add a `deploymentNode` under `Production` only if it genuinely runs product code. Supabase services and third-party providers stay external; do not add data-tier nodes. Add `containerInstance` entries and update section 10. |
| Add or rename a view | Add a view with a stable `PascalCase` key; include the relevant elements; add `autolayout`; update the view index in section 4. Do not reuse a retired key for a different meaning. |
| Change a BRD | Update the BRD and its version; update the traceability row in section 12; change the DSL only if the architecture changes, not to restate a business rule. |
| Retire an element | Remove it from views and deployment, then from the model; update sections 6, 7, 10 and 12; keep the BRD's retired-requirements table as the business history. |

### 14.3 Naming and modelling conventions

- Identifiers: lowerCamelCase (`accountService`, `supabasePostgres`, `containerHost`).
- Display names: title case, human-readable (`Account Service`, `Supabase Postgres`).
- Layer groups: exactly `Transport`, `Application`, `Domain`, `Infrastructure`.
- Component names: prefix with the service short name; keep them unique within the service.
- Data naming: `<Service> Service` for containers and the owned schema name (`identity`,
  `account`, `registry`, `market`, `portfolio`, `reporting`, `job`, `audit`, plus `gateway` for
  the cache schema). There is no per-service database container in the model.
- View keys: `PascalCase`; titles are short noun phrases; dynamic view titles are verb phrases.
- Relationships: verb phrase plus technology, for example "Reads and writes portfolios…" with
  technology "Supabase Postgres (schema portfolio, role portfolio)".
- Async relationships: tag `Async` so the style renders them dashed.
- Tags in use: `Person`, `Software System`, `External`, `Supabase`, `Container`, and the four
  layer tags. The styles block still defines `Database`, `Queue`, `Cache` and `Bucket` shapes for
  model compatibility, but the Supabase-only model does not declare datastore containers.
- Do not encode a business rule only in a component description; link to the BRD.

### 14.4 Review checklist

Before merging a model change:

- [ ] The DSL validates and every view renders.
- [ ] Every declared component has at least one relationship; no orphan boxes.
- [ ] Every new container has an owner, an owned schema/role decision (or an explicit "none"
      for the SPA) and a `containerInstance` on the `Container Host`.
- [ ] Every new component sits in one of the four layer groups.
- [ ] No domain component depends on infrastructure; any Transport-to-Infrastructure edge is one
      of the documented exceptions (health/readiness probes, Supabase Queues consumer projection
      writes, SSE ticket verification, or a route guard/handler calling its `Principal Assertion
      Verifier`).
- [ ] Every asynchronous relationship is tagged `Async`, including Supabase Queues publishers,
      consumers and coordinators and the SSE progress-stream edge.
- [ ] Every internal call states its transport and shows the signed principal assertion, the
      forwarding rule and the `Principal Assertion Verifier` that validates it; every public,
      admin, privacy and internal route guard verifies the assertion, not only the internal
      guard. Every external call states its protocol.
- [ ] Scheduled and recovery hops obtain a run-scoped System assertion from the Job Orchestration
      run-assertion exchange over mTLS; no assertion, credential or system role travels inside an
      event payload.
- [ ] Every backend service has health and readiness endpoints and a `Principal Assertion
      Verifier`; the gateway is the assertion issuer and validates Supabase JWTs instead; the
      first-sighting ensure-user path is distinct from assertion-based lookup and presents the
      Supabase access token over mTLS.
- [ ] Every admin surface has an admin guard and an admin-authorization policy that accepts either
      the Supabase admin claim or a run-scoped System assertion bound to the run key; scheduled
      jobs re-check the policy and do not rely on the schedule as authorization.
- [ ] Every consuming service has an outbox/inbox decision, including Market Data, Job and Audit.
- [ ] Every job consumer deduplicates on the logical run key plus attempt, and every run lease has
      a heartbeat-renewed TTL, a monotonically increasing lease epoch used as a fencing token,
      stale-run expiry and bounded retry. Only the run owner publishes the run outcome and
      heartbeat; fan-out aggregates per-item progress.
- [ ] Every fan-out states per-item fault isolation and a partial-failure outcome; snapshot
      capture states its retry idempotency carve-out for a redelivered (run key, attempt).
- [ ] State-changing events that update a mutable projection carry an aggregate id, version and
      `occurredAt`, and the modelled policy exists for that family.
- [ ] No service is granted access to another service's schema; no business data is claimed to
      live in the gateway cache schema.
- [ ] No datastore, cache, queue or storage deployment node is added; these are Supabase-managed
      external systems.
- [ ] The view index and service catalogue match the DSL names and counts exactly (16 views, 10
      containers, 14 external software systems).
- [ ] New events appear in the event catalogue with producer and consumers.
- [ ] The BRD traceability matrix still covers BRD-001 … BRD-012 and open deviations are recorded.
- [ ] No tooling, vendor or environment is claimed that is absent from the model.
- [ ] This README is updated in the same change as the DSL.

### 14.5 Change control

- One pull request should update `workspace.dsl`, this README and any affected BRD together.
- Validate with `structurizr-cli validate` (or the Docker-based CLI) before review. If neither
  Java nor a running Docker daemon is available, state that in the pull request and perform a
  manual DSL review.
- Do not commit generated exports unless the team explicitly decides to publish rendered views;
  if it does, treat them as artefacts and regenerate them from the DSL.
- Do not modify the BRDs to fit the architecture. Change the architecture or raise the conflict.
- Keep the model honest: an element that is planned but not implemented should be described as a
  target, not drawn as if it exists in production.

---

## 15. Open items requiring product or security sign-off

> The Structurizr DSL component descriptions reference this register as "README section 15".

These are documented gaps rather than hidden claims. They do not block the model, but they need
a decision before the affected requirement can be called implemented:

1. **Price freshness conflict.** BRD-001 mentions under 30 seconds while BRD-011 specifies a
   15-minute collection cadence and BRD-012 leaves OQ-001 open. The architecture is poll-based at
   15 minutes. Either the 30-second target applies only to valuation freshness relative to the
   latest observation, or Market Data needs streaming/higher-frequency collection and a larger
   provider budget.
2. **Notification capability.** BRD-001 (drift guardrail warnings) and BRD-010 (warn affected
   users before chain deactivation) require a notification channel. No in-app or email
   notification service is modelled; the chain-deactivation warning is currently surfaced to the
   Admin only. Add a notification capability or formally scope these out.
3. **Supabase plan limits.** The chosen Supabase plan fixes monthly active users, database size
   and compute, storage, egress, Auth request rates, pgmq throughput, Cron frequency and log
   retention. These limits must be recorded, mapped to the launch capacity target (BRD-012
   NFR-SCL-01) and monitored. Exceeding a limit is a product-visible outage, not only an
   infrastructure event.
4. **PITR, backups and restore drills.** BRD-012 NFR-SEC-14 and NFR-AVL-04/05 are now a
   shared-responsibility arrangement rather than a platform gap: Supabase supplies encrypted
   backups and PITR, but the product must confirm the retention window, cover Supabase Storage
   archives and Supabase Vault key material, and run restore drills with evidence. Until that is
   done, the RPO/RTO requirements are not claimed as verified.
5. **Optional Supabase read replicas.** Read replicas are **not** a current element. They are a
   future scaling option only: if dashboard, oversight or snapshot-history reads exceed the
   primary's capacity, read replicas could serve them within a bounded lag budget while
   valuation, snapshot capture and recommendation computation keep reading the primary. Any
   adoption requires a new model element, a freshness label and a lag-fallback policy.
6. **pgmq throughput and operations.** Supabase Queues is Postgres-backed, so peak event,
   job-command and audit volume competes with business writes. Throughput, visibility-timeout
   tuning, archive growth and dead-letter alerting must be load-tested before launch. If
   throughput is insufficient, the architecture would need a separate broker and a new model.
7. **Supabase Auth rate limits and availability.** Sign-in, token refresh, Auth Hooks and Admin
   API calls are subject to Supabase rate limits. Confirm the limits for the expected user base,
   the hook timeout budget, the refresh strategy and the behaviour when Auth is unavailable.
8. **Log and metric retention.** Supabase Logs & Metrics retention follows the plan. Confirm the
   window is sufficient for incident review and BRD-012 audit expectations; if not, add an
   external log sink. Note that the business audit trail is in Audit Service, not in platform
   logs.
9. **No service mesh.** TLS on the container host's private network plus signed assertions
   replaces a mesh, but there is no automatic certificate rotation, automatic traffic policy or
   sidecar identity. Confirm the host's private-network guarantees and document certificate and
   key rotation. A service mesh is not part of the Supabase-only stack.
10. **Application session semantics.** BRD-003 describes an HTTP-only/Secure application session
    cookie, while the architecture is a stateless BFF with SPA tokens held in memory and
    sessions managed by Supabase Auth. The BRD wording and the sign-out/refresh-token
    revocation sequence must be reconciled. **Must-level deviation.**
11. **Observability vendor and SLOs.** Supabase Logs & Metrics is the current platform; its
    retention, dashboards, SLO tooling and alert routes are plan-dependent. Any additional
    observability vendor and the SLO definitions are platform decisions not fixed here.
12. **Snapshot repeat semantics conflict.** BRD-007 FR-VAL-09/AC-VAL-05 and BRD-011
    BR-JOB-03/AC-JOB-04 require a repeat for the same portfolio, date and type to update the
    existing record. The architecture enforces first-write-wins immutability instead (section
    11.5, decision 22) and the model says so explicitly. Either product accepts rejection as the
    intended behaviour and the BRDs are amended, or Portfolio Service must implement an
    idempotent upsert for an unsealed snapshot key while keeping published snapshots immutable.
    Until that decision is made, those Must requirements are not claimed as implemented.
    **Must-level deviation.**
13. **Decimal change semantics conflict.** BRD-010 BR-ADM-03 allows an admin to change a token's
    decimals with explicit confirmation and re-sync, and BRD-005 BR-AST-04 allows a reviewed
    change with re-normalisation. The architecture treats decimals as immutable after mapping
    (section 11.5, `Map Asset Contract Use Case`). Either product accepts absolute immutability
    and amends BR-ADM-03/BR-AST-04, or a reviewed change path must be designed that
    re-normalises holdings and states the snapshot impact. Until then those requirements are not
    claimed as implemented. **Must-level deviation.**
14. **Retention periods per personal-data category.** BRD-012 NFR-PRV-02 requires a documented
    retention period per personal-data category, and BRD-009 OQ-005 leaves reporting retention
    open. The retention mechanism is modelled for audit, prices and snapshots and also for
    reporting and identity (`reporting.retention`, `identity.retention`), but the actual periods
    for identity records and reporting narratives/delivery logs need product and legal sign-off.
15. **Gateway security-audit durability window.** The gateway publishes security audit events
    synchronously and completes a request only after Supabase Queues acknowledges the record; if
    the publish cannot be acknowledged within the bounded timeout the request fails closed. A
    residual window therefore exists between the bounded timeout expiring and the fail-closed
    response, and durability ultimately depends on Supabase Postgres and the pgmq queue table.
    Gateway processes keep no local durable spool, so there is no second copy to reconcile.
    Security must confirm the bounded timeout value and whether any additional durable local
    spool is required for the BRD-012 audit-completeness requirement.
16. **Disabled-user cache propagation window.** The gateway caches an authenticated principal
    until an identity status event evicts it or the cache TTL expires. A user disabled or
    deactivated during that window can keep making requests within the TTL. Event eviction makes
    this rare, but the worst case is the TTL. Security must sign off the TTL as the accepted
    worst-case revocation latency, or require a shorter TTL, a per-request status check, or
    both. **Must-level deviation.**

> Items 10, 12, 13 and 16 are Must-level deviations that require explicit product or security
> sign-off before this model is treated as compliant with the corresponding BRD. Items 3, 4, 6,
> 7, 8 and 9 are Supabase-platform dependencies that require verification before launch. They
> are disclosed here rather than silently claimed as implemented.

---

## Related documents

- [Structurizr workspace](./workspace.dsl)
- [Business requirements index](../brds/README.md)
- [BRD-001 — Product Vision, Scope & Success](../brds/BRD-001-product-vision-scope-and-success.md)
- [BRD-012 — Security, Compliance & Non-Functional Requirements](../brds/BRD-012-security-compliance-and-non-functional-requirements.md)
