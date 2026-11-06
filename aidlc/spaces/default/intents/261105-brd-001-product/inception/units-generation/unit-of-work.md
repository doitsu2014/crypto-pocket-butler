# Units of Work — Crypto Pocket Butler

Thirteen units, one per component from the domain design (`domain-design/components.md`), plus a
first walking-skeleton unit that proves the end-to-end path before feature breadth. The units
deliver the committed scope in `requirements-analysis/requirements.md` and the stories in
`user-stories/stories.md`, within the boundaries recorded in `domain-design/decisions.md`.
Boundary strategy: one unit per component, coarse granularity, strict topology, independent
deployment per service.

| Unit ID | Directory | Name | Kind | Deployment | Complexity |
|---|---|---|---|---|---|
| U1 | `u1-walking-skeleton` | Walking Skeleton and Asset Registry | service | Standalone (gateway + registry) | XL |
| U2 | `u2-web-app` | Web App (SPA and PWA) | ui | Standalone (static SPA) | L |
| U3 | `u3-identity` | Identity | service | Standalone | M |
| U4 | `u4-account` | Account | service | Standalone | L |
| U5 | `u5-market-data` | Market Data | service | Standalone | M |
| U6 | `u6-portfolio` | Portfolio | service | Standalone | L |
| U7 | `u7-ledger` | Ledger | service | Standalone | L |
| U8 | `u8-reporting` | Reporting | service | Standalone | M |
| U9 | `u9-notification` | Notification | service | Standalone | M |
| U10 | `u10-analytics` | Analytics | service | Standalone | S |
| U11 | `u11-job-orchestration` | Job Orchestration | service | Standalone | M |
| U12 | `u12-audit` | Audit | service | Standalone | S |
| U13 | `u13-support-console` | Support Console | ui | Standalone (static app) | S |

## Unit responsibilities

### U1 — Walking Skeleton and Asset Registry

The thinnest end-to-end slice: the SPA shell calls the API Gateway; the gateway validates the
Supabase JWT and forwards it to the Asset Registry service; the registry reads and writes its own
Postgres schema through sqlx; the whole path runs on the Docker Swarm with health and readiness
endpoints and logs, metrics, and traces in the observability stack. The unit also delivers the
full Asset Registry capability: canonical assets, chains, token contracts, decimals, price
sources, and the unmapped-observation quarantine.

Owns: the gateway shell, the registry service and schema, the SPA shell, the Swarm deployment
manifests, and the health and observability wiring. Implements US1.2 (first valuation path) and
US9.2 (registry administration).

### U2 — Web App

The full SPA and PWA journeys: onboarding, connect, dashboard and portfolio, asset detail, drift
and order intent, briefs, settings and privacy, and support. Renders data from the gateway; holds
no business math; session tokens in memory only; English and Vietnamese; WCAG 2.2 AA.

### U3 — Identity

Internal users keyed by Supabase Auth, roles from `app_metadata.role`, MFA requirement for users
with exchange credentials, session listing and revocation, consent and preference state, the
jurisdiction and age gate, support-access grants, export and deletion orchestration, and
crypto-shredding keys.

### U4 — Account

Read-only exchange and wallet accounts for Binance, OKX, EVM chains, BNB Smart Chain, and Solana;
WalletConnect read-only sessions; CSV import; credential verification; holdings sync; sync
status; spam and dust filtering.

### U5 — Market Data

Multi-source prices with deterministic priority and fallback, FX rates, freshness and staleness,
provenance, and the unpriced result.

### U6 — Portfolio

Portfolios and account membership, targets and guardrails, base-currency valuation, drift in
percentage points, immutable snapshots, order intents, recorded trades, and immediate allocation
effect.

### U7 — Ledger

Append-only transactions, deduplication across sources, import batches, transfer pairing, cost
basis, net invested, realized and unrealized P&L, coverage labels, and corrections.

### U8 — Reporting

Daily and weekly briefs composed from valuation, snapshots, and P&L; in-app and email; explicit
failure with a reason; every figure carries an as-of time and data status.

### U9 — Notification

Alert catalogue, per-type preferences, mandatory security and credential email, dedupe, bounded
retries, delivery log, and in-app and email dispatch.

### U10 — Analytics

Consent-aware, PII-free event ingestion, activation and retention aggregates, and feature flags
with an owner, an expiry, and a rollout percentage.

### U11 — Job Orchestration

Schedules, single-flight locks, run records, bounded retries and backfills, missed-run detection,
and the heartbeat that drives the dead-man check.

### U12 — Audit

Append-only audit events through an insert-only role, support-access records, admin queries,
exports, and retention under a separate audited privileged role.

### U13 — Support Console

Least-privilege support UI for account metadata and tickets, under a recorded reason, a time
bound, and a second approver; never reads credentials or portfolio contents.

## Implementation notes and constraints

- Every backend unit is a Rust Cargo workspace of `domain`, `application`, `infrastructure`, and
  `http` crates, with dependencies pointing inward only.
- Every service unit owns one Supabase Postgres schema and one Postgres role; no cross-schema
  grants; no Row Level Security.
- Every service unit validates the Supabase JWT itself and enforces ownership on user-owned
  resources.
- Every unit follows TDD: a failing test first for all production code, 80% line coverage per
  Rust crate.
- Every deployment carries a health check, a rollback path, and versioned up/down sqlx
  migrations.
- No message queues; internal HTTP with bounded retries and idempotency keys.
- No Billing unit: plans, entitlements, and quotas are out of this release.
- U1 is the walking skeleton; it must run end to end before later units are built.
