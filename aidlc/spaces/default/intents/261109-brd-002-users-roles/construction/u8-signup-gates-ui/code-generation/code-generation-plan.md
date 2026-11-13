# Code Generation Plan — u8-signup-gates-ui

**Stage:** Code Generation · **Unit:** `u8-signup-gates-ui` (ui, complexity S) · **Methodology:** TDD (Red → Green → Refactor, tests land with the code they cover)
**Design sources:** `functional-spec.md` (revision 2 — §2.1 routing table, §2.2–§2.6 screens and outcomes, §6 G1/G2/G3), `frontend-components.md` (revision 2 — component contracts, `GateStatus` union), `nfr-design/logical-components.md` (placement and isolation), `nfr-design/performance-design.md` (deadlines, retry policy, chunking, caching), `nfr-design/security-design.md` (key holder, allowlist mapping, self-scoped routes, §11 dependencies), `infrastructure-design/*` (no infrastructure change; shared-image pipeline), `contract-summary.md` (C1 Principal schema, C3, C6, C8), `unit-of-work.md` (U8), `requirements.md` (FR19.2–FR19.5, NFR25, NFR26).

## Summary

- Builds: the sign-up gates feature inside the existing SPA — a lazy `/gates` route with a searchable country combobox (full ISO 3166-1, EN+VI), three acceptance checkboxes, a submit-time missing-acceptance summary, the terminal blocked-jurisdiction notice (sign-up and continued-use variants), inline retryable error surfaces with correlation ids, a `RequireGatesPassed` guard wrapping every post-sign-in product route, a route-level error boundary, the three additive API-client methods with a 15 s deadline and typed no-record sentinels, two query/mutation hooks, a memory-only idempotency-key holder, and a build-time-generated country data module.
- Touches: `web/src/features/signup-gates/` (new feature folder — components, hooks, holder, data module, MSW test harness, tests), `web/src/api/client.ts` and `web/src/api/types.ts` (additive only), `web/src/app/router.tsx` plus a new `web/src/app/RequireGatesPassed.tsx` (guard + route-level `errorElement`), `web/src/i18n/en.json` and `vi.json` (additive `gates.*` namespace), `web/scripts/generate-countries.mjs` (new generator), `web/package.json` + `yarn.lock` (one devDependency: `msw@^2.7.0`). No Rust, no schema, no `infra/swarm-stack.yml`, no nginx or CSP change.
- Tests: Vitest 3 + Testing Library + jest-axe against a feature-local MSW v2 gateway mock — 9 new test files under `web/src/features/signup-gates/` and `web/src/app/`, plus one additive API test file `web/src/api/gates.test.ts`; every file covers the happy path and at least two error/edge cases (5–8 tests per component), and every rendered screen state passes jest-axe.

## Recorded decisions, deviations, and dependencies

Read this section before building; each item is a settled decision, not an open question.

1. **Server routes do not exist yet — by design.** None of G1-read-only, G2, or G3 is declared in the gateway's deny-by-default table (`services/gateway/domain/src/route.rs`); all five security-design §11 dependencies are owned by the gateway/U1/U4/U6 units and none has landed. The client is built against the designed contract below. Against today's gateway every gate call receives the deny-by-default 404 problem, which the feature maps to its error surfaces — entry resolution fails safe to the full-page retry and never into the app (infrastructure-specification.md). Nothing in this unit touches gateway or service code.
2. **G1 shape deviation (security-design §11 dep 1, recorded there and in NFR22.3).** functional-spec §6 names G1 as `GET /v1/principals/{authUserId}` ("resolve or provision"); that route provisions on read and puts an auth user id in the logged URL. Per security-design §5 the feature targets **self-scoped read-only** routes only and **never falls back to a subject-in-path call**. This plan pins the client target as `GET /v1/me/principal` (read-only, non-provisioning, defined no-record answer). The route entry itself is gateway/U1 work; until it exists the call 404s (fail-safe, item 1). The existing unauthenticated `POST /v1/signup-gate` (gateToken) and `createSignupGate()` in client.ts serve the pre-signup flow and are **not** touched, reused, or extended.
3. **Contract targets** (single source: this table; all paths are `/v1`-relative inside `client.ts`):
   - **G1** `GET /me/principal` → C1 `Principal` (`userId`, `role`, `status: pending|active|disabled|pending_deletion|deleted`, `nextStep: none|verify_email|enrol_mfa|restore_account`) or the defined no-record answer, mapped in `client.ts` to the sentinel `null` (a *resolved* value — never a rejection).
   - **G2** `GET /signup/gate-state` → `{ missing: ("terms"|"risk"|"age")[], jurisdictionBlocked: boolean }` or no-record → sentinel `null`.
   - **G3** `POST /signup/gates`, body exactly `{ countryCode, terms, risk, age }`, header `Idempotency-Key`; outcomes `200` allowed · `403` restricted · `422` acceptance not asserted (→ re-fetch G2) · `503` geolocation unavailable · 5xx/network possibly-partial.
4. **`msw@^2.7.0` becomes a `web` devDependency** (same major as support-console; frontend-components.md §6 records it for the organisation's dependency review). Dev-only: imported exclusively from feature test files, never from `src/` application code, so it cannot reach the production bundle (security-design §3). Install with `yarn` only.
5. **Country module generation (functional-spec §6 open item 5).** `web/scripts/generate-countries.mjs` emits `web/src/features/signup-gates/countries.ts` — a frozen `{ code, en, vi, search }[]` for the full ISO 3166-1 alpha-2 list. The script carries the official alpha-2 code list as its only input and resolves names through Node's built-in `Intl.DisplayNames` (`en` and `vi`), so there is **no new dependency and no network fetch**; `search` is the lowercased, diacritic-folded concatenation of both names, precomputed (performance-design §4). Regeneration is `node scripts/generate-countries.mjs` from `web/`, committed like any source change; the generated file is checked in (it ships in the route chunk).
6. **`owned-elsewhere` destinations.** `verify-email` → `/verify-email`, `enrol-mfa` → `/settings/mfa` (both routes exist). `restore-account` → `/account/restore` and `account-blocked` → `/account/blocked` are U7 screens not yet in the router; the guard maps to those path constants, and until U7 registers them the router's catch-all sends the visitor to the sign-in screen — a fail-safe landing recorded here, not a screen this unit invents.
7. **No shared-surface reuse for errors.** The gate never renders the shared `ErrorPanel` (it renders `detail`); all gate error surfaces are dedicated feature components receiving an already-mapped localised variant + correlation id (security-design §5). `ErrorPanel` itself is unchanged.
8. **Quality targets are not negotiable.** No threshold, budget, lint rule, or test may be relaxed, skipped, or narrowed to make a step pass. The size budgets (300 kB initial / 150 kB route chunk / 150 kB css, gzip) are enforced by the existing `yarn size` with **no config change**; the `SignupGatesPage` module name (ending `Page`) keeps the chunk inside the `route-*.js` glob via `web/vite.config.ts`'s existing `chunkFileNames`.
9. **Brownfield discipline.** All existing files are modified in place — no `_modified`/`_new` duplicate files, no copied-and-renamed modules. `client.ts`, `types.ts`, `router.tsx`, and the two catalogues receive additive changes only; existing callers and tests stay untouched and green.

## Steps

Layers from the testing posture that do not apply to this ui unit are named once, here: **Data model / database behavior** and **Repository / data access** — no schema, no migration, no service code; **API / endpoint** — the server side of G1/G2/G3 is gateway/U1/U4/U6 work (item 1). The applicable layers are static data, transport, feature logic, frontend behavior, and localisation, each run Red → Green → Refactor with the failing output recorded before implementing.

### Step 1 — Project structure and production configuration skeleton

- [x] 1.1 Create `web/src/features/signup-gates/` with the file layout below (empty exports are fine at this point; every file gains content in its layer's step):
  - `countries.ts` (generated, step 3) · `generate` output target
  - `submissionTicket.ts` · `useGateStatus.ts` · `useSubmitGates.ts` · `gateStatus.ts` (the `GateStatus`/`ServerMissing`/`ClientMissing` types from frontend-components.md §2)
  - `SignupGatesPage.tsx` (the lazy route module — name ends `Page`, decision 8) · `SignupGatesScreen.tsx` · `GateForm.tsx` · `CountrySelect.tsx` · `AcceptanceCheckbox.tsx` · `MissingAcceptanceSummary.tsx` · `BlockedJurisdictionNotice.tsx` · `GateErrorNotice.tsx` · `GateFullPageRetry.tsx`
  - `test/server.ts` + `test/handlers.ts` (feature-local MSW harness, step 2)
- [x] 1.2 Add the `gates.*` i18n key namespace scaffolding decision: keys are authored in step 6/7 against `web/src/i18n/en.json` and `vi.json` (additive; `resolveLocale` and `createTranslator` untouched).
- [x] 1.3 Add the API DTO types to `web/src/api/types.ts` (additive): `Principal` (C1 enums, decision 3), `SignupGateState` (`missing`, `jurisdictionBlocked`), `SignupGatesPayload` (`countryCode`, `terms`, `risk`, `age`). Transcribe from contract-summary C1 — no invented fields.

### Step 2 — Test runner readiness (before the first Red)

- [x] 2.1 Add `msw@^2.7.0` to `web/package.json` devDependencies and install with `yarn` (decision 4; Yarn 4.18.1 via corepack — never npm/npx/pnpm). `yarn.lock` updates in the same change.
- [x] 2.2 Verify the existing runner: `web/vitest.config.ts` (jsdom, `globals: true`, setup `./src/test/setup.ts`, include `src/**/*.test.{ts,tsx}`) needs **no change**; confirm with `yarn --cwd web test src/features/auth/SignInPage.test.tsx` (an existing suite) that the runner is green before any new test is written.
- [x] 2.3 Create the feature-local MSW harness: `web/src/features/signup-gates/test/server.ts` (setupServer, started/stopped in the consuming test files' `beforeAll`/`afterEach`/`afterAll` — the global `src/test/setup.ts` stays untouched so existing fetch-stub tests are unaffected) and `test/handlers.ts` with default handlers for `GET /v1/me/principal`, `GET /v1/signup/gate-state`, `POST /v1/signup/gates` (fixtures per unit-test-instructions.md; includes delayed/hung handler factories for deadline tests).
- [x] 2.4 Record the exact unit-scoped commands in `unit-test-instructions.md` (shipped with this plan) and use only those for the Red/Green cycles; the project-wide suite runs at step 8 and in Build and Test.

### Step 3 — Static data layer: generated country module

- [x] 3.1 **Red:** write `web/src/features/signup-gates/countries.test.ts` (5–8 tests): the module exports a frozen non-empty array (~250 entries); every entry has `code` (uppercase alpha-2, unique), non-empty `en` and `vi` names, and a `search` string that is lowercase, diacritic-free, and contains both names folded; a known probe country (e.g. `VN` → "Vietnam"/"Việt Nam") is present; the array is `Object.isFrozen`. Run `yarn --cwd web test src/features/signup-gates/countries.test.ts` and record the failing output (module missing).
- [x] 3.2 Write `web/scripts/generate-countries.mjs` (decision 5): embedded ISO 3166-1 alpha-2 list; names via `Intl.DisplayNames(["en"], { type: "region" })` and `["vi"]`; diacritic folding via `String.prototype.normalize("NFD")` + combining-mark strip + lowercase; emits `src/features/signup-gates/countries.ts` as `Object.freeze`d typed const with a "generated — do not edit" header.
- [x] 3.3 **Green:** run `node scripts/generate-countries.mjs` from `web/`, then the test; record passing output. Verify the emitted file's raw size is in the 10–20 kB gzip estimate range (performance-design §4; the mechanical check is `yarn size` at step 8).
- [x] 3.4 **Refactor:** tidy the generator (formatting, comments citing FR19.3/Q2 and open item 5) while the test stays green.

### Step 4 — Transport layer: `client.ts` gate additions

All additions live in `web/src/api/client.ts` (new `// ── Signup gates (U8) ──` section) and are used only through the single API-client module. No existing method or caller changes.

- [x] 4.1 **Red:** write `web/src/api/gates.test.ts` (MSW-backed; 8+ tests across the four additions, happy path + ≥2 error/edge each):
  - `getPrincipalReadOnly()`: resolves `Principal` on 200; resolves the sentinel `null` on the defined no-record answer (404-equivalent problem) — assert it *resolves*, never rejects; rejects `ApiError` on 500.
  - `getSignupGateState()`: resolves `{missing, jurisdictionBlocked}` on 200; sentinel `null` on no-record; rejects on 5xx.
  - `submitSignupGates(values, idempotencyKey)`: sends exactly `{countryCode, terms, risk, age}` and the `Idempotency-Key` header (assert both on the MSW request); never adds an IP or any extra field; surfaces 403/422/503/5xx as typed `ApiError` carrying `status` + `correlationId` and **no `detail` passthrough** into the mapped result.
  - `callWithDeadline`: a hung handler (>15 s — use fake timers or a short-injected deadline for testability without real waiting) aborts with the deadline error type; the abort is never auto-retried (request-count assertion: exactly 1 request).
  - Allowlist mapping: the exported gate-error classifier maps status/type → variant (`restricted` | `unasserted` | `unavailable` | `incomplete`) and drops `detail` (security-design §5).
  - Run `yarn --cwd web test src/api/gates.test.ts`; record failing output.
- [x] 4.2 **Green:** implement `callWithDeadline` (performance-design §5 sketch: `AbortController`, 15 000 ms, typed `DeadlineAbort`, `clearTimeout` in `finally`), `getPrincipalReadOnly()`, `getSignupGateState()`, `submitSignupGates(values, idempotencyKey)`, and the allowlist problem→variant mapper. Sentinel mapping (no-record → `null`) happens **inside** the client methods so queries resolve (performance-design §2). All three calls go through `callWithDeadline`. Record passing output.
- [x] 4.3 **Refactor:** share the problem-parsing path with the existing `request<T>` plumbing without changing existing behaviour (`yarn --cwd web test src/api/client.test.ts` must stay green — run it as the refactor's guard).

### Step 5 — Feature logic layer: key holder and hooks

- [x] 5.1 **Red:** write `web/src/features/signup-gates/submissionTicket.test.ts` (5–8 tests): first submit intent mints a `crypto.randomUUID()` key stamped with the current session id; the identical payload reuses the key; **any field edit mints a new key**; success discards it; a different session id never sees the previous key (unbound holder); nothing is written to any storage API (assert `localStorage`/`sessionStorage` untouched). Record failing output.
- [x] 5.2 **Green:** implement `submissionTicket.ts` — module-scope holder above the screen component (survives remounts), memory-only (security-design §4). Record passing output. **Refactor** while green.
- [x] 5.3 **Red:** write `web/src/features/signup-gates/useGateStatus.test.tsx` (8+ tests) covering the full spec §2.1 decision table, top-to-bottom, first-match-wins: G1 failure (after bounded retries) → `error`; no-record sentinels → `gates-required` with empty `serverMissing`; `nextStep` verify_email/enrol_mfa/restore_account → `owned-elsewhere` with the right destination; `disabled`, `pending_deletion`, `deleted` → `owned-elsewhere(account-blocked)`; `pending` or `active`+non-empty `missing` → `gates-required(serverMissing)`; `jurisdictionBlocked: true` → `jurisdiction-blocked(continuedUse: true)`; `active`+`none`+`[]`+not blocked → `passed`. Also: G1 and G2 issue **in parallel** (assert both requests in flight before either settles); `staleTime: 0` (re-entry refetches — no cached `passed` unlock); a deadline on either read yields `error` with **one** attempt (no auto-retry, request-count assertion); fast 5xx retries at most 3 attempts with jittered delay; 404/4xx never retried. Record failing output.
- [x] 5.4 **Green:** implement `gateStatus.ts` types (transcribe frontend-components §2 union exactly) and `useGateStatus.ts`: query key `["signup-gates", <sessionUserId>, "entry"]` (performance-design §8 namespacing), parallel queries, sentinel-aware combine, retry predicate `count < 2 && isAutoRetryable` + custom jittered `retryDelay` (performance-design §5 formula), no `refetchInterval`. Record passing output. **Refactor** while green.
- [x] 5.5 **Red:** write `web/src/features/signup-gates/useSubmitGates.test.tsx` (8+ tests): 200 → invalidates the gate-state key and resolves `passed` on re-resolve; 403 → `blocked` outcome; 422 → invalidates + re-fetches G2 and the hook surfaces the server `missing` list (never parses the problem body for fields); 503 → `nothing-saved` variant; 5xx/network/deadline → `incomplete` variant; `retry: 0` (exactly one request per attempt, request-count assertion); Continue-driven `isPending` flips synchronously in the same act() as activation; retries reuse the ticket key, edits mint a new one (integration with 5.1's holder). Record failing output.
- [x] 5.6 **Green:** implement `useSubmitGates.ts` (mutation, `retry: 0`, outcome mapping per frontend-components §3, invalidation on success/422/failure-driven G2 refetch). Record passing output. **Refactor** while green.

### Step 6 — Frontend behavior layer: components, screen, guard, router

Interactive elements carry `data-testid` attributes (list in unit-test-instructions.md §Test data). Parchment Ledger tokens only — no raw colour/spacing/type values; error states pair colour with icon + text; every string via `t("gates.*")`.

- [x] 6.1 **Red:** write `web/src/features/signup-gates/CountrySelect.test.tsx` (5–8 tests): type-ahead search filters by EN and VI names (diacritic-insensitive); selection stores the alpha-2 code and calls `onChange` once; full keyboard operation per WAI-ARIA APG (ArrowUp/Down, Home/End, Enter, Escape; `role="combobox"`/`listbox`/`option`, `aria-expanded`, `aria-activedescendant`); renders in VI locale; jest-axe clean open and closed. Record failing output.
- [x] 6.2 **Green:** implement `CountrySelect.tsx` — `React.memo`-wrapped, owns its query-string state, `useMemo`'d substring filter over precomputed `search` (performance-design §4/§7: keystrokes re-render only the listbox). Record passing output. **Refactor** while green.
- [x] 6.3 **Red:** write `web/src/features/signup-gates/GateForm.test.tsx` (8+ tests): submit with all four fields calls `onSubmit` with exactly `{countryCode, terms, risk, age}`; submit with missing items renders `MissingAcceptanceSummary` above the fields in DOM order listing every missing item by name (including `country`), moves focus to the summary, `role="alert"`, each entry anchors to its field id, and makes **no** G3 call; per-field inline errors appear beside each field; Continue is never disabled by validation (spec §2.3); `serverMissing` items render as already given/read-only (a recorded acceptance arrives only via G2 — never pre-checked from local state); checkboxes are unchecked on first render; tab order = summary links → country → terms → risk → age → continue; jest-axe clean empty, with summary, and with server-missing. Record failing output.
- [x] 6.4 **Green:** implement `GateForm.tsx`, `AcceptanceCheckbox.tsx` (labelled group via `fieldset`/`group` role; terms/risk carry document links opening in a new tab with `rel="noopener noreferrer"` — links, never fetches; age variant has no link), `MissingAcceptanceSummary.tsx`. Record passing output. **Refactor** while green.
- [x] 6.5 **Red:** write `web/src/features/signup-gates/notices.test.tsx` (6+ tests): `BlockedJurisdictionNotice` sign-up variant states service-unavailable + **no personal data stored** + support contact, offers Sign out **only** (no retry/resubmit/re-selection control exists in the DOM); continued-use variant swaps in the account-temporarily-unavailable line; `restrictedBy` never appears in either; `GateErrorNotice` `nothing-saved` vs `incomplete` wording per spec §2.6, both show the correlation id, both keep a manual retry control, neither renders server `detail`; blocked notice announced as page-level status; jest-axe clean for all four renders. Record failing output.
- [x] 6.6 **Green:** implement `BlockedJurisdictionNotice.tsx` (onSignOut wires the shell's existing `session.signOut()` path — no new token handling) and `GateErrorNotice.tsx`. Record passing output. **Refactor** while green.
- [x] 6.7 **Red:** write `web/src/features/signup-gates/SignupGatesPage.test.tsx` (8+ tests): full submit-allowed journey (form → Continue → `submitting` with Continue disabled and progress announced in the same act() → app content reachable); 403 → screen replaced by blocked notice; 422 → G2 re-fetched and the screen re-renders from its `missing` list; 503 → `nothing-saved` inline error, **field values kept**; mid-sequence 5xx → `incomplete` error, retry (same key) re-fetches G2 showing committed consents as given; deadline/hung G3 → `incomplete` within the deadline, values and key kept; entry error → `GateFullPageRetry` (dedicated component, never `ErrorPanel`); jest-axe on every rendered state (`loading`, `ready`, summary, `submitting`, `blocked` ×2, `error` ×2, full-page retry). Record failing output.
- [x] 6.8 **Green:** implement `SignupGatesPage.tsx` (lazy route module), `SignupGatesScreen.tsx` (state machine `loading/ready/submitting/blocked/error`, composes form + notices, owns no decision logic), `GateFullPageRetry.tsx`. Record passing output. **Refactor** while green.
- [x] 6.9 **Red:** write `web/src/app/RequireGatesPassed.test.tsx` (8+ tests): renders children only on `passed`; `gates-required` renders the gate screen **for any deep-linked product route** (first-source step unreachable — AC6.2.2/AC6.5.2); `jurisdiction-blocked` renders the notice (both variants); each `owned-elsewhere` destination routes to its path (decision 6 constants); `error` renders the full-page retry — **no branch enters the app on an unresolved status**; `loading` renders the skeleton; the guard re-renders from a fresh resolution on remount (no cached-passed unlock, NFR24.1). Record failing output.
- [x] 6.10 **Green:** implement `web/src/app/RequireGatesPassed.tsx` (reads `useGateStatus`, no navigation side effects beyond the destination mapping) and wire `web/src/app/router.tsx`: lazy `SignupGatesPage` at `/gates`; wrap every post-sign-in product route in `RequireGatesPassed` (inside the existing `RequireSession`); add the **route-level `errorElement`** on the gate route rendering the full-page retry inside the route (performance-design §3 — the router has no route-level boundary today; existing routes and the root `ErrorBoundary` unchanged). Record passing output. **Refactor** while green.

### Step 7 — Localisation layer: `gates.*` catalogues

- [x] 7.1 **Red:** write `web/src/features/signup-gates/catalog.test.ts` (5+ tests): every `gates.*` key referenced by the feature (labels, four validation messages, summary sentence, blocked ×2 variants, both error variants, retry, sign-out, support contact, loading/progress announcements) exists in **both** `en.json` and `vi.json` with non-empty, non-identical-to-the-key values; EN and VI key sets are equal (no missing VI key falls back silently); document links resolve per locale. Record failing output.
- [x] 7.2 **Green:** author the `gates.*` keys in `web/src/i18n/en.json` and `vi.json` (additive; spec §2.2 labels: "I accept the Terms of Service.", "I have read and accept the risk disclosure.", "I confirm I meet the minimum age for this service."; spec §2.6 wording: nothing-saved vs couldn't-complete). Record passing output. **Refactor** while green. (Country names are already localised via the step-3 module — the catalogues do not duplicate them.)

### Step 8 — Environment/build configuration and quality gates

- [ ] 8.1 `yarn --cwd web format:check` · `yarn --cwd web lint` (`@typescript-eslint/no-explicit-any` is an error — no `any` anywhere in the new code) · `yarn --cwd web build` (`tsc -b && vite build`).
- [x] 8.2 `yarn --cwd web size` — the mechanical NFR2.1 gate with **no config change**: gate route chunk (page + country module) ≤ 150 kB gzip, initial entry ≤ 300 kB, css ≤ 150 kB. If a budget fails, reduce the code — never raise a budget.
- [x] 8.3 `yarn --cwd web test` — the **full** web suite: every pre-existing test file stays green (brownfield regression floor), all new files pass.
- [x] 8.4 Confirm the production bundle carries no test-only import: `msw` appears in no `src/` non-test module (`web/src/features/signup-gates/test/*` and `*.test.*` are its only importers).

### Step 9 — Documentation and traceability

- [x] 9.1 Module-level comments cite their requirement ids (`//!`-style JSDoc headers: FR19.x, NFR ids, spec sections) per the repository's code-style rule.
- [x] 9.2 No C4/`docs/architecture` change: no container, schema, external system, or relationship changes (infrastructure-specification.md). The gateway route entries this feature waits on are the gateway/U1 units' documentation burden, recorded in this plan's decision 1–2, not added to the C4 model here.
- [ ] 9.3 Tick every checkbox above as completed; the stage's step-5 records (code summary, source manifest, traceability) are produced from the finished tree.

## Story-to-step traceability

| Story AC (functional-spec §7) | Satisfied by steps |
|---|---|
| AC6.2.1 — missing acceptance named, connection refused | 6.3/6.4 (client naming + summary), 5.5/5.6 (422 → G2 re-fetch → server-named `missing`), 4.1/4.2 (typed 422 mapping) |
| AC6.2.2 — first-source step unreachable without recorded age confirmation | 6.9/6.10 (`RequireGatesPassed` wraps every product route; deep links render the gate screen), 5.3/5.4 (no cached-passed unlock) |
| AC6.3.1 — restricted jurisdiction denied before any personal data stored | 4.1/4.2 (exact four-field payload, no IP field), 5.5/5.6 (403 → blocked), 6.5/6.6 ("no personal data stored" notice, terminal), 6.1/6.2 (full ISO list client-side — no restricted copy, server refuses) |
| AC6.3.2 — continued-use blocking | 5.3/5.4 (`jurisdictionBlocked` → `jurisdiction-blocked(continuedUse)`), 6.5/6.6 (continued-use variant), 6.9/6.10 (entry routing renders it instead of the app) |
| AC6.5.1 — age confirmation recorded, self-declaration only | 6.3/6.4 (single checkbox, no date-of-birth or upload field exists), 5.5/5.6 (recorded via G3→C6 server path) |
| AC6.5.2 — first-source step unavailable without confirmation | same mechanism as AC6.2.2: 6.9/6.10 |
| NFR25 (WCAG 2.2 AA) | 6.1–6.10 (ARIA APG combobox, alert summary + focus, tab order, colour+icon+text, jest-axe every state) |
| NFR26 (EN+VI) | 3.1–3.3 (country names both locales), 7.1–7.2 (catalogue completeness), 6.1 (VI render test) |

## Definition of done

Every checkbox ticked; every Red output recorded before its Green; all commands of step 8 pass locally (the same commands CI would run — team rule); `unit-test-instructions.md` commands all green; no existing test, lint rule, budget, or threshold altered; the diff stays inside the file list of the Summary's "Touches" line.

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
