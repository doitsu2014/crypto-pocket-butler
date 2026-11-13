# Code Generation Plan — u7-settings-ui

**Unit:** U7 Settings UI (`u7-settings-ui`) · **Kind:** ui with server additions · **Complexity:** M
**Stories:** US1.4, US4.3, US6.1 · **Requirements:** FR14.4, FR17.3, FR19.1, AC1.4.1, AC1.4.2,
AC4.3.1, AC4.3.2, AC4.3.3, AC6.1.1, AC6.1.2
**Upstream:** `functional-design/functional-spec.md`, `functional-design/frontend-components.md`,
`nfr-design/{performance-design,security-design,logical-components}.md`,
`nfr-requirements/{performance-requirements,security-requirements,tech-stack-decisions}.md`,
`infrastructure-design/{infrastructure-specification,monitoring-design,cicd-pipeline}.md`,
`inception/contract-design/contract-summary.md`, `inception/units-generation/unit-of-work.md`,
`inception/requirements-analysis/requirements.md`

## Summary

- Builds: the four settings panels plus the delete, restore and blocked screens in the SPA; five new self-scoped Identity routes (D0, P1/P2, D1/D2) with their route labels; eight gateway route-table entries.
- Touches: `web/src/features/settings/`, `web/src/features/account-status/`, `web/src/app/router.tsx`, `web/src/api/`, `web/src/components/`, `web/src/i18n/`, `web/src/features/signup-gates/gateStatus.ts`, `services/identity/{domain,application,http}/`, `services/gateway/domain/src/route.rs`.
- Tests: Rust unit + HTTP-boundary + repository suites (about 34 tests across 6 files); Vitest + Testing Library + MSW + jest-axe suites for every rendered state (about 44 tests across 10 files).

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

## How this plan adapts the contract's ordering

The contract's `plan_profile.steps` is the baseline. Three adaptations, each stated rather than
silently dropped:

- **Data model / database behaviour is omitted as a layer.** This unit ships **no schema change and
  no migration** (`infrastructure-design/infrastructure-specification.md`, Database row): every
  column it reads — `users.display_name`, `display_name_overridden`, `email`, `email_verified`,
  `status`, and `deletion_requests` — already exists. There is no Red step to write for a schema
  that is not changing; the columns are exercised by the repository layer, which is where the
  evidence belongs.
- **Repository / data access is conditional in its Green step.** If the projections the reads need
  turn out to exist already (`find_by_auth_subject`, the profile fields), the Red test stays and the
  Green step adds only what is genuinely missing. The plan does not assume a new query exists, and
  it does not write one before a failing test asks for it.
- **Gateway work is its own Red/Green pair inside the API / endpoint layer.** The gateway route
  table is a different service and a different test suite, and its eight declarations are the
  unit's only infrastructure-relevant change; folding it into the Identity handler steps would hide
  it.

Every step lands its tests in the same change as the code it covers (team rule). Each Red step
records the failing command's output before its Green step runs.

## Story-to-step traceability

| Story / requirement | Plan steps |
|---|---|
| AC1.4.1 — provider email and display name shown, override set on save | 9–11 (W1/W2 rules), 12–14 (P1/P2), 15–17 (ProfilePanel) |
| AC1.4.2 — chosen locale and base currency survive a refresh | 15–17 (PreferencesPanel reads the server's value and `source`) |
| AC4.3.1 — restore within 30 days | 9–14 (D1/D2, W3/W4), 15–17 (RestoreAccountPage, DeleteAccountPage) |
| AC4.3.2 — day 31 → deleted | 15–17 (blocked screen, `410` terminal state) |
| AC4.3.3 — pending-deletion refused elsewhere, restore path named | 9–11 (D0 non-denying self-read), 12–14 (D0 route), 15–17 (`gateStatus` routing) |
| AC6.1.1 — locale and base currency save, stored values unchanged | 15–17 (PreferencesPanel sends only the changed fields) |
| AC6.1.2 — withdrawal recorded with a timestamp, visible later | 15–17 (ConsentPanel + consent history) |
| FR14.4 / FR17.3 / FR19.1 — the settings, consent and deletion surfaces | all steps |

## Steps

### Step 1 — Project structure and production configuration skeleton

- [x] Confirm the existing shape and create only the folders the unit needs:
      `web/src/features/settings/` (pages, components, hooks, tests),
      `web/src/features/account-status/` (restore and blocked screens), and the test fixture folder
      the MSW handlers live in.
- [x] Confirm **no new dependency** is added: `web/package.json` already carries React 18,
      react-router-dom 6, TanStack Query 5, Vitest 3, Testing Library, jest-axe and MSW 2
      (tech-stack-decisions T1, spec §8). No Rust dependency is added either — the Identity and
      gateway crates already carry everything these routes use.
- [x] Confirm `services/identity/http/src/routes.rs` has the self-scoped `/v1/me/*` group the new
      routes join, and that `POST /v1/privacy/deletions` and the two export routes already exist
      there (D3 and X7 need only gateway entries).

### Step 2 — Verify the test runners and record the exact unit-scoped commands

- [x] Rust: confirm `cargo test -p identity-http -p identity-application -p identity-domain` and
      `cargo test -p gateway-domain -p gateway-http` run today, and that the database-backed
      repository cases are `#[ignore]`d behind `TEST_DATABASE_URL`.
- [x] Frontend: confirm `yarn test src/features/settings` and `yarn test src/features/account-status`
      (Vitest file filters) run today in `web/`, and that the MSW server and `web/src/test/` helpers
      exist for the new handlers to join.
- [x] Record these exact commands, their filters, and the coverage note in
      `unit-test-instructions.md` before the first Red step. A Red step is invalid without a
      runnable command.

### Step 3 — Repository / data access — Red

- [x] Write failing tests for the projections the reads need, in the Identity repository suite:
      - P1's profile projection resolves a user by **auth subject** and carries display name,
        `display_name_overridden`, provider display name, email, `email_verified` and status.
      - D1's open-request read resolves the caller by auth subject and returns only the request
        whose status is neither cancelled nor erased; no request is a `None`, never an error.
      - D0's status read resolves by auth subject for **every** status, including `disabled`,
        `pending_deletion` and `deleted`.
- [x] Run the suite and record the failing output (the methods do not exist yet).

### Step 4 — Repository / data access — Green

- [x] Add only the repository methods the failing tests name. Where the existing
      `find_by_auth_subject` and deletion lookup already answer, reuse them and add the projection
      as the thin query it needs — no new table, no new index, no migration.
- [x] Re-run the repository suite until green.

### Step 5 — Repository / data access — Refactor

- [x] Keep the statements in the existing adapter style (sqlx, typed errors, `//!` header citing
      the BR ids), with the tests staying green. No behaviour change.

### Step 6 — Business logic — Red

- [x] Write failing application-layer tests against port fakes:
      - **W1** trims, refuses empty and over-200 with a field-scoped error, stores the value, sets
        `display_name_overridden = true`, and emits one `identity.user.profile.updated` audit event
        with `{ field: "displayName", overridden: "true" }`.
      - **W2** adopts the forwarded provider name and clears the flag with `overridden: "false"`;
        with no provider name in the claim it refuses `409` and changes nothing.
      - **D0** answers every status without consulting `UserStatus::denies_use`, and writes no
        audit event.
      - **W3/W4** delegate to the existing `cancel_deletion`, mapping "no open request" to `409` and
        "window closed" to `410`; no new state machine is written.
- [x] Run the application suite and record the failing output.

### Step 7 — Business logic — Green

- [x] Implement the use cases (display-name save, provider-name adoption, status read, open-request
      read, cancel delegation) with the owner resolved from the JWT claims only.
- [x] Re-run until green.

### Step 8 — Business logic — Refactor

- [x] Keep the error types typed per service (`DomainError` inward, `AppError` at the boundary) and
      the audit emission at the same point the existing use cases emit theirs. Tests stay green.

### Step 9 — API / endpoint — Red (Identity)

- [x] Write failing HTTP boundary tests over the Identity router:
      - D0 `GET /v1/me/account-status` → `200 {status}` for each of the four statuses, `401`
        without a token.
      - P1 `GET /v1/me/profile` → `200` with the exact field set; `401`; `503` when the store is
        unavailable.
      - P2 `PATCH /v1/me/profile` → `200` returning the P1 shape; `422` for both-or-neither and for
        an invalid name; `409` when the provider name is unavailable.
      - D1 `GET /v1/me/deletion-request` → `200` with the request; `404` carrying
        `https://crypto-pocket-butler.example/problems/no-record`.
      - D2 `DELETE /v1/me/deletion-request` → `200`; `409`; `410`.
      - No route accepts a user id: a request carrying another person's id in any form is answered
        from the caller's own record.
      - `route_label` returns the new labels for the six settings paths rather than `"other"`.
- [x] Run the HTTP suite and record the failing output.

### Step 10 — API / endpoint — Green (Identity)

- [x] Add the handlers, the DTOs, the router entries in the `/v1/me/*` group, and the `route_label`
      arms (infrastructure-specification "What must exist", route-labels row — without them every
      new route is counted as `route="other"` and the settings SLIs cannot be computed).
- [x] Register the pinned no-record problem type so D1's empty answer is distinguishable from any
      other `404`.
- [x] Re-run until green.

### Step 11 — API / endpoint — Refactor (Identity)

- [x] Keep DTO→domain mapping at the `http` boundary, RFC 7807 rendering in `problem.rs`, and no
      `detail` text in any response the SPA renders. Tests stay green.

### Step 12 — API / endpoint — Red (gateway)

- [x] Write failing gateway tests:
      - Each of the eight paths — `GET /v1/me/account-status`, `GET /v1/me/profile`,
        `PATCH /v1/me/profile`, `GET /v1/me/deletion-request`, `DELETE /v1/me/deletion-request`,
        `POST /v1/privacy/deletions`, `POST /v1/exports`, `GET /v1/exports/{exportId}` — resolves to
        a handler instead of `404` at the deny-by-default table.
      - `POST /v1/privacy/deletions` carries a per-minute budget and answers `429` on the sixth
        request in a window.
      - Every other undeclared path still `404`s (the table stays deny-by-default).
- [x] Run the gateway suite and record the failing output.

### Step 13 — API / endpoint — Green (gateway)

- [x] Add the eight entries (all `requires_admin: false`, all Identity) and the 5/min budget on D3.
- [x] Re-run until green.

### Step 14 — API / endpoint — Refactor (gateway)

- [x] Keep the table's existing ordering and comment style; no change to the rate limiter's keying
      or to any other route. Tests stay green.

### Step 15 — Frontend behaviour — Red

- [x] Write failing Vitest + Testing Library + MSW tests, per the components spec §6, before the
      components exist:
      - `client.ts`: `getAccountStatus`, `getProfileView`, `saveDisplayName`, `useProviderName`,
        `getOpenDeletion` (404 → `null` only for the pinned type), `cancelDeletion`, and
        `requestDeletion` posting to `/v1/privacy/deletions` with no body; `409`/`503` mapping;
        the dead calls removed.
      - `ProfilePanel`: exact PATCH body and re-render from the response; `422` keeps the typed
        value; reset action hidden/disabled/`409` hint.
      - `PreferencesPanel`: changed subset only; per-field `422`; `source` labels; a read failure
        leaves the nav rendered.
      - `ConsentPanel`: grant with the policy version; withdraw; `409` re-reads; required kinds
        read-only and dated; withdrawal date only after a withdrawal.
      - `DeleteAccountPage`: consequences; the dialog gates D3; `202` swaps state; the exact date;
        empty vs non-empty `retainedCategories`.
      - `RestoreAccountPage`: exact date in the profile locale; D2 then `/settings/profile`; `410`
        terminal; D1 `404` re-resolves.
      - `AccountBlockedPage`: both variants, Sign out only.
      - `gateStatus`: `pending_deletion → restore-account`, `disabled`/`deleted → account-blocked`.
      - `StatusPill`: every status by label + shape, pending carries the date as text.
      - jest-axe on every rendered state.
- [x] Run the new files and record the failing output.

### Step 16 — Frontend behaviour — Green

- [x] Implement `web/src/api/types.ts` additions, the `client.ts` methods and removals, the hooks,
      the two feature folders' pages and components, `SettingsLayout` + `SettingsNav` (a bare
      `<Outlet/>`, **no** `QueryBoundary` — its error branch renders `ErrorPanel`, which renders
      `ProblemDetails.detail`), the `ConfirmDialog` focus trap extension, `StatusPill`, the
      `gateStatus.ts` extension, the router wiring (`/settings`, `/settings/delete`,
      `/account/restore`, `/account/blocked`, `/privacy` redirect), and the EN + VI
      `settings.*` strings.
- [x] Add `data-testid` attributes to interactive elements for test automation.
- [x] Re-run until green.

### Step 17 — Frontend behaviour — Refactor

- [x] Remove the dead shell code the spec retires (the old profile/preferences/consent panels, the
      sessions panel and its `useSessions()` call, and the `DestinationUnavailable` fallback for the
      two U7 destinations). Parchment Ledger tokens only — no raw colour, spacing or type value;
      missing values render as an em dash. Tests stay green, and `yarn test` for the whole `web/`
      package stays green.

### Step 18 — Environment / build configuration

- [x] Confirm every new page module is named `*Page` and lands as a lazy route chunk so the
      existing `web/scripts/check-size.mjs` glob and size-limit budgets keep measuring it. No new
      config entry, no threshold changed.
- [x] Run the format, lint and build gates the unit touches: `cargo fmt --all -- --check`,
      `cargo clippy --workspace --all-targets -- -D warnings`, `bash scripts/check-boundaries.sh`,
      `yarn lint`, `yarn format:check`, `yarn build`, `yarn size` in `web/`. `cargo test
      --workspace` runs in full. No gate is relaxed to make a step pass; a failure is surfaced.

### Step 19 — Documentation and traceability

- [x] Write `code-summary.md` (files created/modified, key decisions, test evidence, any test not
      executed and why, deviations from this plan).
- [x] Write `source-manifest.json` listing every application-source path this unit created,
      modified or deleted.
- [x] Generate `traceability.json` with
      `aidlc engine sensor-traceability --stage code-generation --output-path <path> --write-upstream`
      and fill every row so each `OK` target names an existing workspace-relative implementation or
      test file.
- [x] Add inline `//!` module headers citing the BR ids the new Rust modules serve.

## Out of scope for this build

- Contract-summary and C4-model updates for the new paths: the project rule says they move with the
  change that lands them, and they are recorded as this unit's external dependency
  (`infrastructure-specification.md`, "What must exist"). This plan does not edit
  `docs/architecture/` — the paths do not change a container, a schema, an external system or a
  relationship.
- The retention and legal-hold policy text (BRD-012 "must define before launch"), the analytics
  policy-version endpoint, and the `/v1/me/principal` mismatch — all recorded as open items in
  `functional-spec.md` §12 and untouched here.
- `RoleBadge` and `GrantBanner`: they belong to U9/U10 in `support-console/`.