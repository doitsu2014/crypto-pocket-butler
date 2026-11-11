# Unit Test Instructions — u8-signup-gates-ui

Companion to `code-generation-plan.md`. Every command here is **unit-scoped** — run these during
the Red/Green cycles; the project-wide suites belong to plan step 8 and Build and Test.

## Framework and setup

- **Runner:** Vitest 3 (`yarn test` = `vitest run`), jsdom environment, `globals: true`, setup
  file `web/src/test/setup.ts` (jest-dom matchers + Testing Library `cleanup` after each test).
  Config `web/vitest.config.ts`, include glob `src/**/*.test.{ts,tsx}` — **no config change**;
  every new test file must match that glob (`*.test.ts` / `*.test.tsx`).
- **Libraries:** @testing-library/react + user-event, jest-axe (all already in `web`
  devDependencies), and `msw@^2.7.0` added by plan step 2.1 (dev-only). Render through the
  existing helper `web/src/test/render.tsx` (`renderWithProviders` — QueryClient with
  `retry: false` by default + I18nProvider + optional MemoryRouter route).
- **Package manager:** Yarn 4.18.1 via corepack only — never npm/npx/pnpm.

## Unit-scoped commands

```
yarn --cwd web test src/features/signup-gates                       # the whole feature suite
yarn --cwd web test src/features/signup-gates/countries.test.ts
yarn --cwd web test src/features/signup-gates/submissionTicket.test.ts
yarn --cwd web test src/features/signup-gates/useGateStatus.test.tsx
yarn --cwd web test src/features/signup-gates/useSubmitGates.test.tsx
yarn --cwd web test src/features/signup-gates/CountrySelect.test.tsx
yarn --cwd web test src/features/signup-gates/GateForm.test.tsx
yarn --cwd web test src/features/signup-gates/notices.test.tsx
yarn --cwd web test src/features/signup-gates/SignupGatesPage.test.tsx
yarn --cwd web test src/features/signup-gates/catalog.test.ts
yarn --cwd web test src/api/gates.test.ts                           # client.ts gate additions
yarn --cwd web test src/app/RequireGatesPassed.test.tsx             # guard + routing table
yarn --cwd web test src/api/client.test.ts                          # regression guard for the step 4.3 refactor only
```

A single test by name: append `-t '<name>'` to any of the above. Watch mode during a cycle:
`yarn --cwd web test:watch <same path>`.

## Mocking guidance

- **Feature-local MSW server.** `web/src/features/signup-gates/test/server.ts` exports a
  `setupServer(...)` bound to `test/handlers.ts` defaults; each consuming test file starts it in
  `beforeAll`, calls `server.resetHandlers()` in `afterEach`, and closes it in `afterAll`. The
  global `src/test/setup.ts` and the existing `src/test/gateway.ts` fetch stub are **not
  modified** — pre-existing suites keep their harness (brownfield floor).
- **Handlers** key on the three contract targets (plan decision 3): `GET /v1/me/principal`,
  `GET /v1/signup/gate-state`, `POST /v1/signup/gates`. Per-test overrides via
  `server.use(http.get(...), http.post(...))`.
- **Deadline and latency tests** use MSW v2 `delay()` (or a never-resolving `HttpResponse` for
  the hung case) plus `vi.useFakeTimers()` where real waiting is impractical; assert both the
  surfaced error state **and** the request count (a deadline is never auto-retried: exactly one
  request; fast 5xx: at most three; G3: always exactly one per manual attempt; 4xx: never
  retried).
- **No mocking of the feature's own modules** except `crypto.randomUUID` (stub to a fixed value
  in ticket tests) and the shell session holder where the sign-out action is asserted. Business
  decisions are never mocked into existence — the server answers come from handlers.

## Test data

- **Principal fixtures** (C1 enums): `{userId, role, status, nextStep}` with
  `status ∈ pending|active|disabled|pending_deletion|deleted`,
  `nextStep ∈ none|verify_email|enrol_mfa|restore_account` — one fixture per spec §2.1 routing
  row, evaluated top-to-bottom first-match-wins.
- **Gate-state fixtures:** `{missing: [], jurisdictionBlocked: false}` (passed),
  `{missing: ["terms","risk","age"] | subsets, jurisdictionBlocked: false}` (gates-required),
  `{missing: [], jurisdictionBlocked: true}` (continued-use blocked); the no-record answer is
  the 404-equivalent problem that `client.ts` maps to the `null` sentinel.
- **G3 outcomes:** 200 (allowed), 403 problem (restricted — non-disclosing; assert
  `restrictedBy` never renders), 422 problem (unasserted — no structured field list; assert the
  G2 re-fetch), 503 problem (geolocation unavailable), 500/network error mid-sequence.
  Problems carry `type/title/status/correlationId` and a `detail` that must **never** appear in
  any assertion-visible surface.
- **Country probes:** `VN` ("Vietnam"/"Việt Nam"), one diacritic-folded search case (e.g.
  searching "viet" finds VN in EN locale), one VI-locale search case.
- **`data-testid` names** (interactive/asserted elements): `gates-loading`, `gates-form`,
  `gates-validation-summary`, `gates-country-input`, `gates-country-listbox`,
  `gates-country-option-<CODE>`, `gates-terms-checkbox`, `gates-risk-checkbox`,
  `gates-age-checkbox`, `gates-continue`, `gates-error-notice`, `gates-error-retry`,
  `gates-blocked-notice`, `gates-signout`, `gates-fullpage-retry`.

## Coverage and quality floor

- No TypeScript coverage threshold is defined for this project (team memory records it as
  unresolved); the 80 % line-coverage floor applies per **Rust** crate and this unit contains no
  Rust. The floor that applies here is per test file: **happy path + at least two error/edge
  cases**, 5–8 tests per component, and no test that passes regardless of implementation.
- The full-suite regression gate (`yarn --cwd web test`, lint, build, `yarn size`) is measured
  by plan step 8 and by Build and Test on the whole suite — never narrow a config, skip a file,
  or raise a budget to make a unit-scoped run pass.
- jest-axe (`web/src/test/axe.ts` helper pattern) runs against **every rendered screen state**:
  ready, validation summary, submitting, blocked (both variants), both inline error variants,
  full-page retry, combobox open and closed (NFR25.4).

## TDD evidence

For each layer (plan steps 3–7): run the new test file **before** the implementation exists and
record the failing command output (Red), then the passing output (Green), then keep it green
across the Refactor. The recorded outputs are the ordering evidence this repository cannot
otherwise show.
