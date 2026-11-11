# Code Summary — u8-signup-gates-ui

**Stage:** Code Generation · **Unit:** `u8-signup-gates-ui` · **Methodology:** TDD (Red → Green → Refactor)
**Testing contract:** `sha256:3f8863a9358b0b5e19a34ad2fc282a634f0d5323fb50ae1f7429a92a216766c3`

## Files created / modified

### Created — feature module `web/src/features/signup-gates/`
| Path | Content |
|---|---|
| `gateStatus.ts` | `GateStatus` / `ServerMissing` / `ClientMissing` unions, `CONSENT_KINDS`, `FIELD_ORDER`, `GATE_DESTINATIONS`, `GateFormValues`, and `deriveGateStatus()` — the spec §2.1 routing table, first-match-wins |
| `submissionTicket.ts` | Memory-only idempotency-key holder (module scope above the screen), keyed by session id + payload signature, `crypto.randomUUID()`; no storage API touched |
| `useGateStatus.ts` | Parallel G1/G2 `useQuery`s under `["signup-gates", <sessionUserId>, "entry", …]`, `staleTime: 0`, retry `count < 2 && isAutoRetryableGateError`, jittered `retryDelay` |
| `useSubmitGates.ts` | G3 mutation with `retry: 0`; outcome union `passed \| blocked \| unasserted \| nothing-saved \| incomplete`; 422 and possibly-partial failures invalidate and re-read G2; 403 never reads G2 |
| `countries.ts` | Generated, frozen 249-entry ISO 3166-1 alpha-2 module (`code`, `en`, `vi`, diacritic-folded `search`), 27,326 B raw / 6,407 B gzip |
| `CountrySelect.tsx` | Memoised WAI-ARIA APG combobox: full keyboard operation, 50-row window, substring filter over the precomputed `search` |
| `AcceptanceCheckbox.tsx` | Labelled checkbox group member; terms/risk carry locale document links (`rel="noopener noreferrer"`), age variant has none |
| `MissingAcceptanceSummary.tsx` | `role="alert"` submit-time summary above the fields, one anchor per missing item |
| `GateForm.tsx` | Form composition, inline validation, `serverMissing` / `recorded` rendering, values-kept rule |
| `BlockedJurisdictionNotice.tsx` | Terminal blocked notice, sign-up and continued-use variants; Sign out only; no retry/resubmit/re-selection control |
| `GateErrorNotice.tsx` | `nothing-saved` vs `incomplete` inline error surfaces with correlation id and a manual retry; never renders server `detail` |
| `GateFullPageRetry.tsx` | Dedicated entry-failure surface (never the shared `ErrorPanel`) |
| `SignupGatesScreen.tsx` | Screen state machine (loading / ready / submitting / blocked / error); composes form and notices, holds no decision logic |
| `SignupGatesPage.tsx` | Lazy route module (name ends `Page`, keeping the chunk in the `route-*.js` glob) |
| `test/server.ts`, `test/handlers.ts` | Feature-local MSW v2 server and contract-target handlers (defaults plus delayed/hung factories) |
| `countries.test.ts`, `submissionTicket.test.ts`, `useGateStatus.test.tsx`, `useSubmitGates.test.tsx`, `CountrySelect.test.tsx`, `GateForm.test.tsx`, `notices.test.tsx`, `SignupGatesPage.test.tsx`, `catalog.test.ts` | TDD suites for every layer |

### Created — generator and guard
| Path | Content |
|---|---|
| `web/scripts/generate-countries.mjs` | Embedded ISO alpha-2 list + Node `Intl.DisplayNames` (en/vi) + NFD diacritic folding; emits the frozen module. No new dependency, no network |
| `web/src/app/RequireGatesPassed.tsx` | Renders children only on `passed`; every unresolved status renders a gate surface; lazy-loads the gate screen so the form/combobox/country data stay out of the entry chunk |
| `web/src/app/RequireGatesPassed.test.tsx` | 8 tests, including remount re-resolution (no cached-passed unlock) |
| `web/src/api/gates.test.ts` | 16 transport tests for the three client additions |

### Modified (additive only)
| Path | Change |
|---|---|
| `web/src/api/client.ts` | New signup-gates section: `callWithDeadline` (15 000 ms, typed `DeadlineAbort`), `getPrincipalReadOnly()` (`GET /me/principal`, sentinel `null` on the defined no-record answer), `getSignupGateState()` (`GET /signup/gate-state`, sentinel `null`), `submitSignupGates(values, idempotencyKey)` (exact four-field body + `Idempotency-Key`), and the allowlist problem→variant mapper (`restricted` / `unasserted` / `unavailable` / `incomplete`) |
| `web/src/api/types.ts` | `Principal` (C1 enums), `SignupGateState`, `SignupGatesPayload` |
| `web/src/app/router.tsx` | Lazy `/gates` route with a route-level `errorElement` rendering `GateFullPageRetry`; product routes wrapped in `RequireGatesPassed` inside `RequireSession`; `/settings/mfa` left outside the guard because it is itself an `owned-elsewhere` destination |
| `web/src/i18n/en.json`, `web/src/i18n/vi.json` | 36 new `gates.*` keys each; key sets equal; document links locale-specific |
| `web/package.json`, `web/yarn.lock` | `msw@^2.7.0` as a dev-only dependency; every importer is `*.test.*` or `test/*` |
| `web/.yarn/install-state.gz`, `web/tsconfig.tsbuildinfo` | Yarn install state and TypeScript build info refreshed by the install/build commands |

## Key implementation decisions

1. **`serverMissing` vs. `recorded` on `GateForm`.** The design names only `serverMissing` while also requiring that an already-recorded acceptance (known only from G2) renders as given/read-only. `serverMissing` is read as the server's outstanding list (merged into the summary, deduplicated), and an optional `recorded` prop carries G2-recorded consents, rendered checked and disabled. This is a deliberate, documented widening of the component contract in `frontend-components.md` §2.
2. **G2 is authoritative after a 422 or a possibly-partial failure.** `GateForm` clears a locally ticked box whenever the server's `serverMissing` signature changes, so a 422 re-renders from G2's `missing` list rather than stale local state, while an unchanged signature keeps the user's input on a 503 (the values-kept requirement).
3. **Lazy gate surface in the guard.** A static import of the screen pulled the screen and country module into the entry chunk; the guard now lazy-loads it behind `Suspense`. Entry chunk dropped from 108.43 kB to 99.19 kB gzip and the country data appears only in the gate chunk, satisfying performance-design §3 / NFR2.1.
4. **Injectable test seams.** `SignupGatesScreen` / `SignupGatesPage` accept an optional `client?: GateClient`, and `RequireGatesPassed` an optional `status?: GateStatus`, for tests only — production passes neither. This lets the deadline test run against a real `GatewayClient({ deadlineMs: 40 })` without mocking any feature module, and exercises both jurisdiction variants.
5. **Session holder is the only mocked module** in the screen/guard suites, exactly as `unit-test-instructions.md` permits; gateway answers always come from MSW through the real client.
6. **Deadline vs. jsdom.** Transport tests use a cross-realm `AbortSignal` acceptance check plus `Promise.race`; no shim was added to the shared test setup, and the global fetch-stub harness stays untouched.
7. **Country data generation** stays dependency-free: the script carries the official alpha-2 list as its only input and resolves names through Node's built-in `Intl.DisplayNames`; the emitted module is committed and ships in the gate route chunk.
8. **Gateway routes remain absent by design** (plan decisions 1–2): against today's gateway the three calls receive the deny-by-default 404 and the feature fails safe to the full-page retry, never into the app. The client targets self-scoped `GET /me/principal`, never the provisioning `principals/{authUserId}` route.

## Review findings addressed (advisory, single pass)

The advisory review returned NOT-READY with 1 critical, 3 major and 1 minor finding. All five were revised in this unit, each with its test added or updated first (Red before Green); no re-review follows under the advisory class.

1. **Critical — no-record entry pre-checked every acceptance.** `gateStatus.ts` carries `noRecord?: boolean` on the `gates-required` branch (set when either entry read answers the no-record sentinel); `SignupGatesScreen` derives `recorded` as empty on it, so the three boxes render unchecked and enabled and a submit sends `terms/risk/age: true`. A `SignupGatesPage` test serves the no-record fixtures, asserts every box is unchecked and enabled, and asserts the exact POST body.
2. **Major — recorded acceptances sent as `false`.** `GateForm.handleSubmit` sends `true` for a recorded kind, matching what the form displays; a `GateForm` test covers a partially recorded visitor.
3. **Major — any unexpected 404 read as "no record".** `client.ts` exports `NO_RECORD_PROBLEM_TYPE` and resolves the null sentinel only for an exact 404 of that type; a bare 404, `about:blank`, and the gateway's `.../problems/not-found` all reject as `ApiError` (fail-safe to the full-page retry). The MSW handler imports the same constant so fixture and client cannot drift.
4. **Major — endless redirect for unregistered U7 destinations.** `RequireGatesPassed.tsx` renders a terminal "screen unavailable" surface (localised title/body plus the sign-out action) for `restore-account` and `account-blocked` instead of navigating; `verify-email` and `enrol-mfa` still navigate. Guard tests cover every account-state destination.
5. **Minor — failed G2 re-read swallowed as `[]`.** `useSubmitGates.readMissing` returns `null` on failure; a failed re-read after a 422 or a possibly-partial failure yields `incomplete` (values kept, manual retry) with no `missing` claim.

## Test coverage summary

- **Unit-scoped suites (re-run at this step):** `yarn --cwd web test src/features/signup-gates src/app/RequireGatesPassed.test.tsx src/api/gates.test.ts` — **11 files / 110 tests, all passing**.
- **Full web suite (run by the build):** **32 files / 252 tests, all passing** — every pre-existing suite stays green.
- **Lint:** `yarn --cwd web lint` clean. **Build:** `yarn --cwd web build` clean.
- **Size budgets** (`yarn --cwd web size`, no config change, measured after the review revisions): initial **99.41 kB gzip** (limit 300), largest route chunk **25.27 kB gzip** (limit 150), CSS **4.27 kB gzip** (limit 150); the gate async chunk measures 9.56 kB gzip.
- **Accessibility:** jest-axe runs against every rendered screen state — ready, validation summary, submitting, blocked (both variants), both inline error variants, full-page retry, combobox open and closed.
- **Per-file floor:** every new test file covers the happy path and at least two error/edge cases; no test passes regardless of implementation.

## Tests written but not executed

None. Every test file written in this stage was executed here; no suite is deferred to CI.

## Deviations from the plan

1. **`yarn --cwd web format:check` is red on 63 pre-existing files, none of them in this unit.** The committed tree already fails under the locked Prettier version (for example untouched `web/src/telemetry.ts`). The unit's own files pass `prettier --check`. The 63 unrelated files were not reformatted because the plan's "Touches" list and the one-unit-per-change rule prohibit it; plan checkboxes 8.1 and 9.3 are left unticked rather than claiming a green gate. The Prettier drift needs its own scoped change (pin/upgrade the formatter or reformat the tree).
2. **Country module size.** The generated module is 27,326 B raw / 6,407 B gzip, below the 10–20 kB gzip estimate in performance-design §4; the mechanical 150 kB route budget passes with the gate chunk at 9.5 kB gzip.
3. **`recorded` prop on `GateForm`** (decision 1) — a documented widening of the component contract.
4. **Test-only injection seams** (decision 4) on the screen, page, and guard — no production caller passes them; the guard's production path remains `useGateStatus`.
5. **Continued-use notice is the only entry-reachable blocked variant.** `deriveGateStatus` produces only `continuedUse: true`; the sign-up variant is exercised through the screen's 403 path and the notice component tests rather than by fabricating an unreachable guard state.
6. **U7 destinations unregistered.** `restore-account` (`/account/restore`) and `account-blocked` (`/account/blocked`) fall to the router catch-all until U7 lands, as plan decision 6 records.