# Bolt Plan — Crypto Pocket Butler

A **Bolt** is one build pass over a piece of the work — here, one Unit of Work — ending in
something that runs, with a Definition of Done and a confidence hypothesis (what shipping it
will prove). The first Bolt is the **walking skeleton**: the smallest working end-to-end slice
that proves the architecture connects. After it ships, Bolts run in parallel wherever the
dependency graph allows.

Ordering rule: walking skeleton first, then a WSJF score — (value + urgency + risk reduction) ÷
size — subject to the unit dependency graph. The scores and the reasoning are in
`risk-and-sequencing-rationale.md`. Each Unit's scope comes from
`units-generation/unit-of-work.md` and the components in `domain-design/components.md`; the
stories it delivers come from `requirements-analysis/requirements.md` and
`user-stories/stories.md`.

## Bolt sequence

| Bolt | Unit | Parallel with | Score |
|---|---|---|---|
| B1 | U1 `u1-walking-skeleton` | — | 2.23 (skeleton, forced first) |
| B2 | U12 `u12-audit` | — | 5.67 |
| B3 | U3 `u3-identity` | B5, B6 | 4.60 |
| B4 | U13 `u13-support-console` | B5, B6, B7 | 3.67 |
| B5 | U5 `u5-market-data` | B3, B4, B6 | 4.60 |
| B6 | U4 `u4-account` | B3, B4, B5 | 3.13 |
| B7 | U10 `u10-analytics` | B4, B9 | 3.33 |
| B8 | U9 `u9-notification` | B9 | 3.20 |
| B9 | U7 `u7-ledger` | B7, B8 | 3.13 |
| B10 | U6 `u6-portfolio` | — | 3.38 |
| B11 | U8 `u8-reporting` | — | 3.00 |
| B12 | U11 `u11-job-orchestration` | — | 3.20 |
| B13 | U2 `u2-web-app` | — | 2.88 |

## Bolt details

### B1 — Walking skeleton (U1)

- **Units:** U1 `u1-walking-skeleton`.
- **Walking skeleton:** yes. It proves the SPA shell → API Gateway → Asset Registry → its own
  Postgres schema → Docker Swarm path, with the Supabase JWT validated at the gateway and again
  in the service, health and readiness endpoints, and logs, metrics, and traces visible.
- **Definition of Done:** the SPA shell reaches the gateway; the gateway validates a real
  Supabase JWT and forwards it; the Asset Registry service reads and writes its own schema
  through sqlx; the whole path runs on the Swarm; the health check gates the deployment; a
  request produces a trace and a structured log.
- **Confidence hypothesis:** shipping this proves the architecture hangs together end to end —
  the JWT path, schema and role isolation, the deployment model, and the observability wiring
  all work before feature breadth.
- **Expected demo:** open the SPA shell, sign in, and see one asset resolved and served by the
  registry through the gateway.

### B2 — Audit (U12)

- **Units:** U12 `u12-audit`.
- **Definition of Done:** append-only audit events accepted through an insert-only role, admin
  queries answered, and support-access records stored; an event can carry actor, action, target,
  time, context, and outcome.
- **Confidence hypothesis:** the compliance boundary works before any domain event depends on it.
- **Expected demo:** append an event, query it, and show that update and delete are refused.

### B3 — Identity (U3)

- **Definition of Done:** users upserted from the validated JWT and the signed webhook; roles
  from `app_metadata.role`; MFA required with exchange credentials; sessions listed and revoked;
  consent, locale, currency, timezone, jurisdiction and age gate, support-access grants, export
  and deletion orchestration.
- **Confidence hypothesis:** the identity and privacy foundation supports every later unit.
- **Expected demo:** sign in, list sessions, revoke one, and show the jurisdiction gate blocking
  a blocked market before data collection.

### B4 — Support Console (U13)

- **Definition of Done:** account metadata read under a recorded reason, a time bound, and a
  second approver; no credential or portfolio content reachable; every access audited.
- **Confidence hypothesis:** least-privilege support access works without widening data exposure.
- **Expected demo:** request a grant, have it approved, read one account's metadata, and show
  the audit record.

### B5 — Market data (U5)

- **Definition of Done:** multi-source prices with priority and fallback, FX rates, provenance
  and freshness, and an explicit unpriced result for an asset with no sourceable price.
- **Confidence hypothesis:** trustworthy valuation inputs exist, including the unpriced path.
- **Expected demo:** show a priced asset with source and timestamp, then an unpriced asset that
  is excluded and labelled.

### B6 — Account (U4)

- **Definition of Done:** Binance and OKX read-only accounts with verified permissions, wallet
  accounts for EVM, BNB Smart Chain, and Solana, WalletConnect read-only sessions, CSV import,
  holdings sync, sync status, and a manual re-sync.
- **Confidence hypothesis:** real venue and chain data can be gathered without withdrawal risk.
- **Expected demo:** add a read-only key, sync, and show the holdings; then reject a key with
  trading permission.

### B7 — Analytics (U10)

- **Definition of Done:** consent-aware, PII-free event ingestion, activation and retention
  aggregates, and feature flags with an owner, an expiry, and a rollout percentage.
- **Confidence hypothesis:** measurement works without weakening privacy.
- **Expected demo:** an event is refused without consent and accepted with it; a flag without an
  owner is refused.

### B8 — Notification (U9)

- **Definition of Done:** per-type preferences, mandatory security and credential email, dedupe,
  bounded retries, delivery log, and in-app and email dispatch.
- **Confidence hypothesis:** alerts reach users reliably without duplicating them.
- **Expected demo:** raise one drift alert twice and show one delivery plus a dedupe record.

### B9 — Ledger (U7)

- **Definition of Done:** append-only transactions, dedupe across exchange sync and CSV, import
  batches, cost basis, net invested, realized and unrealized P&L, and a coverage label when
  history is missing.
- **Confidence hypothesis:** the retail value loop — "am I up or down?" — produces correct
  numbers with no fabricated cost basis.
- **Expected demo:** import history, show P&L that reconciles to the ledger, then show an
  unavailable P&L with a coverage label.

### B10 — Portfolio (U6)

- **Definition of Done:** portfolios and account membership, targets and guardrails, base-currency
  valuation, drift in percentage points, immutable snapshots, order intents, recorded trades
  with immediate allocation effect.
- **Confidence hypothesis:** valuation, drift, and action behave correctly against real data.
- **Expected demo:** set a target, show drift, record a trade, and show allocation update at once.

### B11 — Reporting (U8)

- **Definition of Done:** daily and weekly briefs composed from valuation, snapshots, and P&L;
  in-app and email; explicit failure with a reason; every figure with an as-of time and status.
- **Confidence hypothesis:** manual reporting is removed without hiding failures.
- **Expected demo:** generate a brief, read it in-app, and show an explicitly failed brief.

### B12 — Job orchestration (U11)

- **Definition of Done:** schedules, single-flight locks, run records, bounded retries and
  backfills, missed-run detection within five minutes, and the heartbeat for the dead-man check.
- **Confidence hypothesis:** scheduled work is observable and self-healing.
- **Expected demo:** stop the scheduler and show the missed-run alert within five minutes.

### B13 — Web app (U2)

- **Definition of Done:** the full SPA and PWA journeys rendered from the gateway, English and
  Vietnamese, WCAG 2.2 AA on core flows, and session tokens only in memory.
- **Confidence hypothesis:** the customer journeys work end to end against real services.
- **Expected demo:** complete onboarding to first valuation, then drift and an order intent, in
  both languages.

## Definition of Done (all Bolts)

- Every story on the Unit's story-map rows has passing acceptance tests.
- TDD followed: tests committed with the code they cover; 80% line coverage per Rust crate.
- The crate boundaries hold, `cargo fmt --check` and `cargo clippy -D warnings` pass, and
  ESLint and Prettier pass for TypeScript.
- Migrations are versioned up/down pairs; the health check gates the deployment; a rollback
  path is documented.
- The change is reviewed by a human and links to its Unit id.
