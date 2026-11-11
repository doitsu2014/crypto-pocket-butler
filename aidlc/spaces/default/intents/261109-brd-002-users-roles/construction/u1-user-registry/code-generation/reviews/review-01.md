## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-11-10T08:51:30Z
**Iteration:** 1
**Verdict Dictated:** no

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | `services/identity/application/src/principal.rs` > `refresh` | On an existing user, `apply_claims` recomputes email, email-verified and display name, but `refresh` calls `users.update` only when the role changed or a pending user becomes active. A provider-side change to an already-active user's email or display name (or to a still-pending user's claims) is returned in memory but never persisted, so later C3 reads and later requests see stale profile data. BR1.4 and functional-spec W1 step 4 require the refresh to update these stored fields. | Persist every changed profile field: compare the user before and after `apply_claims` and call `update` when email, email_verified, display_name or role changed. Add an application test that a changed provider email and display name are stored on an already-active user. | New |
| R-02 | Major | `services/identity/http/src/routes.rs` > `router`, and `services/identity/application/src/user_registry.rs` | `User::update_profile` (the W2 profile-update workflow and the only source of the `display_name_overridden` / `email_overridden` markers) has no application use case and no HTTP route; it is referenced only by unit tests. In production a user therefore cannot set an override, so the override path that BR1.4 and AC1.4 depend on is unreachable even though traceability marks AC1.4 as `OK`. | Add an owner-only, JWT-protected profile-update use case and route wired to `update_profile` and `users.update`, with an HTTP test; or record the workflow as `Deferred` against the unit that will own it and update traceability to match. | New |
| R-03 | Major | `aidlc/spaces/default/intents/261109-brd-002-users-roles/construction/u1-user-registry/code-generation/code-summary.md` > Coverage; `unit-test-instructions.md` > Coverage | The affirmed rule is at least 80% line coverage per Rust crate, but the summary reports 58.39% overall / 64.53% lines and states the adapter and binary crates are below target. The target is not met and no approved exception is recorded; `cargo llvm-cov` itself was not run by this review. | Run `cargo llvm-cov` with the `#[ignore]` adapter suites against a disposable Postgres and record per-crate numbers; bring each crate to 80% or record an explicit, approved exception for the crates that fall short. | New |
| R-04 | Minor | `services/identity/http/src/routes.rs` > `router` | The shared C3 contract lists `PUT /internal/users/{id}/status` (owned by U1) but no route or handler implements it; only the by-subject provision, and by-id / by-subject / by-email reads exist. U5 has nowhere to send a transition request and would need a contract change. | Implement the C3 status-transition endpoint behind a service-token allowlist (u5), or record it as `Deferred` to U5 in traceability. | New |
| R-05 | Minor | `aidlc/spaces/default/intents/261109-brd-002-users-roles/construction/u1-user-registry/code-generation/code-generation-plan.md` > top of file | The plan does not open with the required three-line `## Summary` (Builds / Touches / Tests); it starts with the title and `## Scope`. Consistently, the approval question recorded "0 plan steps", so the numbered `[x]` steps were not counted as plan steps. | Add the `## Summary` section and express the steps as checkbox items the engine counts, so plan progress is tracked. | New |
| R-06 | Minor | `aidlc/spaces/default/intents/261109-brd-002-users-roles/construction/u1-user-registry/code-generation/unit-test-instructions.md` > Exact commands > Whole workspace gate | This file includes a workspace-wide `cargo test --workspace` row (and cites `cargo llvm-cov --workspace`). Stage rules require every run command in this file to be unit-scoped, because Build and Test runs each unit's commands and a workspace command reruns the whole suite once per unit. | Remove the workspace-wide row from this file; keep the unit-scoped commands there and leave the full-suite command in the plan / verification section. | New |
| R-07 | Minor | `aidlc/spaces/default/intents/261109-brd-002-users-roles/construction/u1-user-registry/code-generation/traceability.json` > `reverse` | BR1.8 appears both in `coverage` with status `OK` and in `reverse` with status `N/A`. `reverse` is meant to list only produced items that trace to nothing upstream, so the same upstream ID in both tables is contradictory and may fail the traceability sensor. | Remove BR1.8 from `reverse` (keep it in `coverage`), or explain the distinct produced artifact it is meant to name. | New |
| R-08 | Minor | `services/identity/http/src/dto.rs` > `PrincipalResponse` | The shared C1 `Principal` schema enumerates `role: [owner, admin, support]`, but the response serialises `Role::as_str()`, which yields `user` for a non-admin. The `Role` enum predates this unit, but the new `/v1/principals` route is the first to expose it to the SPA. | Confirm the intended vocabulary with U2 and the contract, then map `user` to `owner` (or amend the contract) before the SPA and U2 depend on it. | New |
| R-09 | Minor | `services/identity/migrations/0004_user_registry_extensions.down.sql` > `UPDATE users` | The down migration maps `disabled` and `pending_deletion` rows to `active`, silently re-enabling accounts an administrator disabled or a user asked to delete. It is documented as a deliberate lossy downgrade, but a rollback would restore access those states had removed. | Confirm this rollback behaviour is acceptable; consider failing the rollback with an explicit operator step, or preserving the disabled state in a form the narrower check accepts. | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `cargo test -p identity --test domain user_registry` | PASS: 5 passed, 36 filtered out | Unit-scoped domain tests green |
| `cargo test -p identity --test application user_registry` | PASS: 8 passed, 30 filtered out | Unit-scoped application tests green |
| `cargo test -p identity --test http user_registry` | PASS: 7 passed, 32 filtered out | Unit-scoped HTTP tests green |
| `cargo test --workspace` | PASS: 1399 passed, 95 ignored | Full suite green; the 95 ignored are the Postgres-backed adapter/migration suites, not run here, so no adapter coverage from this run |
| `cargo fmt --all -- --check` | PASS | Formatting clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS: no issues | Lint clean |
| `yarn --cwd web test` / `yarn --cwd web lint` | PASS: 142 tests | Frontend gate green; this unit changed no frontend file |
| `yarn --cwd support-console test` / `yarn --cwd support-console lint` | PASS: 180 tests | Support-console gate green; unchanged by this unit |
| `cargo llvm-cov` | Not run by this review | The 58.39% coverage figure is the build's own report; it is below the 80%-per-crate rule (see Major finding) |
| `yarn --cwd web build`, `yarn --cwd support-console build`, `bash scripts/check-boundaries.sh` | Not run by this review | Recorded verification command omits these; the known pre-existing `reporting-domain` serde edge is expected to fail boundaries |

### Summary

The registry core is solid and the tests, fmt, clippy and frontend gates pass, but two functional gaps make the built feature incomplete: provider-driven profile refreshes are never persisted, and the W2 profile-update/override path has no application or HTTP entry point. Alongside the admitted sub-80% coverage, those three items should be closed or explicitly accepted before approval.
