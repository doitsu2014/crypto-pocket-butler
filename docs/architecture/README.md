# Crypto Pocket Butler — C4 Architecture

Architecture documentation for **Crypto Pocket Butler**, a read-only crypto portfolio
aggregation and decision-support platform.

The model of record is the Structurizr DSL workspace in [`workspace.dsl`](./workspace.dsl).
This README explains that model. If this document and the DSL disagree about structure, the
DSL is authoritative. If they disagree about a business rule, the [BRDs](../brds/README.md)
are authoritative.

The model describes a **Supabase-only managed infrastructure**: the Rust services and the
React single-page application run on a container host, while authentication, relational data,
object storage, secrets, scheduling and observability are consumed from Supabase.

| Property | Value |
| --- | --- |
| Model file | [`workspace.dsl`](./workspace.dsl) |
| Requirements baseline | BRD-001 … BRD-012, Draft v1.0/v1.1 (aligned 2026-11-02) |
| Simplified | 2026-11-02 (machinery stripped, topology kept; OpenClaw/Notion and price aggregators removed — see section 2) |
| People | 2 |
| External software systems | 10 (6 Supabase services + 4 third-party integrations) |
| Containers | 10 (1 React SPA + 9 Rust services) |
| Components | 0 — C4 levels 1–2 only; component detail lives in the code |
| Views | 5 |
| Relationships | 87 |
| Deployment environments modelled | 1 (`Production`), one `Container Host` node |

---

## 1. Purpose

This documentation gives product, engineering and operations one shared, diagram-backed
description of the system. It answers four questions:

1. **What is the system boundary?** Which capabilities the platform owns and which belong to
   Supabase, the exchange, blockchains or the alerting channel.
2. **Who owns which data?** Every service owns one schema and one Postgres role; no service
   reads another service's tables.
3. **How do the parts communicate?** Synchronous HTTP everywhere. There are no message queues.
4. **Why was it built this way?** Section 10 records the decisions and their costs.

---

## 2. The 2026-11-02 simplification

The previous revision modelled this single-tenant MVP with distributed-systems machinery that
cost more than it protected, and with two integrations the product no longer wants. The
service topology (10 containers) was kept deliberately; the machinery and the integrations
were stripped. The previous model remains in git history.

| Removed | Replaced with |
| --- | --- |
| Supabase Queues (pgmq), transactional outbox, inbox dedup, event consumers | Direct internal HTTP calls with bounded retries; scheduled reconciliation jobs repair any missed notification |
| mTLS between services | Private network + forwarded Supabase JWT (user paths) or a shared internal service token from Vault (job paths); services are not publicly reachable |
| Ed25519 signed principal assertions, signing/verification keys, rotation windows | The gateway forwards the user's Supabase JWT unchanged; every service validates it |
| Identity ensure-user handshake with the gateway | Identity upserts the internal user from a signed Supabase Auth webhook or lazily from the validated JWT on first lookup |
| Hash-chained audit partitions with advisory-lock writer lease | Append-only audit table enforced by an insert-only Postgres role (no UPDATE/DELETE grants) |
| Dead-man watchdog with lease epochs, fencing and heartbeats | Job service writes one heartbeat row; pg_cron checks it and alerts via pg_net |
| Ed25519 SSE stream tickets | The SSE progress stream authenticates with the session JWT |
| JWKS and principal caches in Postgres, Postgres atomic-counter rate limits | In-memory caches and in-memory rate limits in the stateless gateway |
| 318 components in hexagonal groups, 9 component views, 16 views total | Container-level model only; 5 views |
| OpenClaw Agent and Notion publication path | Reporting Service composes daily/weekly briefs in-app and serves them to the SPA through the gateway |
| Price Data Providers (CoinGecko / CoinPaprika) | Market Data Service collects USD prices, candle histories and instrument/pair metadata from the OKX Exchange API |

**Kept on purpose** (business rules, not machinery): read-only credentials encrypted in
Supabase Vault; decimal arithmetic for quantities and values; unpriced/unmapped assets
labelled and excluded from valued totals; deterministic, explainable, human-reviewed
recommendations; idempotent brief generation; append-only audit; crypto-shredding erasure;
per-service schemas and roles.

**BRD alignment (applied 2026-11-02).** The BRDs were revised to v1.1 wherever they mandated
removed machinery or integrations: BRD-001 (goals, risks, metrics), BRD-005 (exchange-sourced
market data replaces CoinGecko/CoinPaprika), BRD-008, BRD-009 (in-app briefs replace
OpenClaw/Notion; retired IDs tracked in its §11), BRD-010 (manual contract entry replaces
provider lookup), BRD-011, BRD-012 (no queues/pgmq; validated JWT principal replaces signed
assertions; in-memory rate limits), plus the GLOSSARY and BRD index.

---

## 3. Opening the model

Requires Docker (the Docker daemon must be running).

Interactive viewing with Structurizr Local — the bind mount must point at the **directory**
containing `workspace.dsl`:

```bash
docker run -d --rm --name structurizr \
  -p 8080:8080 \
  -v "$(pwd)/docs/architecture:/usr/local/structurizr" \
  structurizr/structurizr local
```

Then open <http://localhost:8080> (use another host port if 8080 is taken). Restart the
container after editing the model. It writes a generated `workspace.json` and a
`.structurizr/` log directory; both are gitignored build artefacts.

Validate and export with the Docker-based CLI:

```bash
docker run --rm \
  -v "$(pwd)/docs/architecture:/usr/local/structurizr" \
  structurizr/cli validate \
  -w /usr/local/structurizr/workspace.dsl

docker run --rm \
  -v "$(pwd)/docs/architecture:/usr/local/structurizr" \
  structurizr/cli export \
  -w /usr/local/structurizr/workspace.dsl \
  -f /usr/local/structurizr/export \
  -t mermaid
```

On PowerShell, replace `$(pwd)` with `${PWD}` and use backslashes in the mount path.
Exports are build artefacts, not part of the model of record.

---

## 4. System boundary

**People**

| Person | Role |
| --- | --- |
| Portfolio Owner | Connects read-only accounts, builds portfolios, reviews valuation, drift, snapshots and recommendations |
| Admin | Holds the `app_metadata.role` admin claim; manages registry, jobs, audit queries and privacy requests |

**External systems**

| System | What it provides |
| --- | --- |
| OKX Exchange API | Read-only balances, ticker prices, candle histories and instrument/pair metadata via HMAC-signed REST |
| EVM JSON-RPC Providers | Native and ERC-20 balances and token metadata, primary/fallback per chain |
| Solana RPC Provider | SOL and SPL balances and mint metadata |
| Operational Alerting Channel | Consumer of missed-run, failure, error-rate and stale-price alerts |
| Supabase Auth | Identity provider: JWTs, JWKS, MFA, custom access-token hook, signed webhooks, Admin API |
| Supabase Postgres | One schema + role per service, no cross-schema grants, Supavisor pooler |
| Supabase Storage | Private buckets for audit export and retention archives |
| Supabase Vault | pgsodium secrets: exchange credentials, provider keys, service token, per-subject encryption keys |
| Supabase Cron (pg_cron) | Ticks the scheduler via pg_net, runs retention SQL, hosts the dead-man check |
| Supabase Logs & Metrics | Platform observability with correlation context |

---

## 5. Containers

| Container | Owns | Schema |
| --- | --- | --- |
| React Web Client (SPA) | UI; Supabase Auth flows in-browser; tokens in memory only | — |
| API Gateway (BFF) | JWT validation, principal resolution (TTL cache), deny-by-default routing, admin-claim gating, rate limits, correlation IDs, SSE proxy. Stateless | — |
| Identity Service | Internal users keyed by `auth.users.id`, roles from `app_metadata.role`, user status, session events, deletion/anonymisation, retention. No passwords or MFA state | `identity` |
| Account Service | Read-only exchange/wallet accounts (public-address-only wallets), Vault-encrypted credentials, holdings sync pipeline (decimal, zero-filtered, cross-chain merged, fault-isolated), sync status + SSE progress, registry-projection rebuild | `account` |
| Asset Registry Service | Canonical asset identity, chains and token registries, review-before-activation import, catalogue-to-registry sync, quarantine for unmapped observations | `registry` |
| Market Data Service | USD observations from OKX tickers, candle histories and instrument/pair metadata, shared request budget, freshness, fallback to last stored price, maximum stale age → unpriced | `market` |
| Portfolio Service | Portfolios (unique name, single default, 100% targets), valuation, drift, immutable snapshots, advisory recommendations, projections | `portfolio` |
| Reporting Service | Reporting preferences, in-app brief composition (idempotent per owner/portfolio/period), generation log, retention | `reporting` |
| Job Orchestration Service | Schedules, run triggering, single-flight advisory locks, run records, missed-run detection, bounded retries/backfill, alerts ≤15 min, heartbeat. No business data | `job` |
| Audit Service | Append-only audit events (insert-only role), admin queries, privacy exports in Storage, erasure by crypto-shredding, partition retention | `audit` |

All backend services are Rust (Axum, sqlx); the SPA is React 18 + TypeScript 5.

---

## 6. Communication rules

1. The SPA calls **only** the API Gateway, plus Supabase Auth directly for sign-up/sign-in
   (the single browser-to-Supabase exception).
2. The gateway validates the Supabase JWT (JWKS, in-memory cache), resolves the principal via
   Identity (short-TTL cache), then **forwards the user's JWT unchanged** to the owning
   service over the private network. Every service validates the JWT itself as defence in
   depth and enforces owner scoping.
3. Service-to-service calls are plain internal HTTP: synchronous where the caller needs an
   answer (lookups, snapshots, brief data), fire-and-forget with bounded retries for
   notifications (holdings replacement, unmapped observations, audit events). Idempotency
   keys or version checks make retries safe.
4. Scheduled runs are triggered by the Job Orchestration Service calling each service's
   internal run endpoint with the shared service token from Vault; the HTTP response is the
   run outcome.
5. No message queues. Any missed notification is repaired by the scheduled reconciliation
   jobs (portfolio ↔ account/registry/market comparisons).
6. Security-relevant audit events (access denied, rate limited) are written synchronously;
   domain audit events are best-effort with local logging as fallback.

---

## 7. Data ownership

- One schema and one Postgres role per service; no cross-schema grants. A service's data is
  read by nobody else — other services ask that service over HTTP.
- The audit schema is appended through an insert-only role; retention uses a separate
  audited privileged role.
- Secrets live only in Supabase Vault: exchange credentials (encrypted, masked, never
  logged or published), RPC provider keys, the internal service token, and per-subject
  crypto-shredding keys.

---

## 8. Jobs and scheduling

pg_cron ticks the Job Orchestration Service every minute via pg_net. The scheduler takes a
Postgres advisory lock per job (single-flight), calls the owning service's run endpoint, and
records the outcome. Missed runs are detected from the durable schedules; retries and
backfills are bounded; alerts fire within 15 minutes. A separate pg_cron dead-man check
alerts through pg_net if the scheduler's heartbeat row goes stale.

Scheduled work: account syncs, catalogue sync and reconciliation, price collection,
valuation, snapshots, recommendations, brief generation, retention (identity, reporting,
audit), portfolio reconciliation.

---

## 9. Views

| Key | Name | Level |
| --- | --- | --- |
| `SystemContext` | System Context | C4 level 1 |
| `Containers` | Containers | C4 level 2 |
| `ProductionDeployment` | Production Deployment | Deployment (supplementary) |
| `AccountSyncToValuation` | Account Sync to Valuation | Dynamic |
| `RecommendationToReporting` | Recommendation to Reporting | Dynamic |

No service has a component view — component structure is a code concern. The deployment
view covers `Production` only; Supabase services are external systems, not deployment nodes.

---

## 10. Decisions and costs

| Decision | Cost accepted |
| --- | --- |
| Keep 9 Rust services + gateway (rather than collapsing to a monolith) | 9 deployables, 9 health probes, 9 pipelines — accepted to preserve the BRD service boundaries and independent scaling of sync/price work |
| No message queues; notifications are HTTP + retry + reconciliation | Each notification path re-implements bounded retry; acceptable at single-tenant volume, revisit if services ever fan out to many consumers |
| Forwarded Supabase JWT instead of signed internal assertions | Every service needs JWKS access and clock-skew tolerance; removes an entire key-management subsystem |
| In-memory gateway caches and rate limits | Per-instance state; multi-replica gateways rate-limit per replica, fine for 2 users |
| Append-only role instead of hash-chained audit | Tamper-evidence relies on Postgres grants, not cryptographic chaining; satisfies BRD-012's append-only requirement |
| Crypto-shredding kept for erasure | Per-subject Vault keys must be provisioned with the data they protect |
| Exchange-sourced market data (OKX) instead of aggregators | Only exchange-listed pairs get prices; on-chain assets without an OKX pair go unpriced (labelled, excluded from totals). Revisit when a second exchange or an aggregator is genuinely needed |
| In-app briefs instead of OpenClaw/Notion publication | No external narrative composition; briefs are deterministic templates rendered in the SPA |

---

## 11. BRD coverage

| BRD | Where it is satisfied |
| --- | --- |
| BRD-001 Product vision and scope | Whole model; read-only aggregation, USD valuation, advisory recommendations, briefs |
| BRD-002 Users, roles and permissions | Identity Service (roles from `app_metadata.role`), gateway deny-by-default + admin gating |
| BRD-003 Authentication and sessions | Supabase Auth + SPA (PKCE, MFA, magic link), Identity webhooks, session revocation |
| BRD-004 Account management | Account Service (read-only credentials, Vault encryption, sync pipeline) |
| BRD-005 Asset catalog and market data | Asset Registry Service, Market Data Service (OKX-sourced tickers, candles, pairs) |
| BRD-006 Portfolio management | Portfolio Service (lifecycle, unique name, single default, 100% targets) |
| BRD-007 Valuation, allocation and snapshots | Portfolio Service (decimal valuation, unpriced labelling, immutable snapshots, drift) |
| BRD-008 Rebalancing recommendations | Portfolio Service (deterministic, explainable, human accept/reject) |
| BRD-009 Reporting and agent integration | Reporting Service (in-app briefs; external publication is Won't-MVP per FR-REP-15) |
| BRD-010 Admin configuration | Registry admin (chains/tokens), Job admin, gateway admin-claim gating, audit queries |
| BRD-011 Background jobs and data refresh | Job Orchestration Service + Supabase Cron, section 8 |
| BRD-012 Security, compliance and NFRs | Sections 6–8: JWT validation everywhere, Vault, append-only audit, exports/erasure, retention, alerts |

---

## 12. What to check after rendering

- Every container in the `Containers` view has a justified owner and data decision: each
  backend service owns one Supabase Postgres schema and role; the gateway owns nothing.
- No relationship implies a queue, mTLS, a signed assertion, Notion or a price aggregator;
  if one appears, the model has drifted from sections 2 and 6.
- The two dynamic views complete without a hop through Supabase Queues (it no longer exists).
- Validate with the Docker-based CLI (section 3) before review; if validation fails, state it
  in the pull request rather than merging an unrenderable model.
