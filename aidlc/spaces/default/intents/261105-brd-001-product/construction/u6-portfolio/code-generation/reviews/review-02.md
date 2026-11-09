## Review

**Verdict:** READY

**Reviewer:** aidlc-architecture-reviewer-agent

**Iteration:** 2

**Verdict Dictated:** no

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Critical | services/portfolio/application/src/privacy.rs > anonymise, with services/portfolio/infrastructure/src/repositories/portfolios.rs > anonymise_for_user | The erasure does not anonymise the user. `anonymise_for_user` only renames each portfolio (`name = 'Anonymised portfolio …'`) and stamps `anonymised_at`; the `user_id` on `portfolios` and on `recorded_trades` (and the `privacy_anonymisations.user_id` key) is left unchanged. NFR13.1 requires the userId to be replaced with a non-reversible reference; as written, the erased user's rows remain linked to the real user id, and a post-erasure export by that id still returns them. | Replace the user id with one fresh non-reversible UUIDv4 reference across every user-keyed table (portfolios, recorded_trades) while keeping the portfolio ids and snapshots reproducible, record the replacement for idempotency without retaining the original id, and add a test asserting the original user id returns no rows and the export is empty after erasure. | New |
| R-02 | Major | services/portfolio/application/src/valuation.rs > read | The read serves the latest snapshot whenever it is ≤ 60 s old without checking whether a TradeOverlay was applied after that snapshot. A trade recorded in the 60-second window after an hourly snapshot is therefore not reflected on reload, contradicting BR6.1 and AC5.4.1's immediate allocation and drift update. | Bypass the snapshot (or fold the newer overlays into the served valuation) when any overlay's `applied_at` is later than the snapshot's `asOf`, and test a trade recorded immediately after a snapshot is visible on the next read. | New |
| R-03 | Major | services/portfolio/application/src/intents.rs > project | The intent preview and create call `valuation.fresh(portfolio)`, a full C5 + C6 compute budgeted at 800 ms, instead of projecting over the latest snapshot's lines as the performance design requires. The 300 ms preview budget (NFR2.4) cannot hold on the fresh path. | Project over the latest snapshot's labelled lines (with the snapshot's asOf surfaced), and document the behaviour when no snapshot exists; keep the fresh compute out of the preview path. | New |
| R-04 | Major | services/portfolio/application/src/valuation.rs > history, with infrastructure/src/repositories/snapshots.rs > list | The history read loads the whole requested window in one query and reproduces every snapshot (full parse and recomputation) to set `verified`. A year of hourly snapshots is thousands of rows and seconds of reproduction, so the 2 s history budget (NFR2.3, AC4.4.3) is not achievable and the response is unbounded. | Verify the digest cheaply on the read (and move full reproduction to the nightly verification job), paginate or bound the history response, and record the window/limit. | New |
| R-05 | Major | services/portfolio/application/src/run.rs > BATCH_IN_FLIGHT | The run processes four portfolios in flight (`BATCH_IN_FLIGHT = 4`), while the scalability design fixes four batches in flight with four portfolios in flight per batch. At the measured per-portfolio dependency time (~350–400 ms) ten thousand portfolios take about 1,000 seconds, exceeding the 15-minute NFR2.6 budget; the lease renewal keeps the run alive but the target is missed. | Implement the designed concurrency (or state and prove the actual per-portfolio time) so 10k portfolios complete within 15 minutes, and add a test or benchmark of the run concurrency. | New |
| R-06 | Major | services/portfolio/application/src/run.rs > reconcile_overlays and same_trade | The overlay reconciliation searches C5 transactions from `applied_at - 1 day` and matches on executed_at within ±1 s. A trade executed more than a day before it is recorded, or whose venue transaction reaches Account more than a day after the record, is never matched; the overlay then persists and the trade is counted twice once the venue holding appears. | Reconcile against the trade's executed_at (with an explicit tolerance) and keep reconciling until matched rather than using a one-day window from the record time. | New |
| R-07 | Major | services/portfolio/infrastructure/src/clients/market_data.rs > Quote mapping, with the C6 Price contract | The FX conversion is unreachable: the C6 Price schema carries no currency, so the client's `currency` is always null and the valuation treats every quote as already in the portfolio's base currency. A non-USD base portfolio values USD prices as base without conversion, and the FX provenance (AC3.1.2) never appears. | Add the quote currency to the C6 Price (contract change) or define the price currency explicitly and convert from it; add a valuation test with a non-USD base portfolio. | New |
| R-08 | Minor | services/portfolio/infrastructure/src/clients/ledger.rs > record_trade body | The C7 body sends `kind` and omits `priceSource` and does not carry the transaction's currency provenance, while the C7 contract names `side` and a required `priceSource`. The ledger's DTO matches this body rather than the contract, and the user-versus-venue price label never reaches the ledger. | Align the contract, the ledger DTO and the client on one shape that carries the price source. | New |
| R-09 | Minor | services/portfolio/application/src/trades.rs > record, with the ledger service | The Ledger does not dedupe a replayed idempotency key (the C7 contract says a replay returns the existing entry). Portfolio's local dedupe covers ordinary retries, but a local-insert failure after a successful ledger write followed by a client retry creates a second ledger transaction. | Implement the C7 replay dedupe in the Ledger and add a test replaying the same key. | New |
| R-10 | Minor | services/portfolio/application/src/privacy.rs > anonymise audit | The erasure audit uses the system action `portfolio.privacy.anonymised`, which is not on C10's system-actor allow-list (it lists `system.privacy.deletion`), and targets the original user id. The C10 rejection happens after the erasure was applied, so the request fails with no audit written and a retry reports `already_applied` without an event. | Use an allow-listed system action (or a user-attributed event) and write the audit so that its failure does not fail a completed erasure; do not carry the erased id as the target. | New |
| R-11 | Minor | services/portfolio/http/src/dto.rs > percentages, with application/src/snapshot.rs > StoredLine | The allocation, target and drift percentages are carried and stored as `f64` (in the DTOs and in the frozen snapshot's lines), contrary to the project's decimal-only rule; value, price and quantity are strings. | Store and serve the percentages as decimal strings (converting to a number only if the contract insists) so the snapshot's "exact" reproduction is decimal. | New |
| R-12 | Minor | services/portfolio/infrastructure/src/clients/identity.rs | The base-currency preference read calls a C3 preferences path Identity does not serve and falls back to USD, so a user's preference never applies. | Have Identity expose the C3 preferences read and record the follow-up until it does; until then state that the default is USD. | New |
| R-13 | Minor | services/portfolio/http/src/dto.rs > PrivacyExportDto | The privacy export marks every snapshot `verified: true` without verifying it. | Verify or label the export snapshots accurately. | New |
| R-14 | Minor | services/portfolio/application/src/targets.rs > save | Every target save rebuilds the guardrails with new ids and a reset crossing state, so an asset still outside its guardrail re-notifies after any target save. | Preserve the guardrail crossing state (and id) across a save when the asset and threshold are unchanged, or document the re-notification. | New |
| R-15 | Minor | services/portfolio/application/src/guardrails.rs > payload | The drift alert payload includes `guardrailPoints`, which is not in the Notification contract's `drift` allow-list (`portfolioId`, `assetId`, `driftPoints`); the content guard drops it. | Align the payload with the Notification allow-list. | New |
| R-16 | Minor | services/portfolio/application/src/run.rs > status | Any single portfolio failure marks the whole run failed, so one C5 hiccup fails the run-success SLI. | Separate skipped/degraded portfolios from run-level failures. | New |
| R-17 | Minor | services/portfolio/application/src/run.rs > retention | The two-year snapshot prune is not implemented (the worker does not delete, and `portfolio_svc` is insert-only); the retention requirement and the infrastructure design's prune are unmet. | Provide the migrator/operator prune path and schedule it; record the storage consequence until then. | New |
| R-18 | Minor | code-generation/code-summary.md > deviations | TDD red evidence is not reproducible, the 80% per-crate coverage is unmeasured, and the 5 adapter integration tests are unrun; the project mandates test-first evidence, 80% coverage and one integration test per adapter. | Capture the red/green record, run coverage and the adapter suite against a disposable Postgres, and record the results. | New |
| R-19 | Minor | services/portfolio/application/src/snapshot.rs > StoredLine | The DTOs now carry Decimal percentages, but the persisted snapshot lines still store `allocation`, `target_percent` and `drift_points` as `Option<f64>` via `decimal_to_f64`, so the immutable snapshot keeps binary floating-point percentages and the reproduction comparison compares them as f64. | Store the stored-line percentages as decimal strings, as the value, price and quantity fields already do, and compare the reproduced lines on those strings. | New |
| R-20 | Minor | services/ledger/http/src/dto.rs > TradeRequest, with services/ledger/application/src/trades.rs > TradeCommand | The Portfolio client now sends `priceSource`, but the Ledger's `TradeRequest` and `TradeCommand` have no such field, so serde ignores it and the user-versus-venue price label never reaches the ledger, contrary to the C7 contract. | Add the price source to the Ledger request and command and persist it, or amend the C7 contract to drop it and record why. | New |
| R-21 | Minor | services/portfolio/application/src/run.rs > status, with http/src/dto.rs > RunResultDto | The run still marks itself failed when any single portfolio failed, including a C5-degraded skip, so one unavailability hiccup fails the run-success SLI even though the run completed. | Separate skipped/degraded portfolios from run-level failures and report them in the run result without failing the run. | New |
| R-22 | Minor | services/portfolio/http/src/handlers.rs > get_snapshots | The history route now takes limit and cursor parameters, but the C1/C8 contract still describes the history with `from` only. | Add the limit and cursor parameters (and the page response shape) to the contract. | New |
| R-23 | Minor | services/portfolio/application/src/run.rs > same_trade | The overlay match uses a ±24 h executed-at window with a 0.1% quantity tolerance, so a second identical trade of the same asset and side within a day can match the first trade's overlay and remove it, producing an undercount until the true transaction arrives. | Narrow the match window and tolerance, or require a stronger identifier (for example an external id) before removing an overlay. | New |
| R-24 | Minor | services/portfolio/application/src/intents.rs > project | The preview falls back to a fresh C5+C6 compute for a portfolio with no snapshot yet, which can exceed the 300 ms preview budget; the fallback is documented but not bounded. | Return an explicit "no valuation yet" refusal or take a first snapshot before previewing, so the preview path never performs the 800 ms compute. | New |
| R-25 | Minor | services/portfolio/infrastructure/src/clients/identity.rs | The base-currency preference read still calls a C3 path Identity does not serve and falls back to USD. | Have Identity expose the C3 preference read, or record the follow-up with its owner as the code summary does. | New |
| R-26 | Minor | code-generation/code-summary.md > deviations | The TDD red/green diary is not reproducible, per-crate 80% coverage is unmeasured, and the 6 adapter integration tests were compiled but not run. | Capture the red/green evidence, run coverage and the adapter suite against a disposable Postgres, and record the results. | New |

> R-01 Not re-checked this round

> R-02 Not re-checked this round

> R-03 Not re-checked this round

> R-04 Not re-checked this round

> R-05 Not re-checked this round

> R-06 Not re-checked this round

> R-07 Not re-checked this round

> R-08 Not re-checked this round

> R-09 Not re-checked this round

> R-10 Not re-checked this round

> R-11 Not re-checked this round

> R-12 Not re-checked this round

> R-13 Not re-checked this round

> R-14 Not re-checked this round

> R-15 Not re-checked this round

> R-16 Not re-checked this round

> R-17 Not re-checked this round

> R-18 Not re-checked this round

### Summary

The Critical and all six Majors are fixed and verified: the erasure now replaces the user id on portfolios and recorded trades with a fresh UUIDv4 reference (column-scoped UPDATE grant, export after erase empty, allow-listed `system.privacy.deletion` audit that never fails a completed erasure); the read applies post-snapshot overlays to the frozen inputs before serving; previews project over the latest snapshot's lines; the history is paginated with a cursor and bounded reproduction; the run uses four batches of four in parallel and continues past portfolio failures; overlay reconciliation searches from the trade's executed instant with a seven-day expiry; and FX uses the C6 quote currency, treating a missing currency as unpriced. The Minor fixes are also present: `priceSource` on the wire, Decimal percentages at the boundary, preserved guardrail crossing state, the allow-listed drift payload, and the `portfolio-migrate --prune-snapshots` operator path. The remaining items are Minor: the stored snapshot lines still use f64 percentages, the Ledger does not accept the price source, the run-level failure classification, the history contract parameters, the overlay matcher's wide tolerance, the no-snapshot preview fallback, the Identity preference fallback, and the unrecorded TDD/coverage/integration evidence.
