# Tech Stack Decisions — u8-signup-gates-ui

**Upstream:** `requirements.md` (constraints), codekb `technology-stack.md`, `functional-spec.md`
(revision 2), `frontend-components.md` (revision 2), project memory mandates.

## 1. Mandated stack (confirmed, no substitution)

The project's mandates fix every layer this unit touches; this section records the confirmation
and the version each choice pins to today (codekb `technology-stack.md`):

| Layer | Choice | Version in repo | Mandate source |
|---|---|---|---|
| UI framework | React + TypeScript | 18.3.1 / ^5.7.2 | project memory (ALWAYS React 18 + TS 5) |
| Build | Vite | ^6.0.7 | project memory |
| Routing | react-router-dom | ^6.30.1 | existing `web/` shell |
| Server state | TanStack Query | ^5.62.11 | project memory |
| Auth | @supabase/supabase-js | ^2.47.10 | project memory (SPA → gateway + Supabase Auth only) |
| Styling | Tailwind CSS v4 → Parchment Ledger tokens | ^4.0.0 | project memory (no raw values) |
| Tests | Vitest 3 + Testing Library + jest-axe | ^3.0.0 / ^16.1.0 / ^10.0.0 | team testing posture |
| Bundle gate | size-limit | ^14.3.0 | `web/` budgets (300/150/150 kB gzip) |
| Lint/format | ESLint 9 flat + Prettier 3 | ^9.17.0 / ^3.4.2 | project memory |
| Package manager | Yarn 4.18.1 via corepack | pinned | project memory (npm/npx/pnpm forbidden) |

No database, infrastructure, or backend choice belongs to this unit: it ships inside the
existing `web/` SPA build, calls only the API Gateway, and adds no service.

## 2. Unit-level decisions

### T1 — HTTP mocking for tests: MSW as a new `web/` dev dependency

- **Context.** The story map fixes UI-unit tests as MSW-backed (the support console already
  mocks the gateway with `msw ^2.7.0`); `web/` does not have it today.
- **Decision.** Add `msw ^2.7.0` to `web/` as a **devDependency**, the same major/minor line
  the support console uses. Recorded here for the dependency review the organisation rules
  require ("dependency changes are recorded and reviewed").
- **Consequences.** Gateway handlers (G1/G2/G3) are written once against the real request/response
  shapes; tests exercise the actual `client.ts` fetch path; dev-only, zero production bundle
  impact.
- **Alternatives rejected.** (a) Hand-rolled `fetch` stubs — rejected: diverges from the
  affirmed UI-unit testing pattern and skips the real client path; (b) `nock` — rejected:
  node-interceptor level, not the browser-fetch model Vitest+jsdom runs; (c) reusing the
  support console's handlers wholesale — rejected: different gateway surface, but the handler
  *style* is copied.

### T2 — Country data: build-time-generated static module, zero runtime dependency

- **Context.** Q2 (full ISO 3166-1 list) plus NFR26.1 (EN + VI names) plus NFR9.1 (no runtime
  third-party origin) require a country dataset in the bundle.
- **Decision.** A generated TypeScript/JSON module committed to the repo: alpha-2 codes and
  English names from the ISO 3166-1 table, Vietnamese names from Unicode CLDR locale data,
  produced by a script in `web/` (regeneration is a Code Generation task; spec §6 open item 5
  names who regenerates when CLDR changes).
- **Consequences.** No network fetch on the entry path; ~10–20 kB inside the 150 kB gate-route
  chunk (NFR2.1); deterministic tests; the dataset is reviewable in-repo like any dependency
  change.
- **Alternatives rejected.** (a) Runtime fetch of a country dataset — rejected: violates the
  gateway-only rule for browser calls (NFR9.1) and adds an availability dependency to the
  entry path; (b) `i18n-iso-countries` or similar runtime package — rejected: adds a runtime
  dependency whose VI coverage still needs verifying, for data that changes a few times a year;
  (c) hand-maintained list — rejected: drifts from ISO/CLDR with no regeneration path.

### T3 — Localisation: the existing `web/src/i18n` module

- **Context.** NFR26.1 requires EN/VI for every gate-flow string.
- **Decision.** Reuse the SPA's existing i18n module and message-catalogue conventions; add the
  gate-flow keys (`gates.*`) to both catalogues. No new i18n library.
- **Consequences.** Locale resolution follows the shell's existing `resolveLocale` order
  unchanged (`?lang=` → stored server preference → browser locale → English — NFR26.2); no new
  dependency.
- **Alternatives rejected.** A dedicated micro-translation layer for the feature — rejected:
  forks the locale resolution the shell already owns.

### T4 — Idempotency key generation and lifetime: Web Crypto + feature-level in-memory holder

- **Context.** G3 requires an idempotency key per pending submission (spec §2.4), reused by
  retries and by a re-fetch-then-retry after a 422 (NFR11.1). Holding it inside the screen
  component would mint a new key on every in-app remount — a route bounce mid-failure could
  then double-record a consent grant.
- **Decision.** Generate with the platform `crypto.randomUUID()` and hold the key in a
  **feature-level in-memory holder above the screen** (module scope of the gate feature, or a
  context provider at the shell level): the key is created on first entry to a pending
  submission, survives in-app remounts and navigations during that submission, and is discarded
  on submit success or sign-out. Never persisted (NFR10.1).
- **Consequences.** A full page reload loses the key. The end-to-end idempotence guarantee
  therefore does not rest on the client key alone: G3/C6 must treat already-recorded consent
  kinds as a no-op (recorded as the NFR11.2 contract dependency on C6 idempotency, to be
  confirmed before Code Generation).
- **Alternatives rejected.** A counter+timestamp composite — rejected: weaker uniqueness for no
  gain; component-local state — rejected: a new key per remount breaks retry idempotence
  (NFR11.1); persisting the key across reloads (storage or cookie) — rejected: violates NFR10.1,
  and the reload case is covered server-side by NFR11.2 instead.

## 3. Open dependencies on other units (not stack choices)

Two of the routes this unit needs **already exist** in the repository, so the open list is what
is missing from them, not the routes themselves:

- **`GET /v1/me/consent/state`** is already declared in the gateway route table and served by
  Identity (U4). What is missing: the `jurisdictionBlocked` field the gate status needs
  (spec §6 open item 3 — U6 owns the restricted list and its source), and confirmation that
  this existing route *is* G2 rather than a second gate-state endpoint being added
  (spec §6 open item 2).
- **`POST /v1/signup-gate`** is already served by Identity. What is missing: confirmation that
  this is the single submit route G3 uses (rather than a new `/v1/signup/gates`), its
  `Idempotency-Key` handling, and the C6 skip-already-recorded guarantee NFR11.2 depends on.
- **G1's read-only self-scoped shape**: the functional spec's `GET /v1/principals/{authUserId}`
  puts the raw provider subject in a URL the edge logs by default, which NFR22.3 forbids. The
  needed change is a read-only `/v1/me/...`-style principal route identified from the validated
  token (gateway/U1, spec §6 open item 1); it must land before Code Generation or the exposure
  must be accepted as an explicit recorded deviation.
- **Gateway IP-derivation policy** for C8's `ip` field from the trusted proxy chain
  (spec §6 open item 4 — gateway).
