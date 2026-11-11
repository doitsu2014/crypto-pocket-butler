# Code Generation Plan — u5-account-lifecycle

## Summary

- Builds: The account status machine (admin disable/restore, user deletion request, within-window restore, expiry-and-erasure), its append-only `status_transitions` log and `status_audit_outbox`, the 30-day recovery window, and the idempotent erasure cascade with the erased-subject tombstone.
- Touches: `services/identity` (domain, application, infrastructure, http crates), migration `0006_account_lifecycle`, the gateway route table for the C7 lifecycle routes and the C3 tombstone response.
- Tests: ~30 Rust tests — domain state-machine and window rules, application use cases against port fakes, Axum HTTP boundary tests, and one `#[ignore]`d Postgres adapter/migration suite (up and down).

## Testing Contract

```json
{
  "version": 1,
  "methodology": "tdd",
  "source": "team",
  "ordering": "We write a failing test first for every piece of production code, then the implementation that makes it pass, and we land the test in the same change as the code it covers.",
  "scope": "classic",
  "test_strategy": "standard",
  "project_type": "brownfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "Standard: requirement-driven tests with a happy-path floor per component."
    },
    {
      "layer": "team",
      "text": "- **Methodology**: tdd\n- **Methodology evidence**: The team affirmed test-first development for all production code at\n  the previous practices interview and re-confirmed it in this run's evidence review. The\n  organisation baseline still applies underneath it: requirement-driven tests with a happy-path\n  floor per component (`memory/org.md`), acceptance criteria in Given/When/Then form\n  (`memory/phases/inception.md`), business rules tested in `application` use cases against port\n  fakes, and one integration test per adapter against a real Postgres schema\n  (`docs/development/README.md`). TDD governs when each of those tests is written, not what they\n  cover. The repository cannot prove write-order from history — there are 71 `.rs` files under\n  `services/*/tests` and the crate `tests/` trees (suites plus shared fakes and helpers), 50\n  frontend `*.test.ts(x)` files, and ten test files whose database-backed cases are marked\n  `#[ignore]` — so ordering remains a team commitment rather than an observable.\n- **Ordering**: We write a failing test first for every piece of production code, then the\n  implementation that makes it pass, and we land the test in the same change as the code it\n  covers.\n- **Test types and layout**: domain rule tests per service; application use cases against\n  hand-written port fakes in `tests/fakes.rs` or `tests/common/fakes.rs`; HTTP boundary tests\n  over the Axum routers with status codes and RFC 7807 problem bodies; infrastructure adapter\n  tests against a real Postgres schema, which are `#[ignore]`d by default and run only when\n  `TEST_DATABASE_URL` is set. The ignored adapter and migration suites are now a blocking gate,\n  not a developer opt-in: every run provisions a disposable Postgres database and points\n  `TEST_DATABASE_URL` at it, so an adapter regression fails the change. Until the CI platform is\n  chosen, that gate is run by the same local command a developer runs today. The frontends use\n  Vitest 3 with Testing Library and jest-axe; MSW mocks the gateway in the support console.\n  There is no end-to-end browser journey suite and no cross-service contract test (unresolved).\n- **Commands and runners**: the canonical build, lint and test commands are:\n  - Rust — `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,\n    `cargo test --workspace`, and `bash scripts/check-boundaries.sh` (the mechanical Clean\n    Architecture gate).\n  - Frontend — in both `web/` and `support-console/`: `yarn test` (`vitest run`), `yarn lint`\n    (ESLint 9 flat config), `yarn build` (`tsc -b && vite build`) and `yarn format:check`\n    (Prettier). `web` also defines `yarn size` (size-limit budgets: 300 kB initial entry chunk,\n    150 kB route chunk, 150 kB stylesheet, all gzip).\n\n  `aidlc/spaces/default/memory/project.md` holds this list as the single authoritative source;\n  `docs/development/README.md` and the per-package `package.json` scripts remain the\n  implementation detail behind it.\n- **Coverage**: 80% line coverage per Rust crate remains the affirmed target and is measured\n  with `cargo llvm-cov`, the coverage tool chosen at this interview. No TypeScript coverage\n  threshold has been set (unresolved). The happy-path-plus-two-error/edge-case floor per test\n  file still applies and a test that always passes regardless of implementation is not\n  acceptable (`memory/phases/construction.md`).\n- **Quality gates**: `bash scripts/check-boundaries.sh` is a real mechanical gate — it parses\n  `cargo metadata --no-deps` and fails on any outward crate edge. The Rust gates are\n  `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and\n  `cargo test --workspace`; the frontend gates are `yarn test`, `yarn lint`, `yarn build` and\n  `yarn format:check`. There is still no CI platform in the repository — no `.github/`, GitLab,\n  Jenkins or CircleCI configuration — and the team chose at this interview to leave the platform\n  open for now and record CI as an intention, so today these gates run when a person runs them.\n  Dependency and container-image scanning (`cargo audit`, Trivy) is required by BRD-012\n  NFR-SEC-14 and named in unit design records but has no configuration or pipeline."
    }
  ],
  "obligations": {
    "strategy": "standard",
    "strategy_volume": [
      "Five to eight tests per component.",
      "Unit tests plus integration tests for key boundaries.",
      "Add E2E, performance, or security tests when requirements demand them."
    ],
    "scope_floor": [
      "Keep the existing test suite green.",
      "This scope adds no extra new-test floor beyond the selected test strategy."
    ],
    "combination_rule": "Apply every selected-strategy obligation and every scope-floor obligation; neither replaces the other, and a targeted scope regression may add the narrowest necessary test type beyond the strategy default."
  },
  "plan_profile": {
    "methodology": "tdd",
    "runner_step": "Verify the existing test runner/configuration and record the exact unit-scoped command.",
    "runner_ready_before_first_test": true,
    "testable_layers": [
      "Data model / database behavior",
      "Repository / data access",
      "Business logic",
      "API / endpoint",
      "Frontend behavior"
    ],
    "steps": [
      "Project structure and production configuration skeleton.",
      "Verify the existing test runner/configuration and record the exact unit-scoped command.",
      "Data model / database behavior - Red: write the failing tests and record the failing command output.",
      "Data model / database behavior - Green: implement only enough behavior to pass.",
      "Data model / database behavior - Refactor: improve the implementation while tests stay green.",
      "Repository / data access - Red: write the failing tests and record the failing command output.",
      "Repository / data access - Green: implement only enough behavior to pass.",
      "Repository / data access - Refactor: improve the implementation while tests stay green.",
      "Business logic - Red: write the failing tests and record the failing command output.",
      "Business logic - Green: implement only enough behavior to pass.",
      "Business logic - Refactor: improve the implementation while tests stay green.",
      "API / endpoint - Red: write the failing tests and record the failing command output.",
      "API / endpoint - Green: implement only enough behavior to pass.",
      "API / endpoint - Refactor: improve the implementation while tests stay green.",
      "Frontend behavior - Red: write the failing tests and record the failing command output.",
      "Frontend behavior - Green: implement only enough behavior to pass.",
      "Frontend behavior - Refactor: improve the implementation while tests stay green.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:28b1b8f64ea2370ac2e8ea34ac6ae154b256412b60b0b2b72311ed876bfb0f5c",
  "contract_sha256": "sha256:3f8863a9358b0b5e19a34ad2fc282a634f0d5323fb50ae1f7429a92a216766c3"
}
```

## Plan Steps

The unit is a slice inside the existing Identity service, so no new crate or binary is scaffolded. Steps follow the Testing Contract's TDD ordering, adapted to the four layers this unit touches (data model, repository, business logic, API/endpoint). The "Frontend behavior" layer is genuinely inapplicable to this service unit and is omitted. Each step lands its test and its implementation in the same change.

- [x] **Step 1 — Verify the test runner and record the unit-scoped command.** Run the existing test suite for the Identity package to confirm the runner is green before any change (`cargo test -p identity`), and record the exact unit-scoped command in `unit-test-instructions.md`. *(No production code; runner readiness before the first Red step.)* — traceability: setup.

### Data model / database behavior

- [x] **Step 2 — Red: migration `0006_account_lifecycle` up.** Write the failing adapter test (`tests/account_lifecycle_adapters.rs`, `#[ignore]`d) that applies the up migration to a disposable schema and asserts the new shape: `users.erased_subject_hash`, the five new/loosened `deletion_requests` columns (`recoverable_until`, `cancelled_at`, `erased_at`, `attempt_count`, `next_attempt_at`), the `status_transitions` append-only table with its `seq` index, the `status_audit_outbox` table, and the one-open-request unique index keyed on `cancelled_at IS NULL AND erased_at IS NULL`. Record the failing output. — US4.3, US4.5, AC4.3.1, AC4.5.2, BR5.3, BR5.6.
- [x] **Step 3 — Green: implement the up migration.** Write `services/identity/migrations/0006_account_lifecycle.up.sql` per `infrastructure-specification.md`: add `users.erased_subject_hash`; `ALTER deletion_requests` to add the five columns (nullable `cancelled_at`/`erased_at`, `attempt_count NOT NULL DEFAULT 0`, `next_attempt_at NOT NULL DEFAULT now()`); map `status` to the open/terminal meanings; create `status_transitions` (append-only, insert-only grant) and `status_audit_outbox` with their indexes; drop `0001`'s status-keyed partial unique index and re-create it on the open condition. — traceability as Step 2.
- [x] **Step 4 — Red: migration down + rollback guard.** Extend the adapter test to apply the down migration and assert the added columns and tables are gone; assert the down refuses while an open request exists. — US4.3, AC4.3.1.
- [x] **Step 5 — Green: implement the down migration.** Write `services/identity/migrations/0006_account_lifecycle.down.sql`: reverse the columns, drop `status_transitions` and `status_audit_outbox`, restore the `0001` index, and refuse while any open request exists so a rollback cannot strand a deletion. — traceability as Step 4.
- [x] **Step 6 — Refactor: tighten migration checks.** Re-run the adapter suite green; ensure the CHECK constraints on the new columns match `entities.md` (reason 1–500, one open request, deleted shape) and the down guard's error is surfaced rather than swallowed. — BR5.1, BR5.2.

### Repository / data access

- [x] **Step 7 — Red: `StatusTransitionRepository` and `StatusAuditOutbox` adapters.** Write failing adapter tests asserting an insert-only transition (an `UPDATE`/`DELETE` is refused for the application role), the monotonic `seq`, and the outbox row written in the same transaction. — BR5.9, AC7.2.1, AC7.2.2.
- [x] **Step 8 — Green: implement the two repositories.** Add `services/identity/infrastructure/src/repositories/status_transitions.rs` (insert + `SELECT` newest per user) and the outbox insert/drain queries; wire them into `repositories/mod.rs`. — traceability as Step 7.
- [x] **Step 9 — Red: `DeletionRequestRepository` extension.** Write failing adapter tests for the new columns (claim by `next_attempt_at <= now()`, one open request per user via `cancelled_at`/`erased_at`, `attempt_count` increment and backoff). — BR5.3, BR5.5, BR5.6, AC4.3.2.
- [x] **Step 10 — Green: implement the repository extension.** Extend `services/identity/infrastructure/src/repositories/deletions.rs` (or the deletion port's adapter) to read/write the new columns and claim due requests. — traceability as Step 9.
- [x] **Step 11 — Red: tombstone lookup in the users repository.** Write failing adapter tests asserting `find_by_subject_hash`/`find_by_auth_subject` return the erased subject (a `SubjectErased`-shaped result) when `users.erased_subject_hash` matches, and `anonymise` writes `erased_subject_hash` before `user_personal` is deleted. — BR5.8, AC4.5.1, NFR22.
- [x] **Step 12 — Green: implement the tombstone lookup.** Change `services/identity/infrastructure/src/repositories/users.rs`: both lookups consult `users.erased_subject_hash` when the `user_personal` join is empty; `anonymise` performs the guarded move (copy the hash, set status `deleted`, clear personal fields) per `security-design.md`. — traceability as Step 11.
- [x] **Step 13 — Refactor: repository boundary.** Re-run the adapter suite; confirm no repository leaks a DTO and the erasure move stays one transaction. — clean-architecture gate.

### Business logic

- [x] **Step 14 — Red: domain status machine.** Write failing domain tests (`tests/account_lifecycle_domain.rs` and inline in `domain/src/lifecycle.rs`) for the `StatusTransition` value and the permitted-edge guard (BR5.1's six edges; every other edge is refused), and for the 30-day midnight-UTC window computation. — BR5.1, BR5.3, BR5.4, AC4.1.2, AC4.3.1.
- [x] **Step 15 — Green: implement the status machine and window.** Add `domain/src/lifecycle.rs` (or extend `domain/src/user.rs`) with the `StatusTransition` entity, the edge table, and a `transition` method that returns a `DomainError` for an illegal edge; add the `recoverable_until` computation (midnight UTC of the 30th calendar day). Re-export from `domain/src/lib.rs`. — traceability as Step 14.
- [x] **Step 16 — Red: domain `DeletionRequest` extension and idempotent erasure.** Write failing domain tests for the new request fields (`recoverable_until`, `cancelled_at`, `erased_at`, `attempt_count`, `next_attempt_at`) and the `User::anonymise` change that records `erased_subject_hash`. — BR5.6, AC4.5.2, NFR22.
- [x] **Step 17 — Green: implement the domain extension.** Extend `domain/src/deletion.rs` (DeletionRequest) and `domain/src/user.rs` (anonymise) accordingly. — traceability as Step 16.
- [x] **Step 18 — Red: account-lifecycle use cases.** Write failing application tests (`tests/account_lifecycle_application.rs`) against port fakes for `AccountLifecycleService`: disable/restore require `admin` and a 1–500 reason (403/422), an illegal edge is 409 with nothing written, a second open deletion request is 409, a restore within the window returns to `active`, a restore after the deadline is 410. — US4.1, US4.2, US4.3, BR5.2, BR5.3, BR5.4, AC4.2.1, AC4.2.2, AC4.3.1.
- [x] **Step 19 — Green: implement the use cases.** Add `application/src/account_lifecycle.rs` (disable, restore, request_deletion, cancel_deletion) that writes `users.status`, the transition, and the outbox in one transaction, using the transition path. Re-export from `application/src/lib.rs`. — traceability as Step 18.
- [x] **Step 20 — Red: the expiry job.** Write failing application tests for the expiry-and-erasure pass: a due `pending_deletion` request transitions to `deleted`, the cascade runs (Auth user deleted first, then the guarded in-place anonymise, then consent/preferences through `identity_retention`), a request whose status moved is cancelled and skipped, a failed step leaves the request open with `attempt_count + 1` and a backoff, and a second run over an erased request is a no-op that reports success. — BR5.5, BR5.6, BR5.10, AC4.3.2, AC4.5.2.
- [x] **Step 21 — Green: implement the expiry job.** Rework/extend `application/src/deletion.rs`'s worker (reusing the existing sweep loop, `IDENTITY_SWEEP_INTERVAL_SECONDS`, and the lease) to the status-machine erasure flow with attempt counting and backoff; add the `deletion_erasure_failures_total` metric. — traceability as Step 20.
- [x] **Step 22 — Red: principal tombstone resolution.** Write failing application tests asserting `PrincipalService::resolve_with_origin` returns 403 (never provisioning) when the lookup yields `SubjectErased`. — BR5.8, AC4.5.1.
- [x] **Step 23 — Green: implement the tombstone resolution.** Add the `SubjectErased` outcome to `application/src/principal.rs` and the repository result, mapping it to `AppError::Forbidden`; keep the existing `denies_use` behaviour as the fallback until the u2 precondition ships. — traceability as Step 22.
- [x] **Step 24 — Red: audit outbox drain.** Write failing application tests for the drainer that reads `status_audit_outbox`, posts to C9 (`POST /internal/events`) with an `Idempotency-Key`, and retries with bounded backoff. — BR5.9, AC7.2.1, AC7.2.2, C9.
- [x] **Step 25 — Green: implement the outbox drain.** Add the drainer to the worker loop, reusing the existing `audit_client`/`http_retry` infrastructure. — traceability as Step 24.
- [x] **Step 26 — Refactor: business-logic pass.** Re-run the application and domain suites; confirm the transition path owns every status change (the interim `pending → active` promotion in `apply_claims` is routed through the guarded transition, or explicitly documented as u1's path through u5). — US4.1, BR5.1.

### API / endpoint

- [x] **Step 27 — Red: C7 lifecycle routes.** Write failing Axum router tests (`tests/account_lifecycle_http.rs`) for `POST /internal/users/{id}/disable`, `POST /internal/users/{id}/restore`, `POST /internal/users/{id}/deletion-requests`, and `DELETE /internal/users/{id}/deletion-requests`, covering the 200/201, 403, 409, 410, and 422 RFC 7807 bodies. — C7, W1, W2, W3.
- [x] **Step 28 — Green: implement the C7 routes, handlers, DTOs, and middleware.** Add the routes to `http/src/routes.rs`, handlers to `http/src/handlers.rs`, DTOs to `http/src/dto.rs`, and the `require_internal_caller` allowlist entry; map `recoveryEndsAt` in the response. — traceability as Step 27.
- [x] **Step 29 — Red: C3 tombstone response.** Write a failing Axum test asserting `GET /internal/users/by-subject/{subjectHash}` answers 403/410 for an erased subject instead of 404. — C3, BR5.8, AC4.5.1.
- [x] **Step 30 — Green: implement the tombstone response.** Update the registry read handler to surface `SubjectErased` per `security-design.md` and `infrastructure-specification.md` item 3. — traceability as Step 29.
- [x] **Step 31 — Red: existing deletion surface response codes.** Write failing Axum tests asserting the existing `/v1/privacy/deletions` request returns `recoveryEndsAt` and the 409/410 codes on a second request / late restore. — C7, W2, W3.
- [x] **Step 32 — Green: implement those response codes.** Update the existing deletion handlers/DTOs. — traceability as Step 31.
- [x] **Step 33 — Refactor: endpoint boundary.** Re-run the HTTP suite; confirm DTOs map to/from domain types at the boundary and no DTO leaks inward. — clean-architecture gate.

### Configuration, documentation, traceability

- [x] **Step 34 — Configuration and metrics.** Wire `deletion_erasure_failures_total` (and the transition/outbox lag metric) into `http/src/metrics.rs`/the worker; confirm no `infra/swarm-stack.yml` change is needed (the sweep reuses `IDENTITY_SWEEP_INTERVAL_SECONDS`); update the gateway route table only for the internal C7/C3 additions per the deny-by-default rule. — performance-design, observability-design, US4.4.
- [x] **Step 35 — Documentation and traceability.** Update the C4 model and `docs/architecture/README.md` if a container/schema/relationship changed; write `code-summary.md`, `source-manifest.json`, and `traceability.json` (via the traceability sensor), marking `AC4.4.2` (legal-hold) `Deferred` with the functional-spec's recorded reason. — US4.4, US7.2, FR17.4, project traceability rule.

## Traceability summary

| Story/AC | Steps |
|---|---|
| US4.1 (u5 owns transitions) | 6, 14–15, 18–19, 26 |
| US4.2 (admin disable, no history erase) | 18–19, 27–28 |
| US4.3 (request + 30-day window) | 2–5, 9–10, 14–15, 18–19, 27–28, 31–32 |
| US4.4 (legal-hold-aware erasure) | 20–21, 34–35 |
| US4.5 (deleted denied; idempotent erasure) | 2–5, 11–12, 16–17, 20–21, 22–23, 29–30 |
| US7.2 (status-change audit) | 7–8, 18–19, 24–25 |
| BR5.1–BR5.10 | 2–35 (each rule named at its step) |
