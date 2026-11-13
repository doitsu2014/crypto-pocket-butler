# Logical Components — u8-signup-gates-ui

**Upstream:** `functional-spec.md` (revision 2), `frontend-components.md` (revision 2),
`nfr-requirements/*`, `nfr-design/performance-design.md`, `nfr-design/security-design.md`.
**Unit kind:** ui — the inventory lives inside the existing `web/` SPA build; this unit adds no
service, no schema, and no deployable. Placement follows the repository's existing conventions:
feature code under `web/src/features/<feature>/` (one folder, page + tests together), shared
transport in `web/src/api/`, routing in `web/src/app/`.

## Component inventory inside the SPA

| Logical component | Kind | Responsibility | Depends on | Failure domain |
|---|---|---|---|---|
| Gate route registration (`SignupGatesPage`, lazy) + route-level error boundary + `RequireGatesPassed` guard | routing (`web/src/app/`) | Wraps post-sign-in product routes; renders from `GateStatus` only; never unlocks on unresolved/cached state; the page module's `*Page` name keeps it in the size-limit route glob; the route-level boundary keeps a chunk-load failure inside this route (the shell has none today) | `useGateStatus` | Routing shell; a guard bug is presentational only (NFR24.2 — server still enforces) |
| `useGateStatus` | hook (feature) | Single entry-resolution point: parallel G1/G2 queries, `GateStatus` union, sentinel mapping for "no record", full-page error state | `client.ts`, query client | Entry resolution — its failure surface is the full-page retry; it can never route into the app |
| `useSubmitGates` | hook (feature) | G3 mutation: `retry: 0`, 15 s deadline, outcome mapping (200/403/422/503/5xx→variant), invalidation on success | `client.ts`, key holder | One submission; failures stay inline on the screen, values kept |
| `SignupGatesScreen` | page (feature) | Screen states `loading/ready/submitting/blocked/error`; composes form, summary, notices; owns no decision logic | Hooks, form components | The gate route only |
| `GateForm` + `AcceptanceCheckbox` ×3 | components (feature) | Local form state; four presence checks on submit; labels/links per locale | i18n catalogues | Re-render scope isolated (§7 performance design) |
| `CountrySelect` | component (feature) | ARIA APG combobox over the static module; memoised filtering; stores alpha-2 code | Country module, i18n | Keystroke-local; no network per keystroke |
| `MissingAcceptanceSummary`, `BlockedJurisdictionNotice`, `GateErrorNotice(variant)` | components (feature) | Alert-role summary with focus move; terminal session notice (Sign out only); `nothing-saved`/`incomplete` inline errors with correlation id | i18n | Presentation of server answers only |
| Submission-ticket holder | module state (feature) | `crypto.randomUUID()` key above the screen, stamped with the session id and bound to one exact payload; survives remounts; discarded on success; a field edit mints a new key; never persisted | — | Feature- and session-scoped; loss on reload backstopped server-side (NFR11.2) |
| Country data module (generated) | static data (feature) | Frozen `{code, en, vi, search}` array; build-time generated (T2); no runtime fetch | Generation script (`web/`) | Route-chunk payload; regeneration is a build-time event |
| `client.ts` additions | transport (`web/src/api/`) | `getPrincipalReadOnly()` and `getSignupGateState()` — both against read-only, non-provisioning, self-scoped routes with sentinel-mapped no-record answers (dependency 1 in security-design §11); `submitSignupGates(values, key)` against the **new authenticated** G3 route; allowlist ProblemDetails mapping (never into the shared `ErrorPanel`, which renders `detail`); `callWithDeadline` | Gateway | Shared module — additions are additive; existing callers untouched |
| i18n catalogue additions (`gates.*`) | data (`web/src/i18n/`) | EN + VI for every string incl. country names via the module | Existing i18n module | Additive keys only; no resolution changes |
| MSW handlers for G1/G2/G3 | test infra (feature tests) | Mock the gateway for the suite incl. delayed/hung handlers (deadline tests) | `msw` (dev-only, T1) | Tests only; cannot reach the production bundle |

## Failure domains and blast radius

- **Gateway/Identity unavailable on entry** → `GateStatus.error` → full-page retry; no visitor
  reaches the app. Blast radius: this visitor's session; no other SPA feature's queries are
  touched (separate query keys, shared client only).
- **G3 fails or hangs** → inline error variant, values and key kept, manual retry. Blast radius:
  the one screen; the shell and other routes keep working.
- **Route-chunk load failure** (deploy/offline) → caught by the **route-level error boundary
  this unit ships** (the router has none today; without it the root `ErrorBoundary` would
  replace the whole app) → full-page retry inside the gate route. Blast radius: the gate route
  only; the rest of the shell stays mounted.
- **Guard/component defect** → worst case a screen renders that the server would refuse:
  every gate decision is re-enforced server-side (U4 consent gate, gateway table), so the blast
  radius of any client bug is presentational (NFR24.2, FR15.9).
- **Key-holder defect** (lost or duplicated key) → bounded by the server-side
  skip-already-recorded backstop (NFR11.2) and the G2 re-fetch after any failure: a duplicate
  submit completes rather than double-records.

## Isolation strategy

- The feature folder is self-contained: components, hooks, holder, data module, and tests live
  under one `web/src/features/signup-gates/` directory; the only files touched outside it are
  the route table (`web/src/app/` — the lazy route, its route-level error boundary, and the
  guard wrapper), the additive `client.ts` methods, and the two i18n catalogues. No size-limit
  config change is needed (`web/scripts/check-size.mjs` already measures every non-initial
  chunk). That list is the unit's whole blast radius in the SPA.
- No feature state is shared with other features: no context providers are added, gate query
  keys are namespaced, and nothing subscribes to another feature's cache entries.
- The feature imports no storage API, no JWT library, and no telemetry payload builder — the
  absence is part of the design (security-design §2/§3/§5), not an oversight.

## Shared resources

| Resource | Shared with | Isolation |
|---|---|---|
| `web/src/api/client.ts` + query client | Every SPA feature | Additive methods; query keys namespaced `["signup-gates", <sessionUserId>, …]`; deadline wrapper local to the new methods until adopted elsewhere |
| `web/src/i18n` catalogues | Every SPA feature | New `gates.*` namespace; no changes to resolution (`resolveLocale` untouched) |
| Auth session holder (`web/src/auth/`) | Shell | Read-only reuse; feature adds no token handling |
| Router / route chunking | Shell | One new lazy route (`SignupGatesPage`) + route-level error boundary + guard wrapper; existing routes unchanged |
| Chunk budget gates | `web/` build | `web/scripts/check-size.mjs` (every non-initial chunk) and the size-limit `route-*.js` glob via the `*Page` name — existing config, no new entry |
| Telemetry (`web/src/telemetry.ts`) | Every SPA feature | The feature calls no telemetry API; only the shell's existing render-crash beacon (route, errorCode, correlationId, sessionId — no gate data) can mention the route (security-design §5, NFR22.2) |
| Shared `ErrorPanel` | Other features' retry surfaces | **Not used** by this feature — it renders `detail`; gate error surfaces are dedicated components (security-design §5, NFR22.1) |

## Bridge to Infrastructure Design

Nothing in this unit changes infrastructure: it ships inside the existing SPA build and the
existing nginx/Swarm edge. The infrastructure-relevant outputs are the five recorded
dependencies in `security-design.md` §11 (route-table entries, the self-scoped principal route,
IP derivation) — all owned by the gateway and Identity units, not by this one.
