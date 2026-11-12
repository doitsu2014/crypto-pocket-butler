# Tech Stack Decisions — u7-settings-ui

**Upstream:** `functional-spec.md` (§2 shell and routing, §3 screens, §5 forms and validation,
§6 accessibility, §8 API integration points, §10 error surfaces, §11 component inventory),
`frontend-components.md` (§1 file tree, §3 hooks and API client, §6 test plan), the mandated
stack, and codekb `technology-stack.md` (exact versions present in `web/package.json`).
**Unit-specific decisions are numbered `T1`–`T6`; each one names what was rejected and why.**

## 1. Mandated stack, as it applies here

| Concern | Choice | Note |
|---|---|---|
| SPA runtime | React 18.3.1 + TypeScript 5.7, Vite 6 | Mandated; no change |
| Routing | `react-router-dom` ^6.30.1 | Already the shell's router; the settings routes and `/account/restore` / `/account/blocked` are added to the existing route tree (spec §2.1) |
| Server state and cache | `@tanstack/react-query` ^5.62.11 | Already present; every read and mutation in this unit goes through it (spec §8, `frontend-components.md` §3) |
| Styling | Tailwind CSS v4 mapped to Parchment Ledger tokens | Mandated; no raw values in this unit's styles (project mandate) |
| Auth | `@supabase/supabase-js` | Only for reading the in-memory session; every settings call still goes to the gateway |
| Localisation | the existing `web/src/i18n` module | Extended with new keys in English and Vietnamese; no second i18n library |
| Tests | Vitest ^3 + Testing Library + jest-axe + MSW ^2.7 | All already in `web/`; MSW backs the gateway for every unit test |
| Size gate | `size-limit` ^14.3.0 (`yarn size`) | Guards the route-chunk budget of `performance-requirements.md` NFR2.1 |
| Server routes this unit adds | Axum routes on the existing Identity service, plus gateway route-table entries | Rust, `tower`, `sqlx`, `rust_decimal`, `tokio` — mandated; no new crate and no new service (spec §8) |

## 2. Unit decisions

### T1 — No new runtime or dev dependency

**Context.** The unit needs a route guard, four preference panels, a confirmation dialog, and a
test suite that exercises deadlines and error surfaces.
**Decision.** Build on what `web/package.json` already declares: React, `react-router-dom`,
`@tanstack/react-query`, `@supabase/supabase-js`, `tailwindcss`, and — for tests — Vitest,
Testing Library, jest-axe, and MSW.
**Consequences.** The route-chunk and entry-chunk budgets are unaffected by this unit, and no
`yarn.lock` churn is introduced. The cost is that everything must be composed from primitives the
project already uses, which the existing shell and panels already prove is sufficient.
**Alternatives rejected.** A form library (decided against under T4); a date library (T2 — `Intl`
covers the one date this unit formats); a component library (would duplicate the Parchment Ledger
tokens and the existing components).

### T2 — Timezone options come from `Intl.supportedValuesOf("timeZone")`

**Context.** The preferences panel offers a timezone picker. A bundled IANA dataset is the usual
way to get readable city names.
**Decision.** Read the option list at runtime from `Intl.supportedValuesOf("timeZone")`, formatting
each entry through `Intl.DateTimeFormat` in the person's locale.
**Consequences.** No dataset ships and no network call is made, so the bundle budget holds and the
browser's own CLDR data decides the labels — which is also what every other date on the screen
already uses. The list reflects the engine's tzdatabase version, which is the correct behaviour for
a value the service stores verbatim.
**Alternatives rejected.** A bundled `tzdata` list (~30 kB+ gzip of data for a property that ages
by the release); fetching a list from a CDN (forbidden — the SPA talks to the gateway only, and no
third-party origin is contacted at runtime).

### T3 — Extend the existing i18n module; no second localisation path

**Context.** Every string this unit adds exists in English and Vietnamese (NFR26.1), and the
preferences panel displays each field's server-reported `source`.
**Decision.** Add keys to the existing `web/src/i18n` message catalogue and reuse its resolution
order (`?lang=` → stored preference → browser locale → English). The panel reads `source` from the
server's response and renders it as a label; it never re-derives the chain in the browser.
**Consequences.** One resolution path for the whole SPA, and the displayed source always matches
what the shell actually did. Changing the locale in the panel updates the stored preference through
`X2` and the shell re-renders under the new locale.
**Alternatives rejected.** A new i18n library (a second resolution order in one app is a bug
factory); deriving `source` client-side (would duplicate server logic and could disagree with it —
the project forbids frontend business logic).

### T4 — Plain controlled inputs and a local validation helper; no form library

**Context.** Six fields across four panels: display name, locale, base currency, two consent
switches, and the analytics withdrawal toggle.
**Decision.** Use controlled `useState` inputs with one small validation helper per field, mirroring
the server's rules for feedback only; submit through `useMutation`; render field errors with
`aria-describedby` and keep the entered value after a failed save.
**Consequences.** The dependency list stays as it is, the form behaviour is legible in the
component file, and the server remains the only authority on validity (a client check that passes
never implies server success). The cost is a little repetition across four panels, which the small
field count keeps acceptable.
**Alternatives rejected.** A schema-validation library (a dependency for six fields); an
uncontrolled `FormData` approach (loses the per-field `aria-describedby` error wiring the a11y
requirements depend on, NFR25.3).

### T5 — Extend the existing `ConfirmDialog` in place rather than adding a variant

**Context.** The delete flow needs a modal confirmation that traps focus, closes on Escape, and
returns focus to its trigger (NFR25.4), and the analytics-withdrawal confirmation needs the same
behaviour with different copy.
**Decision.** Reuse the shell's existing dialog component, extending it with the focus-trap and
focus-return behaviour if it does not already carry them, and drive both confirmations from it with
different i18n keys. Do not fork a second dialog.
**Consequences.** One focus-trap implementation to get right and to test, and one place where the
a11y behaviour is verified. If the existing component lacks the trap, the fix benefits every other
dialog in the app.
**Alternatives rejected.** A new `DeleteConfirmDialog` (two focus traps, one of which would be
untested); the native `window.confirm` (no focus management, no styling, not testable for
`aria-live` behaviour).

### T6 — Cache keys, invalidation, and the "no record" answer are named explicitly

**Context.** Four panels each read their own resource, and the entry routing read (D0) must stay
fresh across restores and deletions (`performance-requirements.md` NFR2.9).
**Decision.** Keys, exactly as `frontend-components.md` §3 names them:
`["me","account-status"]` (D0), `["me","profile"]` (P1), `["me","preferences"]` (X2),
`["me","consent"]` (X3/X4), `["me","deletion"]` (D1).
Invalidations: P2 → `["me","profile"]`; X2 → `["me","preferences"]`; X5/X6 → `["me","consent"]`
and `["me","consent","history"]`; D3 and D2 → `["me","deletion"]` and `["me","account-status"]`.
D0 sets `staleTime: 0`, overriding `createQueryClient`'s app-wide `staleTime: 30_000`
(`web/src/api/queryClient.ts`); every other settings query keeps the app default. No route changes
`retry`: queries keep the app's single `5xx`-only retry (`shouldRetry`) and mutations keep the
app's `retry: false`.
D1's "no open request" `404` resolves to `null` through the existing per-request
`nullOnNoRecord` option, which accepts only the pinned `NO_RECORD_PROBLEM_TYPE`
(`https://crypto-pocket-butler.example/problems/no-record`, `web/src/api/client.ts`). **That makes
the sentinel part of what D1 must serve**, so the mapping depends on the contract addition recorded
in §3 below; without it a `404` surfaces as an error, not as the empty state.
**Consequences.** Every screen state is derived from a named key, so a mutation's effect is
provable in a test that asserts the refetch rather than by inspection. The one non-obvious mapping
— `404` as a value, not a failure — is stated here so it is not "fixed" later into an error state.
**Alternatives rejected.** A single `["me"]` root key invalidated wholesale (refetches four panels
for one save and hides which read a screen depends on); treating every D1 `404` as null regardless
of its problem `type` (the app pins the sentinel precisely so an undeclared route can never
masquerade as a normal answer, and the same rule applies here).

## 3. Open dependencies outside this unit

- **D1's no-record problem type must be pinned in the contract.** The client treats `404` as "no
  open request" only when the problem's `type` is
  `https://crypto-pocket-butler.example/problems/no-record`; Identity's `not_found` helper renders
  `https://errors.cpb/identity/not-found` for every `404` (`services/identity/http/src/problem.rs`).
  Before D1 is built, Contract Design must add that pinned `type` to the D1 response — the same
  addition u8 records for its gate reads — or the empty state will render as an error.
- **Gateway entries for `POST /v1/exports` and `GET /v1/exports/{exportId}`** do not exist yet
  (the gateway route table declares no export path; spec §12 open item 6). Until they land, the
  export panel cannot reach the service through the gateway; this unit's screens are specified
  against the contract, and the panel is not built around a route the gateway does not serve.
- **`/v1/me/principal` has no served route** (spec §12 open item 5). This unit therefore routes by
  D0 (`/v1/me/account-status`), not by the principal read.
- **`contract-summary.md`** C3 upstream list, C6 and C7 need the D0/P1/P2/D1/D2/D3 and gateway
  export rows (spec §12 open item 1). That is a Contract Design artefact, owned outside this unit;
  the functional spec records the delta so the contract can be reconciled.
- **`retainedCategories`** is not populated by the contract yet (spec §12 open item 2). The delete
  page renders a fixed placeholder sentence until it is; no category is invented.