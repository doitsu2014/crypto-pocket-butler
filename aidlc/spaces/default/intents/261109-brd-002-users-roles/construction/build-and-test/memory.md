<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
2026-11-12T05:39:41Z — the ten Testing Contracts carry no "Run the selected tests in CI before merge" line, and `aidlc-state.md` lists `ci-pipeline` as SKIP, so the CI obligation is one `N/A` row ("the plan runs no CI Pipeline"), not an Unverified target.
2026-11-12T05:39:41Z — the target matrix groups requirement ids that share one producing check and leaves the id-level view to `cross-unit-traceability.md`; the per-id mapping stays in each Unit's `code-generation/traceability.json` rather than being restated 500 rows deep.
2026-11-12T05:39:41Z — a Unit's performance budgets are verified as mechanised (timeout, deadline, histogram plus alert) by the tests that assert them; the p95 itself is a production measurement the units assign to the metric, and `performance-validation` is skipped in this plan, which the summary records rather than claims.
2026-11-12T06:02:11Z — the blocking database gate must connect as a role that can `SET ROLE` the roles the migrations create (the image's `supabase_admin`, not `postgres`); connecting as plain `postgres` fails `SET ROLE` and surfaces as `PoolTimedOut`, which reads like a broken suite rather than a connection-privilege problem.
2026-11-12T06:02:11Z — coverage was measured with `--include-ignored` (the adapter suites are the only coverage the infrastructure crates get) and `--ignore-run-fail` (so the two known failing seam tests do not suppress the report); the failures are still recorded as failures in `test-results.md`, not absorbed into the coverage run.

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
2026-11-12T05:39:41Z — Standard strategy writes only `integration-test-instructions.md`; no performance or security instruction file was generated, as the strategy step directs.
2026-11-12T06:02:11Z — the failure ladder stops at rung 4 (halt-and-ask) without an autonomous loop-back: F1 (reporting manifest), F3 (frontend formatting drift) and F4 (validator DNS) cannot be fixed from inside any Unit of this intent, so a loop-back would spend the bound and still leave the stage red.
2026-11-12T06:21:47Z — the halt-and-ask was answered "Retry with fix", so Loop-back 1 (test-results.md `## Loop-Back Log`) reopens Code Generation for `u1-user-registry`, `u2-authorisation`, `u3-support-access` and `u6-jurisdiction-gate` — the four Units whose generated code caused F2a, F7 and F6. F2b is deliberately left failing: no code change on pgsodium 3.1.8 can make the assertion true, and weakening it would relax a security claim without the person's decision.
2026-11-12T07:50:00Z — the replay's Step 9 re-ran every command against the repaired tree and a second freshly created database (`identity_test3`). The F2a fix and the `identity` composition root (0.00 % → 84.65 %) held; `identity-infrastructure` rose to 64.95 % but is still short of the floor. The remaining Loop-back 1 legs (U1's coverage work, U3's account/portfolio HTTP coverage, U6's evidence file) are executed next as the same loop-back's forward path, not as a new entry in the bound.

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
2026-11-12T06:02:11Z — the two Identity seam failures were diagnosed rather than re-run until green: F2a is a stale U1 test against the U5 erasure guard, and F2b reproduces outside the suite (a fresh session can still encrypt with a deleted pgsodium key), so re-running could not honestly clear either.

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
2026-11-12T06:02:11Z — should the 46 pre-existing and six intent-touched frontend files be reformatted in this intent, or in a separate change, given the gate is red either way until the pre-existing drift goes?
2026-11-12T06:02:11Z — the crypto-shredding guarantee claims key destruction makes the subject unrecoverable; on pgsodium 3.1.8 the key row is deleted but the server keeps the key material cached, so the guarantee needs either a platform answer or a weaker documented claim.
2026-11-12T06:02:11Z — who owns the unused `serde`/`serde_json` declarations in `services/reporting/domain/Cargo.toml` (pre-existing, outside this intent's Units)?
