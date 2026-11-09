# Code Summary — u6-portfolio

## What was built

The Portfolio service as a five-package Cargo workspace under `services/portfolio/`, mirroring
the account and ledger services: four Clean Architecture crates plus a thin composition package
that owns the three binaries.

| Package | Responsibility |
|---|---|
| `portfolio-domain` | `Portfolio`, `PortfolioAccount`, targets and guardrail crossing state, the pure valuation math with per-line provenance and unpriced handling, immutable `ValuationSnapshot`, advisory `OrderIntent`, recorded trades with their canonical key, and the service allow-list |
| `portfolio-application` | Portfolio list/create/open and account moves, the read path (fresh snapshot ≤ 60 s or a C5+C6 compute), target preview/save, intent preview/create/cancel, recorded trade → C7 + overlay, guardrail evaluation, the leased valuation run, and privacy export/anonymise |
| `portfolio-infrastructure` | sqlx repositories, C5/C6/C7/C9/C10 clients, the Identity preference read, JWKS validation, the service-token registry, the Postgres readiness probe, migrations, the clock, and identifier generation |
| `portfolio-http` | C1 routes, the shared C1/C8 snapshot route, C8 privacy routes, DTO mapping, RFC 7807 problems, correlation id, security headers, and Prometheus text metrics |
| `portfolio` | Composition root and three binaries: `portfolio-api` (8101), `portfolio-worker` (8102), `portfolio-migrate` |

### HTTP surface

| Method and path | Identity | Behaviour |
|---|---|---|
| `GET /v1/portfolios` | JWT | lists the caller's portfolios, creating the default one on demand (BR1.2) |
| `POST /v1/portfolios` | JWT | creates a portfolio; the base currency defaults from Identity or USD |
| `GET /v1/portfolios/{id}` | JWT | valuation with allocation, drift, provenance, unpriced labels, and degraded status |
| `PUT /v1/portfolios/{id}/accounts/{accountId}` | JWT | assigns or moves an account (BR1.3) |
| `PUT /v1/portfolios/{id}/targets` | JWT | previews with `?preview=true` (sum and difference from 100), otherwise saves targets and guardrails |
| `GET /v1/portfolios/{id}/snapshots` | JWT **or** service token | bounded history (default 100, max 500) with `from`, `cursor` and the `x-next-cursor` / `x-has-more` response headers; no reproduction |
| `GET /v1/portfolios/{id}/snapshots/{snapshotId}` | JWT **or** service token | reproduces one snapshot on demand and reports `verified` |
| `POST /v1/portfolios/{id}/order-intents` | JWT | previews the projection with `?preview=true`, otherwise records a pending intent |
| `POST /v1/portfolios/{id}/order-intents/{intentId}/cancel` | JWT | cancels a pending intent; it stays visible |
| `POST /v1/portfolios/{id}/trades` | JWT | records a trade on C7, writes C10 out of band, applies an overlay, then re-evaluates guardrails |
| `GET /v1/internal/privacy/export` | service token (`privacy_export`) | Identity's export of portfolios, membership, targets, guardrails, snapshots, overlays, intents, and trades |
| `POST /v1/internal/privacy/anonymise` | service token (`privacy_anonymise`) | anonymises portfolio names once per idempotency key |
| `GET /v1/internal/smoke` | service token (`smoke-reader`) | release smoke: confirms the API is Portfolio and answers |
| `POST /v1/internal/run/portfolio-valuation` | service token (`run_valuation`) | worker C11 run; 409 when another run holds the lease |
| `GET /healthz`, `/readyz`, `/metrics` | public | readiness checks only Postgres (C5/C6 outages must not fail it) |

### Database

`services/portfolio/migrations/0001_init.up.sql` creates `portfolios`, `portfolio_accounts`,
`target_allocations`, `guardrails` (with the crossing-state shape check), `valuation_snapshots`
(JSONB inputs/lines, a 64-hex digest, and `(portfolio_id, hour_bucket)` uniqueness),
`trade_overlays`, `order_intents`, `recorded_trades`, `valuation_runs` (single-flight partial
unique index), `service_token_registry`, and `privacy_anonymisations`, with the performance
indexes. `portfolio_svc` has `SELECT, INSERT` only on `valuation_snapshots`; the down migration
refuses to drop a schema that holds portfolio rows.

## Key behaviours

- **Owner scope**: every read and write checks the portfolio's `user_id` (BR1.1); a non-owner gets
  a 403 problem, never data.
- **Valuation**: quantity × price with `rust_decimal`, FX conversion with provenance, allocation
  and drift in percentage points over the priced total, unpriced assets excluded and counted
  (BR2.1–BR2.3, BR3.1). C5 unavailable → `partial` + `holdingsUnavailable` and no fabricated
  lines; C6 unavailable → unpriced lines; only Postgres failure is a 503.
- **Snapshots**: frozen inputs (holdings, quotes, FX, targets) hashed with SHA-256 and stored with
  the computed lines; the read path verifies the digest and reproduction, and a mismatch is flagged
  and recomputed rather than served (BR4.1–BR4.2). History is bounded and does not reproduce every
  row; the per-snapshot route reproduces on demand. An overlay recorded after the snapshot's
  `as_of` is applied to the frozen holdings before serving.
- **Intents**: advisory only; a preview projects allocation, drift, and a sell shortfall, and a
  cancelled intent stays visible (BR5.1–BR5.3).
- **Recorded trades**: C7 with a client idempotency key, a C10 audit event out of band, and a
  `TradeOverlay` applied to every later read so allocation and drift move immediately; the run
  removes an overlay once C5 reports a transaction with the same canonical key (BR6.1–BR6.2).
- **Guardrails**: crossing state on the guardrail row; one C9 alert per crossing, the same key
  reused while outside, cleared on return inside; a failed alert leaves the state inside so the
  next run retries it (BR7.1).
- **Run**: a 900 s lease renewed every 60 s, 500 portfolios per page and 4 in flight, one snapshot
  per portfolio per hour, and overlay reconciliation (C11).

## Tests

| Suite | Tests |
|---|---|
| `portfolio-domain/tests/domain.rs` | 88 |
| `portfolio-application/tests/application.rs` | 66 |
| `portfolio-infrastructure/tests/adapters.rs` | 12 |
| `portfolio-http/tests/dto.rs` + `routes.rs` | 32 |
| `portfolio-infrastructure/tests/repositories.rs` | 6 integration tests, `#[ignore]` without `TEST_DATABASE_URL` |
| **Executable total** | **198 passing** |

Exact command: `cargo test -p portfolio-domain -p portfolio-application -p portfolio-infrastructure -p portfolio-http`.

## Review fixes

- **Erasure (Critical)**: `portfolios.user_id` and `recorded_trades.user_id` are replaced with a
  random UUIDv4 opaque reference; portfolio names are anonymised; `privacy_anonymisations` stores
  only that reference and is keyed by the idempotency key, never the original user id. The
  migration narrows `portfolio_svc` to column-scoped `UPDATE (… user_id …)` on portfolios and
  `UPDATE (user_id)` on recorded trades. `privacy_anonymise_erases_the_export_by_the_original_id`
  proves a post-erasure export by the original id returns nothing.
- **Read freshness**: a snapshot served inside the 60 s window has any overlay applied after its
  `as_of` added to its frozen holdings and reproduced in memory, so AC5.4.1 holds on reload without
  a C5/C6 call (`reading_a_valuation_applies_a_post_snapshot_overlay`).
- **Preview path**: intent projection reads the latest snapshot's lines, even when stale, so it
  stays the 300 ms path (`an_intent_preview_uses_a_stale_snapshot_without_c5_or_c6`).
- **History**: bounded pages (default 100, max 500) with an `as_of` cursor returned in
  `x-next-cursor` / `x-has-more`; reproduction is on demand at
  `GET /v1/portfolios/{id}/snapshots/{snapshotId}` (`history_is_paginated_and_reproduction_is_on_demand`).
- **Run concurrency**: four batches of 500 portfolios run in parallel, four portfolios in flight
  per batch; a failed portfolio is counted and the run continues
  (`the_run_records_a_failed_portfolio_and_continues`).
- **Overlay reconciliation**: matches asset, side, quantity within 0.1%, and execution instant
  within 24 h, searching from the trade's `executedAt`; an overlay no sync reports for seven days
  is expired and counted (`portfolio_trade_overlays_expired_total`).
- **FX**: the quote currency comes from C6's Price; a missing currency or an unavailable FX pair
  makes the line unpriced rather than assuming the base
  (`a_quote_without_a_currency_is_unpriced`, `a_non_usd_base_converts_with_provenance`).
- **Minors**: the C7 body sends `priceSource`; the erasure audit uses the allow-listed
  `system.privacy.deletion` action and cannot fail a completed erasure; percentages are `Decimal`
  serialised as JSON numbers (not `f64`); a target save preserves an unchanged guardrail's crossing
  state; the drift alert payload carries only `portfolioId`, `assetId`, and `driftPoints`; the
  two-year snapshot prune runs through `portfolio-migrate --prune-snapshots` with the migrator DSN.

## Deviations and follow-ups

- **TDD red evidence**: the implementation and its tests landed in the same change, but the
  per-layer failing-test output was not separately captured because the whole service was
  compiled as one build. The tests are requirement-first and were written against the domain
  types before the boundary code was wired, but the plan's red/green diary is not reproducible
  step by step in this record.
- **Audit for a recorded trade** runs in a spawned task with the C10 adapter's 1/2/5 s retries, so
  the trade budget is untouched (NFR1.5); every other audit write (targets, intent cancellation,
  privacy) is awaited with the same bounded retries. That is a partial deviation from the security
  design's "retried out of band" wording for those events; a persistent failure is logged and the
  write is not retried again.
- **Identity preference read**: C3 exposes no service-token preferences read yet, so the adapter
  calls `GET /v1/users/{userId}/preferences` and falls back to USD on any non-success. The
  preference takes effect once Identity exposes that read.
- **Snapshot retention release gate**: `portfolio-migrate --prune-snapshots` (or
  `PRUNE_SNAPSHOTS=true`) deletes snapshots older than two years through the migrator/operator DSN.
  It is deliberately not part of the service release; the launch/release runbook must schedule it
  (for example monthly) and the release checklist records that step as a gate.
- **C7 trade idempotency**: Portfolio sends the `idempotency-key` header and dedupes locally by
  `(portfolio_id, idempotency_key)`; the current Ledger adapter does not itself dedupe a replayed
  key, so a Ledger-side replay would be a second transaction. Portfolio never retries a trade
  after a failure, which keeps the window closed in this unit.
- **Gateway routing**: `infra/swarm-stack.yml` now points `PORTFOLIO_URL` at
  `http://portfolio:8101` (the previous default was `:8093`); the gateway route table itself is
  u1's and was not changed.
- **Integration tests** (6) are ignored by default and were not executed because no disposable
  `TEST_DATABASE_URL` was available in this environment.
- **Coverage**: `cargo llvm-cov -p portfolio-<crate> --fail-under-lines 80` was not run in this
  environment (the tool was not available), so the 80% per-crate target is unverified here. The
  198 executable tests cover every public use case, the pure valuation/intent/guardrail math, the
  adapter mappings, and the HTTP boundary.
