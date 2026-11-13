# Business Overview — Crypto Pocket Butler

## Business Domain

Crypto Pocket Butler is a global retail **crypto portfolio aggregation, P&L and decision-support platform**. A retail user connects read-only exchange accounts, wallets, watch-only addresses and CSV exports; the platform consolidates holdings, values them in a base currency, computes allocation, drift, performance and cost-basis P&L, records order intents and trades, and delivers daily/weekly briefs, alerts and reports. A support tier can read account metadata under time-bound, reason-recorded grants; an admin tier manages registries, sources, users, jobs and flags.

The business rules are authored in 21 Business Requirements Documents under `docs/brds/` (BRD-001 … BRD-021). The architecture model of record is `docs/architecture/workspace.dsl`, explained by `docs/architecture/README.md`.

## Purpose

- Give a retail user one truthful, auditable view of a portfolio spread across venues and chains.
- Never custody assets, store private keys, seed phrases, or enable withdrawals: every exchange and wallet integration is read-only.
- Compute all financial maths server-side in decimal arithmetic, and label missing values rather than showing zero.
- Keep one service's data unreachable to another except through that service's HTTP API.
- Make every security-sensitive action auditable.

## Actors and Roles

| Actor | Source in code | Capabilities |
| --- | --- | --- |
| Portfolio Owner | `identity.domain.claims.Role::User` | Own accounts, portfolios, transactions, targets, snapshots, order intents, trades, preferences, exports, deletion |
| Admin | `Role::Admin` (`app_metadata.role = "admin"`) | Owner capabilities plus registry/source/user/job/flag administration and support-grant approval |
| Support | `Role::Support` (`app_metadata.role = "support"`) | Read-only, time-bound, reason-recorded access to account metadata and tickets; never credentials or portfolio contents |
| System / jobs | Service tokens | Scheduled runs over all users' data; never exposed interactively |
| Anonymous visitor | No token | Sign-in, public pages, the pre-sign-up gate |
| Analyst / Viewer | Not implemented | Post-MVP shared-portfolio read access |

Authorisation is deny-by-default. An Admin is additive over an Owner; Support is not an Owner. Role is read **only** from `app_metadata.role`; any other value is treated as a plain user (`services/identity/domain/src/claims.rs:19-31`).

## Key Functionality (implemented in code)

| Domain | Service | What the code does |
| --- | --- | --- |
| Identity, roles, consent, sessions, privacy | `services/identity` | Just-in-time user provisioning from validated claims; role/status refresh; preferences; device-session view; consent history; support grants; export and deletion requests; jurisdiction and sign-up gate; verified-contact read for Notification |
| Account ingestion | `services/account` | Read-only exchange, on-chain, WalletConnect, watch-only and CSV accounts; holdings sync; imports; verified credentials in Vault |
| Asset identity | `services/asset-registry` | Canonical assets, chains, token contracts, price sources, observations and quarantine |
| Prices, FX, benchmarks | `services/market-data` | Multi-source quotes, FX rates, source priority, freshness, tiered request budget |
| Portfolios and valuation | `services/portfolio` | Portfolios, membership, targets, guardrails, valuation, drift, immutable snapshots, order intents, trades |
| Transactions and P&L | `services/ledger` | Append-only transactions, corrections, transfer pairing, FIFO/average cost, realized/unrealized P&L, reconciliation |
| Reporting | `services/reporting` | Daily/weekly brief composition and delivery, idempotent per owner/portfolio/period |
| Notifications | `services/notification` | Alert catalogue, preferences, quiet hours, dedupe, delivery log, bounded retries |
| Analytics and flags | `services/analytics` | Consent-aware event ingestion, aggregates, feature flags, experiment exposure, retention/erasure |
| Scheduling | `services/job-orchestration` | Self-ticking scheduler with a single lease, run records, retries, missed-window recovery, backfills, heartbeat metrics |
| Audit | `services/audit` | Append-only audit store enforced by an insert-only role; queries, support records, privacy exports, retention |
| Edge | `services/gateway` | JWT validation, deny-by-default route table, admin gate, correlation ids, idempotency keys, bounded retries, in-memory rate limits |
| UI | `web/`, `support-console/` | Authenticated SPA and least-privilege support console |

## Declared but not implemented

| Area | Evidence |
| --- | --- |
| Billing, plans, subscriptions, entitlements, quotas, invoices, dunning (BRD-016) | No `services/billing/`; no entitlement/quota code; the C4 model still lists the container (`docs/architecture/workspace.dsl:54`) |
| Gateway principal resolution and entitlement checks | `docs/architecture/README.md` §6 describes them; `services/gateway/application/src/authorize.rs` does not |
| Gateway routing to non-registry services | `services/gateway/domain/src/route.rs:68-90` declares only `/v1/assets`, `/v1/admin/assets`, `/v1/admin/token-contracts`, `/v1/admin/price-sources`, `/v1/admin/observations/...` |
| `disabled` and `pending_deletion` user status (BRD-002 FR-USR-20) | Stored enum is `pending|active|deleted` (`services/identity/migrations/0001_init.up.sql:66`) |
| Shared-portfolio Analyst/Viewer roles (BRD-002 FR-USR-10, Won't) | Deliberately out of MVP scope |

## Business Rules Visible in Code

- Credentials, MFA factors and tokens stay in Supabase Auth; Identity stores only an internal record (`services/identity/domain/src/user.rs:1-5`).
- The Supabase subject is personal data: only a SHA-256 hash (indexed lookup) and a deterministic pgsodium ciphertext are stored (`services/identity/migrations/0001_init.up.sql:79-87`).
- Role and email are refreshed from the signed token on every resolution; a role change is audited (`services/identity/application/src/principal.rs:199-262`).
- A deleted account is denied even with a valid token (`services/identity/domain/src/user.rs:128-137`).
- Support grants require a distinct admin approver, a reason of 1–500 characters, and at most a 60-minute window (`services/identity/migrations/0001_init.up.sql:107-133`).
- The audit trail is insert-only for the application role; an appended event is never updated or deleted (`services/audit/migrations/0001_init.up.sql:399-403`).
- Quantities and values use `rust_decimal`, never binary floating point (workspace dependency `rust_decimal = "1"`).

## Traceability Notes

- BRD-002 (users, roles, permissions) is the active intent's subject; its implemented surface is the Identity service plus the gateway's admin gate.
- The C4 model is a design target: it declares 16 containers (13 Rust services including the gateway), while the code contains 12 service packages and no Billing service.
- `docs/brds/TRACEABILITY.md`, `DECISIONS.md` and `GLOBAL-READINESS.md` record requirement-level decisions; they are product documents, not code.
