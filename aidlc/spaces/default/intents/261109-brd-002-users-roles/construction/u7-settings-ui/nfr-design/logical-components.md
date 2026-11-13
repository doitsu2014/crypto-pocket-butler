# Logical Components — u7-settings-ui

**Upstream:** `functional-design/frontend-components.md` (revisions: §1 hierarchy, §2 props and
state, §3 hooks), `functional-design/functional-spec.md` (§2 shell and entry routing, §3 screens,
§8 API integration points), `nfr-design/performance-design.md`, `nfr-design/security-design.md`,
`nfr-requirements/tech-stack-decisions.md` (T1–T6).
**Unit kind:** ui. The inventory lives inside the existing `web/` SPA build and adds six routes to
the existing Identity service; it adds no service, no schema, and no deployable. Placement follows
the repository's conventions: feature code under `web/src/features/<feature>/`, shared transport in
`web/src/api/`, shared components in `web/src/components/`, routing in `web/src/app/`.

## 1. SPA components

| Logical component | Kind | Responsibility | Depends on | Failure domain |
|---|---|---|---|---|
| `SettingsLayout` + `SettingsNav` | routing / layout (`web/src/app/`, `features/settings/`) | Labelled `nav` with four links and `aria-current="page"` around a bare `<Outlet/>`. The layout runs no query and passes nothing to a boundary, so the navigation paints before any panel's data (NFR2.3) and a panel's failure cannot blank it. It deliberately does **not** wrap the outlet in `QueryBoundary`: that component requires `loading`/`error` from a caller with a query, and its error branch renders `ErrorPanel`, which renders `ProblemDetails.detail` (security-design §5). The entry surface's own resolution error and each panel's own error component handle their states instead | Router | Shell of the settings area only; a panel's failure never blanks the nav |
| `ProfilePanel` + `DisplayNameForm` + `EmailField` | page + components (feature) | Renders P1: display names, read-only email with the `Verified` badge, the `StatusPill`, the save and "use my provider name again" actions. Owns the input draft only while dirty | `useProfile`, `useSaveDisplayName`, `useUseProviderName` | One panel: its own query and mutation; the other three keep rendering |
| `PreferencesPanel` + `SelectField` ×3 + `SwitchField` ×2 | page + components (feature) | Renders X2, drafts all five fields, sends only the changed subset, shows each field's server `source`. Timezone list from `Intl.supportedValuesOf` (T2) | `usePreferences`, `useSavePreferences` | One panel |
| `ConsentPanel` + `RequiredAcceptanceList` + `AnalyticsConsentSwitch` | page + components (feature) | Renders X3 (required kinds, read-only, dated) and X4 (analytics switch + withdrawal date); re-renders from the server after every mutation, never optimistically | `useConsentState`, `useConsentHistory`, `useGrantConsent`, `useWithdrawConsent` | One panel; a consent failure never affects the other panels |
| `PrivacyPanel` + `DangerZone` | page (feature) | Consent history through the existing `DataTable`, the existing export control, and the link to `/settings/delete`. Initiates no deletion call itself | `useConsentHistory`, existing export hooks | One panel |
| `DeleteAccountPage` + `DeleteConsequences` + `RetainedCategories` + `OpenRequestNotice` | page + components (feature) | Two states from D1 — `no-request` (consequences, retained categories, the request control) and `open-request` (`OpenRequestNotice`). D3 `202` swaps state in place; `409` re-reads; a lost answer re-reads rather than assuming success | `useDeletionRequest`, `useRequestDeletion`, `ConfirmDialog` | The delete route only |
| `RestoreAccountPage` + `RecoveryBanner` + `RestoreActions` | page + components (feature, `features/account-status/`) | Reads D1 for the exact `recoveryEndsAt`; Restore (D2) is primary; a `404` re-resolves the status and routes by it; `410` swaps to the terminal view with no second round trip | `useDeletionRequest`, `useCancelDeletion`, `useAccountStatus` | The restore route only |
| `AccountBlockedPage` | page (feature) | Terminal notice for `disabled` / `deleted` with sign-out (and a support contact for `disabled`). Renders no settings control | server-reported status | The blocked route only |
| `ConfirmDialog` (existing, extended in place — T5) | shared component (`web/src/components/`) | Focus moves in on open, Tab is trapped, Escape cancels, focus returns to the trigger on close (NFR25.4). Used by the delete flow and the analytics withdrawal | — | Shared: one focus-trap implementation, so a defect is app-wide — which is why it is tested once, thoroughly |
| `StatusPill` (shared) | shared component (`web/src/components/`) | One label, one dot shape and one token colour per status; the restore date renders as text beside the pill, never inside it and never by colour alone (NFR25.5) | design tokens | Shared: presentational only |
| `gateStatus.ts` (extended) | module (feature, `features/signup-gates/`) | Extends the existing status→destination map with `pending_deletion → restore-account` and `disabled`/`deleted` → `account-blocked`, now reading D0 instead of the denying principal read | `AccountStatusView` | Routing decision — presentational only; the server re-decides every request (security-design §9) |
| `useAccountStatus()` | hook (feature) | The single entry-resolution point: one D0 read, `staleTime: 0`, key `["me","account-status"]`, full-page retry on failure. Never routes into the app on an unresolved status | `client.ts`, query client | Entry resolution; its failure surface is the full-page retry |
| `useProfile` / `useSaveDisplayName` / `useUseProviderName` | hooks (feature) | P1 read and P2 mutations; the P2 response *is* the new `ProfileView`, written into the cache and the query invalidated; `422` → field error, `409` → inline hint on the reset action | `client.ts` | One panel |
| `usePreferences` / `useSavePreferences` | hooks (feature) | X2 read and write, changed subset only | `client.ts` | One panel |
| `useConsentState` / `useConsentHistory` / `useGrantConsent` / `useWithdrawConsent` | hooks (feature) | X3–X6; every mutation invalidates both reads | `client.ts` | One panel |
| `useDeletionRequest` / `useRequestDeletion` / `useCancelDeletion` | hooks (feature) | D1/D3/D2; D1's pinned no-record `404` resolves to `null` (T6); D3 and D2 invalidate `["me","deletion"]` **and** `["me","account-status"]` because a deletion changes the routing status | `client.ts` | The two account-status routes |
| `client.ts` additions | transport (`web/src/api/`) | `getAccountStatus()`, `getProfileView()`, `saveDisplayName()`, `useProviderName()`, `getOpenDeletion()`, `cancelDeletion()`, `requestDeletion()` (rewired to `/v1/privacy/deletions`); RFC 7807 mapping to typed results — status, title and correlation id only, never `detail`; removal of the dead `/users/me`, `/preferences`, `/consent`, `/sessions` calls | Gateway | Shared module: additions are additive; the removals are of calls nothing in the shell uses after this unit |
| i18n catalogue additions (`settings.*`) | data (`web/src/i18n/`) | EN + VI for every string in §4 of the components spec and every screen state | Existing i18n module | Additive keys only; `resolveLocale` untouched (NFR26.2) |
| MSW handlers for D0/P1/P2/D1/D2/D3/X2–X6 | test infra (feature tests) | Mock the gateway for the suite, including delayed and hung handlers for the deadline tests and the pinned sentinel for D1's empty state | `msw` (already a dev dependency) | Tests only; cannot reach the production bundle |

## 2. Identity route additions

| Route | Kind | Responsibility | Failure domain |
|---|---|---|---|
| `GET /v1/me/account-status` (D0) | handler + projection | Answers the caller's own stored status for every status, including the three the denying principal read refuses. Takes no id (security-design §2) | Identity's settings surface; a defect here strands or wrongly blocks one caller |
| `GET /v1/me/profile` (P1), `PATCH /v1/me/profile` (P2) | handler + use case | The exact `ProfileView` projection and the display-name write. `userId` never rendered; the provider subject never projected | The profile panel |
| `GET /v1/me/deletion-request` (D1) | handler + projection | The caller's own open request, or the pinned no-record `404` | The delete and restore routes |
| `DELETE /v1/me/deletion-request` (D2) | handler + use case | Cancels the caller's own open request; `410` after the window | The restore route |

All four sit on Identity's existing router and its existing use cases and repositories; the only new
code is the projection and the stored-record caller lookup the deletion handlers already use. No new
crate, no new schema, no new migration, no new service (T1, spec §8).

The gateway entries for these six routes, and for the two export routes, are **gateway** components
this unit does not own; they are recorded as dependencies in `security-design.md` §14 so Code
Generation cannot assume them.

## 3. Failure domains and blast radius

- **D0 unavailable on entry** → the full-page retry surface; no route into the settings area. Blast
  radius: this person's session; no other SPA feature's queries are touched (separate keys, shared
  client only).
- **One panel's query fails** → that panel's inline error with retry; the nav, the other panels and
  the shell keep rendering. This is the reason the layout owns no query.
- **A save fails or hangs** → the field keeps its value and the error is inline with the
  correlation id; nothing else on the page changes.
- **A deletion or restore answer is lost** → the surface says so and re-reads D1; it never reports
  the irreversible action as done on a lost answer.
- **`ConfirmDialog` defect** → app-wide, because the component is shared. Accepted: one focus-trap
  implementation tested once is safer than two, one of which would be untested (T5).
- **A client status or guard defect** → worst case a screen renders that the server would refuse;
  every decision is re-enforced server-side, so the blast radius is presentational (NFR24.3).

## 4. Isolation strategy

- Two self-contained feature folders — `web/src/features/settings/` and
  `web/src/features/account-status/` — carry the pages, hooks, components and their tests. The only
  files touched outside them are the route table (`web/src/app/`), the existing
  `features/signup-gates/gateStatus.ts` extension, `web/src/api/client.ts` and `types.ts`, the two
  shared components of §1, and the two i18n catalogues. That list is the unit's whole blast radius in
  the SPA.
- No new context provider and no shared feature state: every read is a namespaced `["me", …]` query
  the panel that needs it owns.
- The feature imports no storage API and no JWT library; the absence is part of the design
  (security-design §3), not an oversight.

## 5. Shared resources

| Resource | Shared with | Isolation |
|---|---|---|
| `web/src/api/client.ts` + query client | Every SPA feature | Additive methods and the two documented removals; keys namespaced `["me", …]`; the app's `shouldRetry` and `mutations: retry: false` are used unchanged, not overridden (T6) |
| `web/src/api/types.ts` | Every SPA feature | Three new exported interfaces; no existing type changed |
| `web/src/i18n` catalogues | Every SPA feature | New `settings.*` namespace; resolution untouched |
| `ConfirmDialog` | Existing dialogs (privacy export) | Additive focus behaviour only; existing callers keep their behaviour |
| `StatusPill` | U8 hand-off, U9, U10 | Presentational; the unit defines no new status vocabulary |
| Auth session holder (`web/src/auth/`) | Shell | Read-only reuse; the feature adds no token handling |
| Router / route chunking | Shell | Four `/settings/*` routes, `/settings/delete`, `/account/restore`, `/account/blocked`; all lazy, all `*Page`-named so the size-limit route glob keeps measuring them (NFR2.1) |
| Chunk budget gates | `web/` build | Existing `web/scripts/check-size.mjs` and the size-limit route glob; no new config entry |
| Shared `QueryBoundary` / `ErrorPanel` | Other features' retry surfaces | **Not used** by this unit: `QueryBoundary`'s error branch renders `ErrorPanel`, which renders `ProblemDetails.detail` (`web/src/components/index.tsx`). Each surface renders its own error component with title, status and correlation id only (security-design §5) |
| Identity router and use cases | All Identity clients | Additive routes on the existing router; the two deletions handlers' stored-record lookup is reused, not modified |

## 6. Bridge to Infrastructure Design

Nothing in this unit changes infrastructure. It ships inside the existing SPA build and the existing
nginx/Swarm edge, and its server additions are routes on a service that already runs. The
infrastructure-relevant outputs are the five recorded dependencies in `security-design.md` §14 —
gateway table entries, the D3 rate-limit budget, and the pinned no-record problem type — all of
which land in the gateway and Identity code this unit already touches, not in the deployment.