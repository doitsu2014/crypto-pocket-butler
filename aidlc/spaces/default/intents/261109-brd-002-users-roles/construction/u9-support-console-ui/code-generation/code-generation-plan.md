# Code Generation Plan — u9-support-console-ui

## Summary

- Builds: the support console's grant request, session banner, metadata panel and own-grant recovery, plus the `GET /v1/support-grants/mine` server half it depends on.
- Touches: `support-console/src/{api,domain,hooks,components,screens}`, `services/gateway/{domain,http}`, `services/identity/{application,infrastructure,http}` and one migration pair, plus three comment/doc edits.
- Tests: 10 console suites (Vitest 3, Testing Library, jest-axe, MSW 2) and 5 Rust test targets, written test-first under the TDD contract below.

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

## Scope: the S2 decision this plan takes

`GET /v1/support-grants/mine` (S2) does not exist — no contract path, no gateway `Route` entry, no Identity handler, no `route_label` arm. `nfr-requirements/tech-stack-decisions.md` T1 assigned it to U3, and **U3's work has already landed without it**, so no open change delivers it. The infrastructure design records that as a release blocker and names three ways out: reopen U3, raise a new unit, or fold the route into this unit's Code Generation.

**This plan takes the third option**, because it is the only one this workflow can execute: reopening a shipped unit or raising a new one is a Units Generation / delivery action outside this Unit, and the console cannot be built, tested or shipped without the read. It is also what the functional design already scoped — `functional-design/frontend-components.md` §7 lists exactly these Identity and gateway files under "only the W4 read". The deviation is from T1's ownership assignment only; the code shape is the one functional-spec §8 S2 and contract C5 already describe.

If the person prefers the server half to be a separate unit, this plan's Steps 16–22 and the Rust rows of Step 23 are the block to strike; Steps 1–15 and 24 stay valid alone, but every S2-dependent test in them then has no server half to run against and the unit cannot ship.

## Steps

### Step 1 — Verify the test runner and record the exact commands (no production code)

- [x] Confirm `support-console` still runs under `yarn --cwd support-console test` (Vitest 3), and record in `unit-test-instructions.md` the file-scoped form used for every Red step, e.g. `yarn --cwd support-console test src/domain/grants.test.ts`.
- [x] Confirm the Rust targets the S2 work uses — `cargo test -p gateway --test domain`, `cargo test -p gateway --test http`, `cargo test -p identity --test support_application`, `cargo test -p identity --test support_http` — and note that `cargo test -p identity --test support_adapters -- --ignored` needs `TEST_DATABASE_URL` and is a blocking gate, not an opt-in.
- [x] Traceability: functional-spec §8 S1–S3; team Testing Posture (runner-ready before the first Red).

### Step 2 — Frontend Red: domain rules (W2, W5)

- [x] `src/domain/grants.test.ts`: the reason bound is **1–500** characters (both ends, and a 501-character reason fails); the four statuses render from `grantStatusPresentation`; the clock-derived status expires at `expiresAt`; a `null` or unparseable `expiresAt` on an `active` grant is **expired** (fail closed); "less than a minute left" at zero remaining.
- [x] `src/domain/metadata.test.ts`: `pickSupportMetadata` keeps exactly the whitelist, drops over-sent fields, maps a missing display name to `null`, and **throws** on a malformed projection.
- [x] Record the failing output. Traceability: W2, W5; NFR2.5, NFR22.1, NFR22.2, NFR24.1, NFR24.2, NFR25.3, NFR26.1, NFR26.3.

### Step 3 — Frontend Green: domain and types

- [x] `src/domain/grants.ts`: replace `MIN_REASON_LENGTH = 20` with the spec's 1–500 bound; keep one presentation table for four statuses.
- [x] `src/domain/metadata.ts`: the whitelist picker and the throwing path.
- [x] `src/api/types.ts`: add `GrantStatus`, `SupportAccessGrant`, `GrantRequestInput`, `SupportMetadata`; remove `AccountMetadata`, `AccountSearchResult`, `MetadataReadInput`, `AccountSearchInput`, `GrantRequestValues`.
- [x] Traceability: W2, W5; NFR22.1, NFR22.2, NFR24.1.

### Step 4 — Frontend Red: the client (W1, W2, W3, W7)

- [x] `src/api/client.test.ts` (extend; its retired-call cases are deleted as part of the same edit): `requestSupportGrant` sends exactly `{userId, reason}` (no duration, no approver); `listMyGrants` calls `GET /support-grants/mine` with a 15 s deadline and maps an empty list; `fetchSupportMetadata` calls `GET /support-grants/{grantId}/metadata` and maps through the picker; `pickSupportGrant` accepts `grantId ?? id`; a malformed projection rejects; `getSupportGrant`, `searchAccounts`, `fetchAccountMetadata` are gone. `src/api/client.admin.test.ts` is untouched.
- [x] Traceability: W1, W2, W3, W7; NFR2.2, NFR2.8.

### Step 5 — Frontend Green: the client

- [x] `src/api/client.ts`: add `listMyGrants`, `fetchSupportMetadata` (both `deadlineMs: 15_000`); `pickSupportGrant` key rule; remove the three retired functions and the `X-Support-Reason` header.
- [x] Traceability: W1, W2, W3, W7; NFR9.2.

### Step 6 — Frontend Red: cache policy (NFR10.4, NFR2.10)

- [x] `src/api/queryClient.test.ts` (extend) and `src/hooks/useClearSupportCacheOnErrors.test.tsx`: `clearSupportQueries` removes only `["support", …]` keys; the new `useClearSupportQueriesOnRouteErrors` clears everything on `401` and only the support prefix on `403`; **the existing `useClearSupportCacheOnErrors` still clears everything on both** (U10's admin callers keep their behaviour).
- [x] Traceability: NFR10.4, NFR2.10, NFR22.5.

### Step 7 — Frontend Green: cache policy

- [x] `src/api/queryClient.ts`: add `clearSupportQueries` beside `clearSupportCache`.
- [x] `src/hooks/useClearSupportCacheOnErrors.ts`: add the second exported hook; leave the first untouched.

### Step 8 — Frontend Red: the two query hooks (W4, W3)

- [x] `src/hooks/useMyGrants.test.tsx`: disabled without a token and for a non-support role; key `["support", "grants", "mine"]`; `staleTime: 0`; explicit `refetchOnWindowFocus: false` and `refetchInterval: false` (focus and timers issue no request); `retry: false`; a route-level `403` clears the support cache.
- [x] `src/hooks/useSupportMetadata.test.tsx`: enabled only when the clock-derived status is `active`; `staleTime: 10_000`; one read per grant plus Refresh; a `403` calls `onAccessEnded` and drops the key.
- [x] Traceability: W4, W3; NFR2.3, NFR2.4, NFR2.7, NFR2.9, NFR2.10.

### Step 9 — Frontend Green: the two hooks

- [x] `src/hooks/useMyGrants.ts`, `src/hooks/useSupportMetadata.ts` — nothing else in the hooks folder moves.

### Step 10 — Frontend Red: components (W1, W5, W6)

- [x] `GrantRequestForm.test.tsx`: happy path (one request per submit, Submit disabled in flight, success line from the `201`, fields cleared); empty id; empty reason; 501-character reason; `409` offers **Check your current grant**; `404`; `403` uses the neutral not-allowed line and never `ProblemDetails.detail`; `422` per-field messages; `429` shows the `Retry-After` seconds with no automatic retry; jest-axe on every state.
- [x] `GrantBanner.test.tsx`: the four status variants with remaining-minutes text; expired clock; the metadata-only reminder; the check-status and request-new callbacks.
- [x] `SupportMetadataPanel.test.tsx`: happy path renders user, accounts table and portfolios; empty accounts and empty portfolios; the filter narrows without a request; `403` → access ended and `onAccessEnded`; `401`; `503` with **Refresh**; no read while the status is not active; the read-audit statement; jest-axe on every state.
- [x] `GrantApprovalQueue.test.tsx`: approve disabled for a non-admin viewer, enabled for a different admin, self-approval copy.
- [x] Traceability: W1, W5, W6; NFR2.1, NFR2.8, NFR2.11, NFR22.4, NFR22.6, NFR24.3, NFR25.1, NFR25.2, NFR25.5.

### Step 11 — Frontend Green: components

- [x] Rework `GrantRequestForm.tsx` (no duration, no approver, `onCheckStatus` from the shell) and `GrantBanner.tsx` (session grant only, `now` from the shell).
- [x] Add `SupportMetadataPanel.tsx` (owns the filter text; mounted only with a live grant).
- [x] Adjust `GrantApprovalQueue.tsx` for the non-admin disabled state.
- [x] **Remove** `AccountSearch.tsx` and `.test.tsx`, `AccountMetadataPanel.tsx` and `.test.tsx`, `useSupportGrant.ts` and `.test.tsx`, and confirm no surviving import references them (W7).
- [x] Traceability: W1, W5, W6, W7.

### Step 12 — Frontend Refactor while green

- [x] Share the message constants the form, banner and panel now repeat (`src/domain/presentation.ts` or the component's own module — one home, no second presentation table); remove exports nothing uses; keep every suite green.

### Step 13 — Frontend Red: the shell (W4)

- [x] `SupportShell.test.tsx`: on mount a support session adopts the newest grant from `mine` **without comparing `requestedBy`** (the list is already caller-scoped, and `requestedBy` is Identity's internal id, not the Supabase auth id); a just-created grant is adopted from the `201`; an admin approving **another agent's** grant never becomes the admin's banner; the `search` section is renamed `metadata` and a `#search` hash still lands; a support session never sees approvals; an admin sees the admin link instead of the form; sign-out clears the session and cache.
- [x] Traceability: W4, W6; NFR2.10, NFR10.4, NFR24.4.

### Step 14 — Frontend Green: the shell

- [x] `src/screens/SupportShell.tsx`: session grant state, shared clock, the `mine` read and adoption rule, the section map change, role gating, `onAccessEnded`; stop holding the unused pending list.
- [x] Traceability: W4, W6.

### Step 15 — MSW handlers

- [x] `src/test/` handlers for `POST /support-grants`, `GET /support-grants/mine`, `GET /support-grants/{grantId}/metadata`, including delayed, `409`, `403`, `429` and `503` variants; dev-only, never in the production bundle. No handler asserts a route outside functional-spec §8.

### Step 16 — Rust Red: the gateway route (S2)

- [x] `services/gateway/tests/domain.rs`: the table declares `GET /v1/support-grants/mine` with `requires_admin: false` and **no** per-route budget, and it is matched before any broader `/v1/support-grants` pattern (first-match).
- [x] `services/gateway/tests/http.rs`: the path forwards to Identity with the caller's unrouted secret and the JWT forwarded unchanged; an undeclared sibling path is still `404`.
- [x] Run each as `cargo test -p gateway --test domain` and `cargo test -p gateway --test http`.
- [x] Traceability: functional-spec §8 S2; NFR10.3, NFR11.1.

### Step 17 — Rust Green: the gateway route

- [x] `services/gateway/domain/src/route.rs`: one `Route` entry.
- [x] Traceability: S2; NFR10.3.

### Step 18 — Rust Red: the Identity store read (repository layer)

- [x] `services/identity/tests/support_application.rs` (extend), against the port fakes in `tests/common/`: `mine` returns only the caller's grants, newest first, at most 20 (`MY_GRANTS_LIMIT`), and an empty list (not an error) for an agent with none.
- [x] `services/identity/tests/support_adapters.rs` (extend), `#[ignore]`d and run against the disposable Postgres: the same three cases against the real table, including that a 21st grant is dropped.
- [x] Run as `cargo test -p identity --test support_application` and, with `TEST_DATABASE_URL` set, `cargo test -p identity --test support_adapters -- --ignored`.
- [x] Traceability: S2; NFR2.3.

### Step 19 — Rust Green: the store read

- [x] `services/identity/application/src/ports.rs` (the read, if absent), `application/src/support.rs` (the use case), `infrastructure/src/repositories/support.rs` (`list_by_support_agent(&UserId, u32)`), `MY_GRANTS_LIMIT = 20` beside the existing support constants.

### Step 20 — Rust Red: the Identity HTTP route (API layer)

- [x] `services/identity/tests/support_http.rs` (extend): `GET /v1/support-grants/mine` answers `200` with a **bare array** of grant rows ≤ 20; `403` for a signed-in caller without the support role; `401` without a token; the `route_label` arm names the route (today the same path would be counted as a revoke, `services/identity/http/src/middleware.rs:140`).
- [x] Run as `cargo test -p identity --test support_http`.
- [x] Traceability: S2; NFR10.2, NFR10.3, NFR24.3.

### Step 21 — Rust Green: the Identity HTTP route

- [x] `services/identity/http/src/handlers.rs`, `dto.rs` (`Vec<PendingGrantResponse>`, no envelope), `routes.rs` (registration), `middleware.rs` (`/v1/support-grants/mine` arm), plus the contract's `requestedBy` on the bare-array row.

### Step 22 — Migration pair

- [x] `services/identity/migrations/0008_support_mine_index.{up,down}.sql` — the next number after `0007_support_access`: `CREATE INDEX support_grants_mine_idx ON support_access_grants (support_agent_id, requested_at DESC)` and its `DROP INDEX`. The shipped `support_grants_read_idx` leads on `user_id` and `support_grants_live_pair_idx` is partial, so neither serves a status-free recency read. No other schema change.
- [x] One `#[ignore]`d migration case in `support_adapters.rs`: apply `0008` up, reverse it, re-apply it.
- [x] Traceability: NFR23.1.

### Step 23 — Documentation and traceability

- [x] `support-console/README.md:29,33` — the retired `X-Support-Reason` header and the console's route list.
- [x] `web/nginx.conf:7-9` — the log-format comment that still names the retired header (comment only).
- [x] `services/gateway/infrastructure/src/upstream.rs:88-90` — the comment naming the retired header (comment only).
- [x] `inception/contract-design/contract-summary.md` — C1/C2 and C5 entries for the `mine` path (bare array, `requestedBy`, ≤ 20, newest first) and a C1/C2 entry for the shipped `GET /v1/support-grants/{grantId}/metadata`, which no external surface declares today.
- [x] `docs/architecture/README.md` / `workspace.dsl`: **no change** — a route addition is not a container or relationship change (`docs/architecture/README.md` §13).
- [x] Traceability: NFR9.2.

### Step 24 — Gates

- [x] Console: `yarn --cwd support-console format:check`, `lint`, `build`, `test`.
- [x] Rust: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` with the disposable Postgres, `bash scripts/check-boundaries.sh`.
- [x] A failing gate is reported, never relaxed: no threshold, test or lint rule is lowered to pass.

## Out of scope

- Any second grant surface (an admin-facing by-id read, a support-facing user search): functional-spec §12 items 3–5 record them as open.
- A `denied` status: no producer exists.
- i18n for NFR26.2: deferred (decision `ab46b27b`), no owner unit.
- Console telemetry, a size budget script, CI configuration and vulnerability scanning: platform gaps recorded in the infrastructure design, not this unit's.
- Any `infra/swarm-stack.yml` change: none is needed, and none is made.

## Risks

- **S2 is the plan's only external dependency and it now sits inside the plan.** If the person prefers it as its own unit, Steps 16–22 leave and the console half ships later.
- **The build's own browser tests are MSW-backed.** They prove the client contract, not the live gateway; the identity smoke script cannot cover S2 (no support-role fixture), so a manual pass under a real support session is the only live check until that fixture exists.