# Code Generation Plan — u10-admin-users-ui

## Summary

- Builds: the support console's admin users section (exact-email lookup, record, role change, disable/restore, pending-grant approvals) plus its server surface in the gateway and Identity
- Touches: `support-console/src/` (new `screens/admin/`, `components/RoleBadge.tsx`, additive `api/client.ts` and `api/types.ts`, `screens/SupportShell.tsx`), `services/gateway/domain/src/route.rs`, `services/gateway/domain/src/path.rs` (if the match arm needs it), `services/identity/application/`, `services/identity/http/`, `services/identity/infrastructure/`, and the D-S1 contract ripple in `docs/`
- Tests: TDD, Standard strategy — Rust unit and HTTP-boundary suites for each touched crate plus one `#[ignore]` adapter test, and seven Vitest suites (≈40 cases) with jest-axe in every rendered state

**Story traceability.** Every step serves US7.1. AC7.1.1 (search → open record → change role → disable) is carried by steps 2–4 (lookup widening and role read), 5–7 (disable/restore route and handler), 11–15 (lookup, record and the two change forms) and 16–17 (section and shell). AC7.1.2 (no entitlement control) is carried by step 17 (the record renders no entitlement field or control) and by the gateway step 2, which declares no entitlement path — an undeclared path is `404`. FR20.1/FR15.4 (view and approve support grants) are carried by step 8 (pending-grant list read) and step 16 (approvals panel). NFR24–NFR26 are carried by the console steps (provider-fresh role, memory-only session, ARIA/live regions, `Intl` formatting).

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

**How this contract maps onto this unit.** The unit is brownfield and touches four testable layers: repository/data access (the grant-list read), business logic (the admin lookup use case and the role read port), API/endpoint (gateway `Route` table and Identity handlers), and frontend behavior (the console). There is no data-model change — the required status index on `support_grants` is U3's migration, not this unit's. TDD applies per layer: each production change below is paired with a step that writes the failing test first and records the failing output, then the minimal implementation, with the refactor pass inside the same layer.

## Steps

### Step 1 — Verify the runners and record the unit-scoped commands

- Run the existing console suite (`yarn --cwd support-console test`) and the two Rust suites (`cargo test -p gateway-domain -p identity-application -p identity-http`) to confirm they are green before the first change; record the exact commands in `unit-test-instructions.md`.
- No production code changes.

### Step 2 — Gateway route table: Red

- Add failing tests in `services/gateway/tests/domain.rs` and `services/gateway/tests/http.rs`:
  - `POST /v1/admin/users/lookup` matches, is admin-only, and answers a per-route budget of 60.
  - `PUT /v1/admin/users/{userId}/role` still matches and now answers a budget of 20 **from the first matching entry** — a duplicate later entry must not shadow it (assert the budget the table returns, not just that the path matches).
  - `POST /v1/admin/users/{userId}/disable` and `…/restore` match, admin-only, budget 20.
  - `GET /v1/support-grants` matches, admin-only, budget 60.
  - `GET /v1/admin/users` no longer matches (the address-carrying U1 lookup is gone).
- Record the failing output.

### Step 3 — Gateway route table: Green

- Edit `services/gateway/domain/src/route.rs`: remove the `GET /v1/admin/users` declaration, add `POST /v1/admin/users/lookup`, `POST …/{userId}/disable`, `POST …/{userId}/restore`, `GET /v1/support-grants`, and set `max_requests_per_minute: Some(20)` on the existing `PUT /v1/admin/users/{userId}/role` entry.
- If `PathPattern` or the matcher needs a new arm for the literal `lookup` segment, change `services/gateway/domain/src/path.rs` with its own test in `services/gateway/tests/domain.rs` (written first).
- No stack, config or replica change.

### Step 4 — Role read port: Red

- In `services/identity/application/tests/` (the authorisation suite), add failing tests with a fake reader: `RoleReader::read_role` returns the provider's `Role`; a provider error surfaces as `ExternalError` and the caller maps it to `AppError::Unavailable`.
- Record the failing output.

### Step 5 — Role read port: Green

- Add `RoleReader` beside `RoleWriter` in `services/identity/application/src/authorisation/ports.rs`.
- Implement it on `SupabaseRoleWriter` in `services/identity/infrastructure/src/authorisation/role_writer.rs` (or rename the struct to the adapter's real role while keeping its exported name stable), reusing the same Supabase Auth client, connect 1 s / total 3 s, one attempt.
- Add the `#[ignore]`d adapter test that reads a role back after a write against `TEST_DATABASE_URL`/provider test doubles, following the existing adapter-test pattern.

### Step 6 — Admin lookup widening: Red

- In `services/identity/application/tests/` add failing tests for the new admin lookup:
  - `POST /v1/admin/users/lookup` body `{"email"}` resolves the subject with `reveal_auth_subject` and returns `AdminUserView` carrying `role` from `RoleReader`.
  - A provider read failure returns `AppError::Unavailable` **and** records `AuditOutcome::Failed` with a `provider-unavailable` context before returning — the success event is not written.
  - A successful lookup still records exactly one `identity.admin.user_lookup` `Success` event, after the role read.
  - A non-admin caller is denied and audited as today.
- Record the failing output.

### Step 7 — Admin lookup widening: Green

- Add `AdminUserView` (application) and its `role` field in `services/identity/application/src/user_registry.rs`; add `AdminLookup::by_email_admin(&Caller, &str) -> Result<AdminUserView, AppError>` keeping `by_email` for the owner read.
- Do not widen `UserRecordView`; the shared C3 shape stays as it is.

### Step 8 — Pending-grant list read: Red

- Failing tests in the application suite (port fakes): `SupportGrantRepository::list_by_status(PendingApproval, 100)` returns newest-first bounded rows; a non-admin caller is refused; another `status` value is refused by the handler layer (step 10).
- Failing `#[ignore]`d adapter test for the SQL read against the disposable Postgres schema.
- Record the failing output.

### Step 9 — Pending-grant list read: Green

- Add `list_by_status` to the `SupportGrantRepository` trait and its sqlx implementation, newest-first, `LIMIT 100`, reading only U3's table under Identity's own role and DSN. No migration, no grant rule.

### Step 10 — HTTP boundary and routers: Red

- Failing HTTP tests over the Identity router (`services/identity/http/tests/` or the crate's existing boundary suite):
  - `POST /v1/admin/users/lookup` → `200` with the admin DTO carrying `role`; `403` non-admin; `404` unknown address; `422` malformed/absent email; `503` provider failure.
  - `POST /v1/admin/users/{userId}/disable` and `…/restore` → `200` C7 answer; `409` outside the BR5.1 edge; `422` reason; `403`; `404`.
  - `GET /v1/support-grants?status=pending_approval` → `200` array; `422` any other status; `403` non-admin.
  - `GET /v1/admin/users` → `404`/`405` (the handler is no longer registered on that path).
  - A middleware test asserting the new admin paths map to their own `route_label` arms instead of `other`.
- Record the failing output.

### Step 11 — HTTP boundary and routers: Green

- Add `AdminUserResponse` in `services/identity/http/src/dto.rs` (carrying `role`) and the `lookup_admin_user` handler in `handlers.rs`.
- Register in `services/identity/http/src/routes.rs`: replace `GET /v1/admin/users` with `POST /v1/admin/users/lookup`; register `disable_account`/`restore_account` on a new JWT-authenticated `/v1/admin/users/{userId}/disable|restore` router that re-checks the admin role from the forwarded JWT (the existing handlers stay on their internal service-token routes); add `GET /v1/support-grants` to the list handler.
- Add the `route_label` arms for the four admin paths in `services/identity/http/src/middleware.rs`.

### Step 12 — Console client, types and deadline: Red

- Add `support-console/src/api/client.admin.test.ts` with failing cases:
  - the five calls build the documented method, path, body and `Authorization` header; the reason never appears in a URL; `lookupAdminUser` sends a JSON body to `/admin/users/lookup`.
  - `listPendingGrants` sends `status=pending_approval`.
  - a `403` and a `503` surface as `ApiError` with the correlation id.
  - `deadlineMs` aborts with `DeadlineError` and a settled call does not abort a later one.
  - a `429` with `Retry-After` (seconds and HTTP-date) sets `retryAfterSeconds`; an unparseable one leaves it `undefined`.
- Record the failing output.

### Step 13 — Console client, types and deadline: Green

- `support-console/src/api/types.ts`: add `ConsoleRole`, `AdminUserStatus`, `AdminUserRecord`, `RoleChangeBody`, `RoleChangeResult`, `AdminStatusBody`, `AdminStatusResult`; add `errors`/`retryAfterSeconds` where the design puts them.
- `support-console/src/api/client.ts`: add `PUT` to `RequestOptions["method"]`; add the optional `deadlineMs` with `AbortController` inside `request()` and `DeadlineError` in its catch; parse `Retry-After` in `parseProblem`; add `lookupAdminUser`, `changeUserRole`, `disableUser`, `restoreUser`, `listPendingGrants`, all passing `deadlineMs: 15_000`. `getSupportGrant` stays — it is used by `useSupportGrant` and both its suites, so it is not deleted (nfr-design logical-components §1.1 correction).

### Step 14 — UI components: Red

- Failing suites per `frontend-components.md` §6: `RoleBadge.test.tsx`, `AdminUserLookup.test.tsx`, `AdminUserRecord.test.tsx`, `RoleChangeForm.test.tsx`, `StatusChangePanel.test.tsx`, `AdminApprovals.test.tsx`, `AdminUsersSection.test.tsx` — with MSW handlers for A1′/A2/A5′/D1/D2 including delayed, `429`, `403`, `404`, `409` and `503` variants, and `jest-axe` on every rendered state.
- Cover: exact-email validation before any request; `404` "No user has that address."; `403` session line with correlation id; `503` no record and no stale role; role transition + "This change is audited." + re-read; per-field `422`; disable confirmed only from Confirm (zero requests before it); Escape cancels and returns focus; restore offered only for `disabled`; `pending_deletion`/`deleted` explanatory line and no action; self-account warning on role change and disable; approvals never render an empty list on failure; approve/revoke refetch and call `onGrantChanged`; the section is absent for `support` and a hand-typed hash falls back.
- Record the failing output.

### Step 15 — UI components: Green

- Create `support-console/src/screens/admin/AdminUsersSection.tsx`, `AdminUserLookup.tsx`, `AdminUserRecord.tsx`, `RoleChangeForm.tsx`, `StatusChangePanel.tsx`, and `support-console/src/components/RoleBadge.tsx`.
- Add `useAdminLookup`, `usePendingGrants`, `useChangeRole`, `useSetUserStatus` (beside the existing hooks) with the explicit options of performance-design §3 (`staleTime: 0`, `retry: false`, `refetchOnWindowFocus: false`, `refetchInterval: false`), the unmount `removeQueries({queryKey: ["admin"]})`, and `useClearSupportCacheOnErrors` wired per hook.
- Shell wiring in `SupportShell.tsx`: widen `SectionId` with `admin-users`, add the `SECTIONS` entry with `adminOnly: true`, render the lazy section inside `Suspense`, and widen `visibleSection` to read `adminOnly` instead of naming `incidents`.
- Tokens only (`ui.ts` class helpers), `StatusBadge` and `RoleBadge` labels + shapes, `role="status"` announcements, confirm-block focus management, `data-testid`s from `frontend-components.md` §4.

### Step 16 — Refactor pass (tests stay green)

- Within each touched layer, remove duplication, keep the client module the only URL owner, keep `request()` behaviour for every existing caller byte-identical, and re-run the layer's suite after each change.

### Step 17 — Quality gates and the D-S1 contract ripple

- Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `bash scripts/check-boundaries.sh`, `cargo test --workspace`, `yarn --cwd support-console test/lint/build/format:check`; report any failure rather than adjusting a threshold.
- Land the D-S1 ripple in the same change: `functional-spec.md` §3.1/§8 (A1′ → `POST /v1/admin/users/lookup` with a body), `frontend-components.md` §3.1 (`lookupAdminUser` POST shape), `contract-summary.md` C1/C2, C3, C5 and C7 additions, the C4 model (`docs/architecture/workspace.dsl`) and `docs/architecture/README.md` together.

### Step 18 — Documentation and traceability

- Write `code-summary.md` (files, decisions, test coverage, tests written but not executed with the reason) and `source-manifest.json` listing every application source path created, modified or deleted.
- Generate `traceability.json` with `aidlc engine sensor-traceability --stage code-generation --output-path <record>/construction/u10-admin-users-ui/code-generation/traceability.json --write-upstream`, fill every row with an existing workspace-relative file as its `OK` target, and fire the sensor.

## Progress

- [x] Step 1 — Verify the runners and record the unit-scoped commands
- [x] Step 2 — Gateway route table: Red
- [x] Step 3 — Gateway route table: Green
- [x] Step 4 — Role read port: Red
- [x] Step 5 — Role read port: Green
- [x] Step 6 — Admin lookup widening: Red
- [x] Step 7 — Admin lookup widening: Green
- [x] Step 8 — Pending-grant list read: Red
- [x] Step 9 — Pending-grant list read: Green
- [x] Step 10 — HTTP boundary and routers: Red
- [x] Step 11 — HTTP boundary and routers: Green
- [x] Step 12 — Console client, types and deadline: Red
- [x] Step 13 — Console client, types and deadline: Green
- [x] Step 14 — UI components: Red
- [x] Step 15 — UI components: Green
- [x] Step 16 — Refactor pass (tests stay green)
- [x] Step 17 — Quality gates and the D-S1 contract ripple
- [x] Step 18 — Documentation and traceability

## Notes on scope that stays outside this unit

- The status index on `support_grants` is U3's migration (required dependency, not built here).
- No new dependency is added (`msw` is already a console devDependency).
- The production `/v1` edge is not defined in this repository; the console's same-origin `/v1` assumption is unchanged and D-S1 keeps the address out of any URL regardless of that edge.