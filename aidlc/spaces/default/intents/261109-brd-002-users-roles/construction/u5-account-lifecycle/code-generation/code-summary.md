# Code Summary — u5-account-lifecycle

Built the account status machine, its append-only `status_transitions` log and `status_audit_outbox`, the 30-day recovery window, the expiry-and-erasure job with attempt counting and backoff, the erased-subject tombstone (lookup, denial, C3 response), and the C7 lifecycle routes — inside the existing Identity service, with migration `0006_account_lifecycle`.

## Files created

| Path | Purpose |
|---|---|
| `services/identity/migrations/0006_account_lifecycle.up.sql` | `users.erased_subject_hash`; five `deletion_requests` columns; `status_transitions` (insert-only); `status_audit_outbox`; the one-open-request index |
| `services/identity/migrations/0006_account_lifecycle.down.sql` | Reverses 0006; refuses while an open request exists |
| `services/identity/domain/src/lifecycle.rs` | `StatusTransition`, the permitted-edge table, the 30-day midnight-UTC window and the retry backoff |
| `services/identity/application/src/account_lifecycle.rs` | Disable / restore / request-deletion / cancel-deletion use cases |
| `services/identity/infrastructure/src/repositories/status_transitions.rs` | `PgStatusTransitionStore`: guarded status write, transition insert and outbox insert in one transaction |
| `services/identity/tests/account_lifecycle_domain.rs` | Domain state-machine and window rules |
| `services/identity/tests/account_lifecycle_application.rs` | Use cases against port fakes |
| `services/identity/tests/account_lifecycle_http.rs` | C7 and C3 boundary tests with RFC 7807 bodies |
| `services/identity/tests/account_lifecycle_adapters.rs` | `#[ignore]`d migration, repository, outbox and tombstone suite |

## Files modified

`services/identity/domain/src/{ids,lib,user,deletion}.rs`, `services/identity/application/src/{lib,ports,common,deletion,principal,user_registry}.rs`, `services/identity/application/src/consent/ports.rs`, `services/identity/infrastructure/src/repositories/{mod,users,deletions}.rs`, `services/identity/infrastructure/src/consent/outbox.rs`, `services/identity/http/src/{dto,handlers,metrics,problem,routes,state}.rs`, `services/identity/src/lib.rs`, and the existing tests `services/identity/tests/{common/fakes,common/mod,application,worker,repositories}.rs`.

No `docs/architecture/` change: no new container, schema element in the C4 model, external system, or relationship. `infra/swarm-stack.yml` is unchanged (the sweep reuses `IDENTITY_SWEEP_INTERVAL_SECONDS`). No gateway route-table change: C7/C3 are `/internal/...` paths and C3's change is a response code, not a route.

## Key implementation decisions

- **One transaction per status change.** `StatusTransitionStore` (`application/src/ports.rs`) exposes `apply` / `open_deletion` / `cancel_deletion` / `newest_for_user`; `PgStatusTransitionStore` writes `users.status` guarded on the observed `from`, the transition row, and the outbox row together. The domain re-validates the edge inside the adapter, so an illegal edge writes nothing.
- **30-day window** — `recoverable_until` is midnight UTC on the 30th calendar day. Retry backoff is 1/2/4/8/16/32 minutes capped at 60; after five attempts the request is parked (`failed` + `cancelled_at`) so it stops blocking a new request, and `deletion_erasure_failures_total` counts it.
- **Tombstone** — `PgUserRepository::anonymise` writes `users.erased_subject_hash` in one transaction *before* deleting `user_personal`, guarded on `status IN ('pending_deletion','deleted')` so a concurrent restore wins. Both subject lookups fall back to `erased_subject_hash`; C3 answers `410 Gone`; principal resolution audits `identity.principal.erased` and returns 403 without provisioning.
- **Erasure order** — Supabase Auth delete → `pending_deletion→deleted` transition + outbox → guarded tombstone move → contributor erasure → shredding key → consent/preferences through `identity_retention`. Every step is progress-flagged and idempotent; a second run is a no-op that reports success.
- **Outbox drain** — the existing `identity-api` `AuditOutboxDrainer` drains both the consent and status tables, one client, per-table claim and backoff.

## Test coverage summary

| Command | Result |
|---|---|
| `cargo test -p identity --test account_lifecycle_domain --test account_lifecycle_application --test account_lifecycle_http` | 23 passed |
| `cargo test -p identity` | 230 passed, 37 ignored (pre-existing suites stay green) |
| `TEST_DATABASE_URL=<disposable Postgres> cargo test -p identity --test account_lifecycle_adapters -- --ignored --test-threads=1` | 10 passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean |
| `bash scripts/check-boundaries.sh` | **fails, pre-existing and unrelated**: `reporting-domain` imports `serde`/`serde_json`; `services/reporting` has no changes in this work |

Line coverage was not measured in this unit run (`cargo llvm-cov` was not invoked). The 80% per-crate floor is a whole-project obligation that Build and Test checks on the full assembled suite; this unit's run neither proves nor waives it.

## Tests written but not executed

- The pre-existing `services/identity/tests/repositories.rs` adapter cases (22 of 27) and the `authorisation_adapters` cases that seed users through `PgShreddingKeyRepository::create_for_user` need pgsodium. The local disposable Postgres has no pgsodium in `pg_available_extensions`, so `create_shredding_key` fails with `schema "pgsodium" does not exist`. This is an environment limitation, not a regression. The updated existing `deletion_claims_are_atomic_and_a_failed_request_is_retryable` case could not run here; its new claim semantics are covered by two passing cases in `account_lifecycle_adapters.rs`.
- The `#[ignore]`d adapter suite WAS executed here (10 passed) against a local disposable Postgres, as recorded above.

## Deviations from the plan

1. **TDD ordering was not held for two layers.** The domain and adapter tests were written before their implementations. The application and HTTP **implementations were written before their tests**: a full rebuild (after `cargo clean -p identity-infrastructure` to embed the new migration) took very long, and the implementation was written while it compiled. The tests are behavioural and not tautological, but this work does **not** claim strict Red-then-Green order for those two layers, and the team rule requiring it is not satisfied for them.
2. **`SubjectErased`** is represented by returning the erased `User` from the repository lookups plus a distinct `identity.principal.erased` audit and 403, rather than a new lookup enum. Observable behaviour matches the design; the repository contract did not change shape.
3. **Migration 0006 is transactional**, not statement-wise: no `CONCURRENTLY`, and `deletion_requests` is backfilled in place. The runner keys its live-safe path off `LIVE_SAFE_VERSION = 5`; extending it was out of scope.
4. **C7 owner paths** resolve the stored user directly so a `pending_deletion` account can restore; every other route keeps u2's denial.
5. **Existing tests updated to the approved contract**: a second deletion request is now 409, the sweep runs at +31 days with the new backoff, the deleted-vs-disabled audit action differs, the 0004-down test reverts 0006 then 0005 first, the deletion-claim test uses the due window, and `DeletionService::request` moved to `AccountLifecycleService::request_deletion`.
6. **A post-transition failure parks the request with the account already `deleted`** and `erased_at` unset; the `failed` row is the operator resume point. Recorded in the test and comments rather than changing the security design's order.
7. **`DeletionService` was slimmed** to open-request/status/C10-event reads; its old `request`/`wake`/`clock` moved to the lifecycle use case.

## Carried gaps

- **AC4.4.2 (legal hold) is `Deferred`**, not satisfied: BRD-017's legal-hold register and a statutory retention schedule do not exist. This release erases unconditionally, as `functional-spec.md` records; the gap has two named owners (the register, and compliance's retention schedule). See `traceability.json`.
- **Precondition on u2** (read-versus-write predicate for `pending_deletion` and the `account:connect` operation) is still open; until it ships, a `pending_deletion` account is denied everything, which is stricter than BR5.7 and safe.
- **The boundary gate failure in `reporting-domain`** needs an owner outside this unit.
