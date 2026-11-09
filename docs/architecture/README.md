# Crypto Pocket Butler — C4 Architecture

Architecture documentation for **Crypto Pocket Butler**, a global retail crypto portfolio
aggregation, P&L and decision-support platform.

The model of record is the Structurizr DSL workspace in [`workspace.dsl`](./workspace.dsl).
This README explains that model. If this document and the DSL disagree about structure, the
DSL is authoritative. If they disagree about a business rule, the [BRDs](../brds/README.md)
are authoritative.

The model describes a **Supabase-as-database-platform + Docker Swarm** infrastructure: the Rust
services, the React single-page application, the support console and the self-hosted
observability stack run on a Docker Swarm cluster, while authentication, relational data,
object storage, secrets and scheduling are consumed from Supabase.

| Property | Value |
| --- | --- |
| Model file | [`workspace.dsl`](./workspace.dsl) |
| Requirements baseline | BRD-001 … BRD-021, Draft v1.0–v2.2 (aligned 2026-11-04) |
| History | Simplified 2026-11-02; global retail extension 2026-11-04 (BRDs 013–021) |
| People | 3 |
| External software systems | 17 (5 Supabase services + 12 third-party integrations) |
| Containers | 16 (1 React SPA + 1 support console + 13 Rust services including the gateway + 1 observability stack) |
| Components | 0 — C4 levels 1–2 only; component detail lives in the code |
| Views | 10 |
| Relationships | 143 |
| Deployment environments modelled | 1 (`Production`), one Docker Swarm cluster |

---

## 1. Purpose

This documentation gives product, engineering and operations one shared, diagram-backed
description of the system. It answers five questions:

1. **What is the system boundary?** Which capabilities the platform owns and which belong to
   Supabase, exchanges, blockchains, market-data providers, notification providers, the
   merchant of record, support tooling or the alerting channel.
2. **Who owns which data?** Every service owns one schema and one Postgres role; no service
   reads another service's tables.
3. **How do the parts communicate?** Synchronous HTTP everywhere. There are no message queues.
4. **Where do global concerns live?** Base currency, locale, FX, privacy, entitlements,
   notifications, analytics and support each have a named owner.
5. **Why was it built this way?** Section 10 records the decisions and their costs.

---

## 2. Architecture history

### 2.1 The 2026-11-02 simplification

The previous revision modelled a single-tenant MVP with distributed-systems machinery that
cost more than it protected, and with two integrations the product no longer wants. The
service topology was kept deliberately; the machinery and the integrations were stripped. The
previous model remains in git history.

| Removed | Replaced with |
| --- | --- |
| Supabase Queues (pgmq), transactional outbox, inbox dedup, event consumers | Direct internal HTTP calls with bounded retries; scheduled reconciliation jobs repair any missed notification |
| mTLS between services | Private network + forwarded Supabase JWT (user paths) or a shared internal service token from Vault (job paths); services are not publicly reachable |
| Ed25519 signed principal assertions, signing/verification keys, rotation windows | The gateway forwards the user's Supabase JWT unchanged; every service validates it |
| Identity ensure-user handshake with the gateway | Identity upserts the internal user from a signed Supabase Auth webhook or lazily from the validated JWT on first lookup |
| Hash-chained audit partitions with advisory-lock writer lease | Append-only audit table enforced by an insert-only Postgres role (no UPDATE/DELETE grants) |
| Dead-man watchdog with lease epochs, fencing and heartbeats | Job service writes one heartbeat row from its leased tick loop; a Prometheus rule on its age (180 s, `absent()` included) pages within five minutes |
| Ed25519 SSE stream tickets | The SSE progress stream authenticates with the session JWT |
| JWKS and principal caches in Postgres, Postgres atomic-counter rate limits | In-memory caches and in-memory rate limits in the stateless gateway |
| 318 components in hexagonal groups, 9 component views, 16 views total | Container-level model only; 10 views |
| OpenClaw Agent and Notion publication path | Reporting Service composes daily/weekly briefs in-app and serves them to the SPA, email and PDF |
| Price Data Providers (CoinGecko / CoinPaprika) | Market Data Service collects prices, FX and benchmarks from multiple licensed sources with deterministic priority |

**Kept on purpose** (business rules, not machinery): read-only credentials encrypted in
Supabase Vault; decimal arithmetic for quantities and values; unpriced/unmapped assets
labelled and excluded from valued totals; append-only transaction and audit records;
deterministic, explainable, human-reviewed user-initiated order intents and trades with
immediate allocation effect; idempotent brief and notification generation; crypto-shredding
erasure; per-service schemas and roles.

### 2.2 The 2026-11-04 global retail extension

BRDs 013–021 added the product domains required for a global mass-market retail launch. The C4
model now includes their containers, data ownership and integrations.

| Added | Why |
| --- | --- |
| Ledger Service (`ledger` schema) | Transactions, cost basis, realized/unrealized P&L and reconciliation are a distinct domain from valuation |
| Notification Service (`notification` schema) | Drift, price, sync, key, security and brief alerts need preferences, dedupe and delivery |
| Billing Service (`billing` schema) | Plans, entitlements, quotas, subscriptions, invoices and dunning are server-side concerns |
| Analytics Service (`analytics` schema) | Consent-aware activation, retention, conversion, flags and experiments |
| Support Console | Least-privilege, time-bound, audited support access without credentials or portfolio contents |
| Multi-venue and multi-chain coverage | Five exchange APIs, EVM, Solana, Bitcoin, Tron, WalletConnect, xpub and CSV |
| Multi-source prices, FX and benchmarks | Trustworthy valuation and performance beyond a single exchange |
| Transactional email, web push, merchant of record, help desk and status page | Global notification, billing and support obligations |

The previous 2026-11-02 model remains the base; the extension is additive and preserves the
no-queue, per-service-schema and forwarded-JWT decisions.

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

Validate and export with the Docker-based CLI — the same `structurizr/structurizr` image the
preview uses, so validation matches what the preview will accept:

```bash
docker run --rm \
  -v "$(pwd)/docs/architecture:/usr/local/structurizr" \
  structurizr/structurizr validate \
  -w /usr/local/structurizr/workspace.dsl

docker run --rm \
  -v "$(pwd)/docs/architecture:/usr/local/structurizr" \
  structurizr/structurizr export \
  -w /usr/local/structurizr/workspace.dsl \
  -f mermaid \
  -o /usr/local/structurizr/export
```

Relationships are declared at the most specific level — a person or container — and
Structurizr rolls them up to the parent software system in system-context views. Do not also
declare the parent-level relationship with the same description and technology: newer
Structurizr versions reject that as a duplicate, and older ones draw two arrows.

On PowerShell, replace `$(pwd)` with `${PWD}` and use backslashes in the mount path.
Exports are build artefacts, not part of the model of record.

---

## 4. System boundary

**People**

| Person | Role |
| --- | --- |
| Portfolio Owner | Connects read-only accounts and wallets, tracks P&L, builds portfolios, reviews valuation, drift, performance and snapshots, records order intents and trades, reads briefs and manages billing |
| Admin | Holds the `app_metadata.role` admin claim; manages registries, market-data sources, users, entitlements, jobs, audit queries, feature flags and privacy requests |
| Support Agent | Reads account metadata and tickets under time-bound, reason-recorded, audited access; never sees credentials or portfolio contents |

**External systems**

| System | What it provides |
| --- | --- |
| Exchange APIs (OKX, Binance, Coinbase, Kraken, Bybit) | Read-only balances, ticker prices, candle histories and instrument/pair metadata via HMAC-signed REST, subject to regional availability |
| EVM JSON-RPC Providers | Native and ERC-20 balances and token metadata, primary/fallback per chain |
| Solana RPC Provider | SOL and SPL balances and mint metadata |
| Bitcoin & Tron RPC / Indexers | Bitcoin address and xpub balances, Tron and TRC-20 balances including USDT |
| WalletConnect Relay | Read-only wallet sessions from the SPA without exposing keys |
| Market Data & FX Providers | Secondary prices, long-tail quotes, cross-checks and historical FX |
| Benchmark Index Provider | Licensed BTC, ETH or broad-index series for performance comparison |
| Transactional Email Provider | Consented alerts, briefs, billing and account email |
| Web Push Provider | Consented web-push notifications to installed PWAs |
| Merchant of Record | Global checkout, tax calculation and remittance, invoices, dunning and refunds |
| Help Desk & Status Page | Ticketing, knowledge base, public status page and incident communication |
| Operational Alerting Channel | Consumer of missed-run, failure, error-rate and stale-data alerts |
| Supabase Auth | Identity provider: JWTs, JWKS, MFA, passkeys, custom access-token hook, signed webhooks, Admin API |
| Supabase Postgres | One schema + role per service, no cross-schema grants, Supavisor pooler |
| Supabase Storage | Private buckets for audit exports, retention archives, user exports and PDF briefs |
| Supabase Vault | pgsodium secrets: exchange credentials, provider keys, service token, merchant-of-record credentials, email/push keys, per-subject encryption keys |
| Prometheus/Alertmanager | Alert rules including the scheduler's dead-man check |

Observability is **not** an external dependency: the Observability Stack (Prometheus, Loki,
Tempo, Grafana) is an internal container deployed on the same Swarm cluster (section 5).

---

## 5. Containers

| Container | Owns | Schema |
| --- | --- | --- |
| React Web Client (SPA) | UI for onboarding, dashboard, accounts, portfolios, valuation, P&L, snapshots, orders, briefs, notifications, settings and billing; Supabase Auth flows in-browser; WalletConnect read-only sessions; tokens in memory only | — |
| Support Console | Least-privilege support UI for account metadata and tickets; reason-recorded, time-bound and audited | — |
| API Gateway (BFF) | JWT validation, principal resolution, deny-by-default routing, admin/support gating, entitlement checks, rate limits, correlation IDs, SSE proxy. Stateless | — |
| Identity Service | Internal users keyed by `auth.users.id`, roles from `app_metadata.role`, status, locale, base currency, timezone, consent, support-access grants, deletion and retention | `identity` |
| Account Service | Read-only exchange, on-chain, WalletConnect, watch-only and CSV accounts; verified credentials; holdings sync; spam/dust filtering; DeFi position labelling; regional coverage; quotas | `account` |
| Asset Registry Service | Canonical asset identity, chains, token registries, price-source registry and quarantine for unmapped observations | `registry` |
| Market Data Service | Multi-source prices, candles, pair metadata, FX and benchmarks with priority, fallback, licensing and freshness | `market` |
| Portfolio Service | Portfolios and shared-account membership, targets, guardrails, base-currency valuation, drift, immutable snapshots, order intents, trades, performance and attribution | `portfolio` |
| Ledger Service | Append-only transactions, imports, dedupe, transfer pairing, cost basis, net invested, realized/unrealized P&L and reconciliation | `ledger` |
| Reporting Service | Reporting preferences and daily/weekly brief composition; in-app, email and PDF; idempotent per owner/portfolio/period | `reporting` |
| Notification Service | Alert catalogue, preferences, quiet hours, digests, dedupe, delivery log and retries; email and web push dispatch | `notification` |
| Billing Service | Plans, subscriptions, entitlements, quotas, invoices, dunning, refunds and merchant-of-record integration | `billing` |
| Analytics Service | Consent-aware event ingestion, activation/retention/conversion aggregates, feature flags and experiment exposure | `analytics` |
| Job Orchestration Service | Schedules, run triggering, a single OrchestratorLease, run records, missed-run detection, bounded retries/backfill, alerts ≤5 min, heartbeat | `job_orchestration` |
| Audit Service | Append-only audit events (insert-only role), admin queries, support-access records, privacy exports, erasure by crypto-shredding, partition retention | `audit` |
| Observability Stack | Logs, metrics, traces, dashboards and alert evaluation, self-hosted on Swarm: Loki, Prometheus, Tempo, Grafana | — |

All backend services are Rust (Axum, sqlx); the SPA and support console are React 18 +
TypeScript 5.

---

## 6. Communication rules

1. The SPA calls **only** the API Gateway, plus Supabase Auth directly for sign-up/sign-in and
   WalletConnect for read-only wallet sessions (the browser-to-external exceptions).
2. The gateway validates the Supabase JWT (JWKS, in-memory cache), resolves the principal via
   Identity (short-TTL cache), checks entitlements and quotas via Billing for gated actions,
   then **forwards the user's JWT unchanged** to the owning service over the private network.
   Every service validates the JWT itself as defence in depth and enforces owner scoping.
3. Service-to-service calls are plain internal HTTP: synchronous where the caller needs an
   answer (lookups, snapshots, brief data, P&L), fire-and-forget with bounded retries for
   notifications (holdings replacement, alerts, audit events). Idempotency keys or version
   checks make retries safe.
4. Scheduled runs are triggered by the Job Orchestration Service calling each service's
   internal run endpoint with the shared service token from Vault; the HTTP response is the
   run outcome.
5. No message queues. Any missed notification is repaired by the scheduled reconciliation
   jobs (portfolio ↔ account ↔ ledger ↔ registry ↔ market comparisons).
6. Security-relevant audit events (access denied, rate limited, support access) are written
   synchronously; domain audit events are best-effort with local logging as fallback.
7. Analytics ingestion checks consent through Identity before storing any event; no PII,
   addresses, credentials, or tokens are sent.
8. Notification delivery is owned by the Notification Service; other services queue alerts
   over internal HTTP and never call email or push providers directly.
9. Support reads go through the gateway and Identity with a recorded reason, a time bound and
   an audit event; the Support Console never calls portfolio or credential endpoints.

---

## 7. Data ownership

- One schema and one Postgres role per service: `identity`, `account`, `registry`, `market`,
  `portfolio`, `ledger`, `reporting`, `notification`, `billing`, `analytics`, `job`, `audit`.
  No cross-schema grants. A service's data is read by nobody else — other services ask that
  service over HTTP.
- The audit schema is appended through an insert-only role; retention uses a separate
  audited privileged role.
- Secrets live only in Supabase Vault: exchange credentials, RPC and market-data provider
  keys, the internal service token, merchant-of-record credentials, email/push provider keys,
  and per-subject crypto-shredding keys.
- The Support Console owns no schema; it reads account metadata through Identity and writes
  access records through Audit.
- Analytics owns consent-aware aggregates and experiment exposure, not a shadow user database;
  consent state is owned by Identity.

---

## 8. Jobs and scheduling

The Job Orchestration Service ticks itself from an in-process loop (30–90 s), elected by a
single OrchestratorLease row (30 s TTL). It records one row per attempt, calls the owning
service's run endpoint with a per-window idempotency key, and retries within a bound. Missed
runs inside a 24-hour lookback are recovered oldest-first; backfills are bounded to seven days;
a Prometheus rule on the heartbeat age (180 s, `absent()` included) pages within five minutes.
Nightly run retention is an operator systemd timer running the migrate CLI.

Scheduled work: account syncs, transaction sync, catalogue sync and reconciliation, price,
FX and benchmark collection, valuation, rolling timezone-aware EOD snapshots, performance,
brief generation, notification dispatch and digests, billing renewal and dunning, analytics
aggregation and flag expiry, reconciliation, and retention (identity, reporting, audit,
analytics).

---

## 9. Views

| Key | Name | Level |
| --- | --- | --- |
| `SystemContext` | System Context | C4 level 1 |
| `Containers` | Containers | C4 level 2 |
| `ProductionDeployment` | Production Deployment | Deployment (supplementary) |
| `AccountSyncToValuation` | Account Sync to Valuation | Dynamic |
| `TransactionToPnl` | Transaction Import to P&L | Dynamic |
| `DriftAlertToNotification` | Drift Alert to Notification | Dynamic |
| `OnboardingToFirstValue` | Onboarding to First Value | Dynamic |
| `BillingToEntitlement` | Billing to Entitlement | Dynamic |
| `SupportAccess` | Support Access | Dynamic |
| `OrderToReporting` | Order to Reporting | Dynamic |

No service has a component view — component structure is a code concern. The deployment view
covers `Production` only; Supabase services are external systems, not deployment nodes.

---

## 10. Decisions and costs

| Decision | Cost accepted |
| --- | --- |
| Keep 12 backend Rust services + the gateway (rather than collapsing to a monolith) | 13 deployables, 13 health probes, 13 pipelines — accepted to preserve the BRD service boundaries and independent scaling of sync, price, ledger, notification, billing and analytics work |
| Separate Ledger Service for transactions, cost basis and P&L | One more service and a reconciliation protocol with Account and Portfolio; accepted because transaction history is the retail value loop and must evolve independently |
| Separate Notification Service | Email/push provider integrations, deliverability, consent and preference state; accepted because alerts are the retention loop and must not be coupled to reporting |
| Separate Billing Service | Merchant-of-record integration, tax/consumer compliance, entitlement and quota state; accepted because plan enforcement is server-side and cross-cutting |
| Separate Analytics Service | Event taxonomy, consent review, storage and privacy controls; accepted to keep product analytics out of operational services |
| Separate Support Console | Another UI and access-audit path; accepted to keep support least-privilege and away from credentials |
| No message queues; notifications and analytics are HTTP + retry + reconciliation | Each path re-implements bounded retry; acceptable at launch volume, revisit if fan-out grows |
| Forwarded Supabase JWT instead of signed internal assertions | Every service needs JWKS access and clock-skew tolerance; removes an entire key-management subsystem |
| In-memory gateway caches and rate limits | Per-instance state; multi-replica gateways rate-limit per replica, revisit before horizontal scale |
| Append-only role instead of hash-chained audit | Tamper-evidence relies on Postgres grants, not cryptographic chaining; satisfies BRD-012's append-only requirement |
| Crypto-shredding kept for erasure | Per-subject Vault keys must be provisioned with the data they protect |
| Multi-source prices, FX and benchmarks | Licensing, rate limits, source health, attribution and reconciliation; accepted because a single exchange cannot serve global retail |
| Five exchange, four chain-family and wallet adapters at launch | Venue-specific quirks, regional availability and API terms; accepted as the global retail table stakes |
| Merchant of record for subscriptions | Provider fees and some lock-in; accepted to avoid operating global subscription tax |
| Transactional email and web push providers | Deliverability, consent and provider incidents; accepted because in-app-only alerts break the retention loop |
| Docker Swarm as the deployment platform (instead of a managed container platform) | The team operates the cluster: nodes, upgrades, TLS and the routing mesh; accepted for a small global team |
| Self-hosted Swarm observability stack (Loki/Prometheus/Tempo/Grafana) instead of Supabase Logs & Metrics | One more stack to deploy, upgrade and back up; Supabase is scoped to the database platform; dashboards, alert rules and retention are fully under product control |
| Single Swarm cluster with CDN and multi-AZ at launch | No active-active multi-region; accepted for launch, revisit when latency or availability targets require it |
| Consent-aware analytics only | Lower acquisition insight than ad-tech tracking; accepted for privacy and regulatory safety |

---

## 11. BRD coverage

| BRD | Where it is satisfied |
| --- | --- |
| BRD-001 Product vision and scope | Whole model; global retail aggregation, P&L, valuation, orders, briefs, billing and support |
| BRD-002 Users, roles and permissions | Identity Service (roles from `app_metadata.role`), gateway deny-by-default + admin/support gating, Support Console |
| BRD-003 Authentication and sessions | Supabase Auth + SPA (PKCE, MFA, passkeys, magic link), Identity webhooks, session revocation |
| BRD-004 Account management and coverage | Account Service (verified read-only credentials, multi-venue and multi-chain adapters, WalletConnect, xpub, CSV, spam filtering, quotas) |
| BRD-005 Asset catalog and market data | Asset Registry Service, Market Data Service (multi-source prices, FX, benchmarks, licensing, freshness) |
| BRD-006 Portfolio management | Portfolio Service (lifecycle, shared-account membership, targets, guardrails, templates, rebalancing proposal) |
| BRD-007 Valuation, allocation and snapshots | Portfolio Service (base-currency valuation, unpriced labelling, immutable snapshots, drift, attribution) |
| BRD-008 Orders and trades | Portfolio Service + Ledger Service (order intents, trades, immediate allocation effect, reconciliation) |
| BRD-009 Reporting | Reporting Service (in-app, email and PDF briefs; localization and timezone) |
| BRD-010 Admin configuration | Registry, source, user, entitlement and flag admin through the gateway and owning services; audit |
| BRD-011 Background jobs | Job Orchestration Service, section 8 |
| BRD-012 Security, compliance and NFRs | Sections 6–8: JWT everywhere, Vault, MFA, append-only audit, privacy exports/erasure, retention, alerts, 99.9% target |
| BRD-013 Transactions, cost basis and P&L | Ledger Service (imports, dedupe, cost basis, P&L, reconciliation) |
| BRD-014 Onboarding, education and first-run | SPA + Account/Portfolio APIs + Analytics activation events |
| BRD-015 Notifications and alert preferences | Notification Service (catalogue, preferences, quiet hours, dedupe, email/push dispatch) |
| BRD-016 Billing, entitlements and quotas | Billing Service + gateway entitlement checks + merchant of record |
| BRD-017 Privacy, data rights and export | Identity, Audit and retention jobs; Storage exports; processor and residency records |
| BRD-018 Localization, multi-currency and accessibility | SPA locale and currency preferences; Identity preferences; Market Data FX series; accessible components |
| BRD-019 Support, service status and incident communication | Support Console, Audit support-access records, Help Desk & Status Page, operational alerts |
| BRD-020 Performance, benchmarks and attribution | Portfolio Service (TWR/MWR, contribution, attribution) + Market Data benchmarks + Ledger transactions |
| BRD-021 Product analytics, experimentation and measurement | Analytics Service (consent-aware events, metric aggregates, feature flags, experiment exposure) |

---

## 12. What to check after rendering

- Every container in the `Containers` view has a justified owner and data decision: each
  backend service owns one Supabase Postgres schema and role; the gateway, support console and
  observability stack own none.
- No relationship implies a queue, mTLS, a signed assertion, Notion or a single price source;
  if one appears, the model has drifted from sections 2 and 6.
- The dynamic views complete without a hop through Supabase Queues (it no longer exists).
- Ledger, Notification, Billing and Analytics each have a schema and no cross-schema grants.
- Entitlement checks appear on the gateway and on the owning service for quota-sensitive work.
- Support access is read-only, time-bound, reason-recorded and audited; no credential path
  reaches the Support Console.
- Analytics ingestion checks consent through Identity; no PII or secrets appear in events.
- Every service emits logs, metrics and traces to the Observability Stack; nothing logs
  credentials or tokens.
- Validate with the Docker-based CLI (section 3) before review; if validation fails, state it
  in the pull request rather than merging an unrenderable model.

---

## 13. Implementation status

The C4 model describes the **target architecture for BRD-001 … BRD-021**. The repository
currently contains the business requirements, the architecture baseline and the design system;
the Rust services, SPA and infrastructure code are the next construction phase. When code is
added, this README and `workspace.dsl` must change together: a new container, schema, external
system or relationship is an architecture change, not an implementation detail.
