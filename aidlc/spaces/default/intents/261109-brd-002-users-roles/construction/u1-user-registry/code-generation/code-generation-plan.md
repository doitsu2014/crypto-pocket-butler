# Code Generation Plan — u1-user-registry

**Unit:** `u1-user-registry` (U1) · **Scope:** classic · **Depth:** Standard · **Test strategy:** Standard
**Source of truth:** `functional-spec.md`, `rules.md`, `entities.md`, `contract-summary.md` (C1, C3, C8), `nfr-design/*`, `infrastructure-specification.md`.

## Scope

Extend the existing Identity service user registry: profile override markers, the four public statuses plus the transient `pending`, transactional provision with the jurisdiction gate and shredding key, principal resolution, admin lookup, the C3 routes, the public `/v1/principals` gateway route, an identity migrate binary and stack service, and the `0004` migration pair. No new service; no new Postgres role.

## Decisions this plan settles (from the stage reviews)

| Open point | Resolution in this build |
|---|---|
| Migration number | u6 took `0003`; this unit uses **`0004_user_registry_extensions`** (up/down). |
| Email index | Add a partial index `users(email) WHERE email IS NOT NULL` to the `0004` up migration; drop it in the down. |
| Down-migration safety | The `0004` down maps `disabled` and `pending_deletion` rows back to `active` (a comment states why) **before** re-imposing the narrower check, so the rollback cannot fail on new statuses. |
| Identity migrate runner | Add `services/identity/src/bin/migrate.rs`, `Dockerfile.migrate`, an `identity-migrate` one-shot Swarm service and an `identity_migrate_database_url` secret, mirroring the other services. |
| Gate is in-process | The registry calls the `JurisdictionGate` component in-process; the C8 HTTP endpoint stays for external consumers (U8). The design's "gate unavailable while users still resolve" is corrected to: a gate failure affects provisioning only, in-process. |
| Pending → active | Provision `active` when the provider says the email is verified; provision `pending` when it is not. A pending user's later verified request promotes through the lifecycle transition path (u5 when built; until then the registry performs the guarded promotion itself with an audit event, recorded as a deliberate interim). |
| Profile overrides | Add `display_name_overridden` and `email_overridden` booleans to `users`; the refresh skips an overridden field. |
| Admin search contract | Add `GET /v1/admin/users?email=<exact>` to the gateway (C1) and `GET /internal/users?email=<exact>` to C3; exact match only; the response is the metadata boundary (display name, email-verification status, status, ids) and never portfolio contents. |
| Role event | No `identity.role.changed` from this unit; role changes belong to u2. |
| Hash collision | Fail closed: a hash hit whose decrypted ciphertext differs returns a validation error, logs a security event, creates and returns no record. |
| Gate denied | `422`, matching C3. |
| users shared row | U2/U3/U5 read status and ownership through C3 or in-process; role comes from the validated token. |
| Readiness | `/healthz` stays the Swarm health check (a DB outage must not restart-loop every replica); `/readyz` reports database and dependency state and is alerted on, not used as the update gate. |
| Recovery | Supabase Postgres automated backups; restore path documented in the summary; RPO ≤24 h, RTO ≤4 h. |
| Smoke | Smoke is `/healthz` + `/readyz` plus one principal resolution using an `identity_smoke_token` service token (new secret). |

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

## Steps

1. [ ] **Skeleton/configuration** — modules: `domain/src/user_registry.rs` additions (status enum, overrides), `application/src/user_registry.rs` (ProvisionUser, ResolvePrincipal, AdminLookup), `infrastructure/src/repositories/users.rs` additions, `http` handlers/routes/dto.
2. [ ] **Verify runner** — `cargo test -p identity-domain -- --list`; record the unit commands.
3. [ ] **Data model — Red** — failing status/override tests.
4. [ ] **Data model — Green** — implement.
5. [ ] **Data model — Refactor**.
6. [ ] **Repository — Red** — `#[ignore]` tests: provision transaction, unique subject hash, override skip, admin email index.
7. [ ] **Repository — Green** — `0004` migration pair (status check widened, override columns, partial email index, down maps new statuses back before narrowing), `PgUserRepository` extensions, migrate binary + Dockerfile.
8. [ ] **Repository — Refactor**.
9. [ ] **Business logic — Red** — provisioning with gate (in-process) and email-verified status; resolve with status denial; admin lookup boundary; hash-collision fail-closed; no role event.
10. [ ] **Business logic — Green** — use cases behind ports.
11. [ ] **Business logic — Refactor**.
12. [ ] **API — Red** — HTTP tests for `/v1/principals`, C3 routes, admin search (403/404), gate-denied 422.
13. [ ] **API — Green** — handlers, DTOs, gateway route declarations (`/v1/principals`, `/v1/admin/users`), metrics.
14. [ ] **API — Refactor**.
15. [ ] **Frontend** — not applicable.
16. [ ] **Deployment** — `identity-migrate` service, migrator DSN secret, `identity_smoke_token`; no replica change.
17. [ ] **Environment/build** — config for the migrator DSN and smoke token.
18. [ ] **Documentation/traceability** — `code-summary.md`, `source-manifest.json`, `traceability.json`.

### Loop-back 1 repair (Build and Test, 2026-11-12)

Build and Test reopened this Unit for F2a (a stale seam test) and F6 (crate coverage). The repair
below is the only work this replay adds; steps 1–18 are kept as they stand.

19. [ ] **Repair — seam test (F2a), Red then Green** — in
    `services/identity/tests/repositories.rs`, the test
    `a_user_is_found_by_the_hashed_subject_and_the_subject_is_protected_at_rest` stores an
    **active** user and then calls `anonymise`, which the erasure guard from U5 deliberately
    refuses. Red: add a case asserting that `anonymise` on an active user is refused with
    `Conflict`, and move the existing at-rest assertions onto a leg that first drives the stored
    user to `pending_deletion` through `User::transition` plus `UserRepository::update`. Green:
    the seam test passes against a disposable Postgres, and the at-rest protection assertions are
    unchanged in substance.
20. [ ] **Repair — coverage of this Unit's adapters (F6)** — add tests for the error and retry
    branches of `PgUserRepository` and `PgShreddingKeyRepository` (provision rollback, hash
    collision fail-closed path, `reveal_auth_subject` on a missing row, key destruction on an
    unknown key), inside the existing `#[ignore]` suites. These raise
    `identity-infrastructure`, whose floor is judged per crate and re-measured after u3 adds its
    support-adapter cases; any remaining shortfall is reported, never lowered.
21. [ ] **Repair — coverage of the composition root (F6)** — move the parts of
    `services/identity/src/` that can be exercised without binding a port (configuration parsing,
    router and middleware assembly, health/readiness wiring) behind testable entry points and
    cover them, so the `identity` crate is no longer 0.00 %.
22. [ ] **Repair — record** — update `code-summary.md`, `source-manifest.json` and
    `traceability.json` with the repair and its evidence.

Not in this Unit's repair: F2b (the destroyed pgsodium key still decrypts) is left failing — no
code change can make that assertion true on pgsodium 3.1.8, and the guarantee's wording is a
separate decision.

## Loop-back 1, second pass — `identity-infrastructure` (Build and Test, 2026-11-12)

Step 20's adapter cases lifted `identity-infrastructure` from 53.63 % to **1 716 / 2 642 lines,
64.95 %**, still under the 80 % floor. Step 20 recorded that a shortfall would be reported, never
lowered, and Build and Test's reviewer reopened the point at R-04. This pass closes it with tests
only; steps 1–22 stand as they are.

The uncovered surface, from the coverage export, is concentrated in adapter modules that no test
constructs:

| Module | Covered / lines |
|---|---|
| `infrastructure/src/support/drainer.rs` | 23 / 113 |
| `infrastructure/src/support/sources.rs` | 21 / 110 |
| `infrastructure/src/support/observation.rs` | 11 / 53 |
| the remaining adapter modules (audit client, geo locator, the repositories' error arms, JWKS) | the balance of the 926 uncovered lines |

23. [x] **Baseline.** Re-measure the crate alone and record the exact per-file numbers in
    `code-summary.md` as this pass's before-picture:
    `cargo llvm-cov -p identity-infrastructure -p identity --include-ignored --ignore-run-fail --json target/coverage.json`
    (paired with the `identity` package, which carries the test binaries; adjust the flag form to
    the tool's accepted syntax and record the command actually used).
24. [x] **Support adapters — Red/Green.** Extend the existing `#[ignore]` adapter suite with cases
    that construct and drive the three support modules directly: the drainer pass (one delivered,
    one transient retry, one dead-lettered, one missing-token row) against the outbox store, and
    the Account/Portfolio HTTP sources (success, non-2xx, malformed projection) with a stub
    server. Assert the observable outcome — the row's state, the request the stub received, the
    audit event written — not that a line ran. *Trace:* BR3.7–BR3.13, R-04.
25. [x] **Remaining adapter arms — Red/Green.** Cover the audit client, the geo locator, the JWKS
    adapter and the repository error arms that the export shows as uncovered, asserting the error
    mapping each produces. Skip any arm that cannot be reached without a live external system and
    name it as a gap. *Trace:* F6.
26. [x] **Record.** Re-run step 23's command, put the after-picture beside the before-picture, name
    every path still uncovered and why, update `code-summary.md`, `source-manifest.json` and
    `traceability.json`, and re-run `aidlc engine sensor-traceability --stage build-and-test`.

## Story-to-step traceability

| Story | Steps |
|---|---|
| US1.1–US1.6 | 1, 3–14 |
| US4.1 | 3–5, 9–13 |
| US7.1 | 6–14 |

## Definition of done

All steps checked; the full gate command passes (1377+ tests, fmt, clippy, frontends); the `#[ignore]` adapter suites pass against a disposable Postgres; the migrate path is exercised; coverage targets are reported honestly where pre-existing crates fall short.
