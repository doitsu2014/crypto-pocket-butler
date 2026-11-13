# Frontend Components — U8 Sign-up Gates UI (`u8-signup-gates-ui`)

**Stack:** React 18 + TypeScript 5, Vite, TanStack Query, Tailwind CSS v4 mapped to Parchment
Ledger tokens (no raw colour/spacing/type values), `@supabase/supabase-js` for auth (existing).
**Location:** a feature module in `web/` (e.g. `web/src/features/signup-gates/`); all gateway
calls live in the single API-client module `web/src/api/client.ts` — components never call the
gateway directly. No business math, no compliance decisions in the frontend.
**Upstream:** `functional-spec.md` (this unit, revision 2) — states, transitions, and endpoints G1/G2/G3.
**Revision:** 2 — revised after the advisory review; see `functional-spec.md` header.

## 1. Component hierarchy

```
App router (existing)
└── RequireGatesPassed            route guard for every post-sign-in product route
    └── SignupGatesScreen         the dedicated gate screen (route: /gates)
        ├── GateForm
        │   ├── MissingAcceptanceSummary  submit-time validation summary (above the fields in DOM order)
        │   ├── CountrySelect     full ISO 3166-1 combobox
        │   ├── AcceptanceCheckbox × 3   terms / risk / age
        │   └── SubmitButton ("Continue")
        ├── GateErrorNotice       retryable error (503/network/5xx), inline
        └── BlockedJurisdictionNotice     terminal session state
```

## 2. Components, props, and state

Interface-level contracts (implementation lands in Code Generation, TDD):

```ts
// Gate state as resolved from G1+G2; drives RequireGatesPassed routing
// (spec §2.1 — every Principal status and nextStep has a destination).
type GateStatus =
  | { kind: "loading" }
  | { kind: "passed" }
  | { kind: "gates-required"; serverMissing: ServerMissing[] }
  | { kind: "jurisdiction-blocked"; continuedUse: boolean }
  | { kind: "owned-elsewhere"; destination: "verify-email" | "enrol-mfa" | "restore-account" | "account-blocked" }
  | { kind: "error"; correlationId?: string };

// The only items the server can name (G2 `missing`): the three consent kinds.
type ServerMissing = "terms" | "risk" | "age";
// Client-side validation additionally names the country (spec §2.3):
type ClientMissing = ServerMissing | "country";
```

| Component | Props | Internal state / notes |
|---|---|---|
| `RequireGatesPassed` | `children` | Reads the `useGateStatus()` query; renders children only on `passed`; `gates-required` → `<SignupGatesScreen>`; `jurisdiction-blocked` → `<BlockedJurisdictionNotice>`; `owned-elsewhere` → the named screen's route (shell/U7, spec §2.1 table); `error` → full-page retry; `loading` → page skeleton. No navigation side effects. |
| `SignupGatesScreen` | — | Owns the submit lifecycle: `ready → submitting → {passed \| blocked \| error}`; holds the per-visit idempotency key (in memory). Renders GateForm plus GateErrorNotice/BlockedJurisdictionNotice per state. |
| `GateForm` | `onSubmit(values: GateFormValues)`, `isSubmitting`, `serverMissing: ServerMissing[]` | Controlled values `{ countryCode, terms, risk, age }`; validation runs on submit only (spec §2.3); the summary lists client-missing items (including `country`) merged with `serverMissing` (server-named items win). |
| `CountrySelect` | `value`, `onChange(code)`, `error?: string`, `labelId` | Searchable single-select combobox over the full ISO 3166-1 list; stores alpha-2 code; WAI-ARIA combobox pattern. Data: a static build-time-generated module in the repo (EN names from the ISO 3166-1 table, VI names from Unicode CLDR), bundled with the gate **route chunk** — no runtime fetch, inside the 150 kB route budget (spec §6, open item 5). |
| `AcceptanceCheckbox` | `acceptance: ServerMissing`, `checked`, `onChange`, `documentHref?`, `error?: string` | Never pre-checked from local state; an already-recorded acceptance arrives only via G2 and renders as given/read-only. Label + linked document; `aria-describedby` to its error. Age variant has no document link, label per spec §2.2. |
| `MissingAcceptanceSummary` | `missing: ClientMissing[]` | Rendered above the fields in DOM order; `role="alert"`, receives focus on appearance; each entry is an anchor to its field's id; rendered only when non-empty. |
| `BlockedJurisdictionNotice` | `continuedUse: boolean`, `onSignOut()` | Terminal: notice text — "no personal data stored" for sign-up (`continuedUse: false`), account-temporarily-unavailable line when `continuedUse` (spec §2.5) — support contact link, Sign out button only. No retry, no form. |
| `GateErrorNotice` | `variant: "nothing-saved" \| "incomplete"`, `correlationId?: string`, `onRetry()` | Inline retryable error; wording per spec §2.6 (`nothing-saved` only where nothing can have committed); retry is manual. |

## 3. Data flow and hooks

- `useGateStatus()` — TanStack Query over G1 (read-only principal lookup) + G2
  (`GET /v1/signup/gate-state`), combined into `GateStatus` per the spec §2.1 decision table.
  Cached under one query key; the sign-up mutation and every retryable failure invalidate it.
- `useSubmitGates()` — mutation over G3 (`POST /v1/signup/gates`) with the screen's
  `Idempotency-Key` header (reused across retries of the same submission):
  - `200` → invalidate gate state → router renders the app (guard now `passed`).
  - `403` → screen state `blocked` (BlockedJurisdictionNotice; nothing stored).
  - `422` → invalidate and **re-fetch G2**; the screen re-renders from its `missing` list
    (spec §2.3 — G2 is the single source of server-named items; the ProblemDetails body is
    never parsed for field names and its `detail` is never rendered).
  - `503` → screen state `error`, variant `nothing-saved`; field values kept.
  - network/5xx → screen state `error`, variant `incomplete`; on retry the G2 re-fetch shows
    whatever committed (spec §2.6).
- `client.ts` additions: `getPrincipalReadOnly(authUserId)`, `getSignupGateState()`,
  `submitSignupGates(values, idempotencyKey)` — the only module touching the gateway; error
  mapping from RFC 7807 ProblemDetails to typed results happens here (status + correlationId
  only; no `detail` passthrough).

## 4. Form validation

Client-side (presentation only — the server re-checks everything):

| Check | Trigger | Message key (EN/VI) |
|---|---|---|
| `countryCode` present | submit | `gates.country.required` |
| `terms === true` | submit | `gates.terms.required` |
| `risk === true` | submit | `gates.risk.required` |
| `age === true` | submit | `gates.age.required` |

Failures populate `MissingAcceptanceSummary` (all missing items at once, in field order) and
per-field inline errors; focus moves to the summary. Validation never disables Continue (spec
§2.3: missing items are named, not silently blocked). Only the three consent kinds can arrive
from the server (`ServerMissing`); `country` is client-only.

## 5. Accessibility and design-system conformance

- WCAG 2.2 AA (NFR25): labelled group (`fieldset`/`group` role) around the four fields;
  combobox follows WAI-ARIA APG; alert summary above the fields in DOM order + focus
  management (tab order = DOM order: summary links → country → terms → risk → age → continue);
  visible focus rings; contrast from tokens; error state = colour + icon + text (never colour
  alone).
- Parchment Ledger tokens only; financial figures do not appear in this unit, so no tabular
  mono/em-dash rules apply. No missing-value rendering needed (nothing numeric is shown).
- All strings via the existing i18n module, EN + VI (NFR26), including the generated country
  names; browser-locale fallback before any user preference exists.

## 6. Testing surface (MSW-backed, per story-map build order)

Build order: component contract first (states + accessibility), then the screen, then tests —
all tests land with the code (TDD). Per the story map, UI-unit tests are MSW-backed, mocking
the gateway (G1/G2/G3). **Dependency note:** MSW is currently a dev dependency of
`support-console/` only; adding it to `web/` is a new dev dependency and is recorded here for
the dependency review the organisation rules require (org memory: dependency changes are
recorded and reviewed). Same MSW major version as the support console.

| Test area | Cases (floor: happy path + ≥2 error/edge) |
|---|---|
| `RequireGatesPassed` | passes children when `passed`; renders gate screen when `gates-required`; renders blocked notice when `jurisdiction-blocked`; routes `owned-elsewhere` destinations; full-page retry on `error` |
| `GateForm` + summary | submit with all fields → G3 called with exact payload and idempotency key; submit missing items → summary names each (incl. country), focus moves, no G3 call; server 422 → G2 re-fetched and its `missing` rendered |
| `CountrySelect` | selects by search; keyboard-only operation; axe clean; EN/VI names render |
| Submit outcomes | 200 → app reachable; 403 → blocked notice, no retry control; 503 → `nothing-saved` notice, values kept; mid-sequence 5xx → `incomplete` notice, retry re-fetches G2 showing committed consents |
| Blocked notice | sign-up wording ("nothing stored"); continued-use wording; sign-out action |
| jest-axe | every rendered state of the screen |
