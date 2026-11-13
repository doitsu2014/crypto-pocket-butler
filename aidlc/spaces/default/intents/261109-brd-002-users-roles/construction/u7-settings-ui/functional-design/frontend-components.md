# Frontend Components — U7 Settings UI (`u7-settings-ui`)

**Stack:** React 18 + TypeScript 5, Vite, TanStack Query, React Router 6, Tailwind CSS v4 mapped
to Parchment Ledger tokens (no raw colour/spacing/type values), `@supabase/supabase-js` for auth.
**Location:** `web/src/features/settings/` (the shell, profile, preferences, consent, privacy and
delete screens), `web/src/features/account-status/` (restore and blocked screens), and
`web/src/components/` for the one shared component of §11 of the spec. All gateway calls live
in `web/src/api/client.ts` — components never call the gateway directly.
**Upstream:** `functional-spec.md` (this unit) — screens, states, and endpoints D0/P1/P2/D1/D2/D3
and X1–X7.
**Test tooling (already present in `web/package.json`):** Vitest 3, Testing Library, `jest-axe`,
and MSW 2. No dependency is added by this unit.

## 1. Component hierarchy

```
web/src/app/router.tsx
├── /settings                        SettingsLayout            (gatedPage)
│   ├── index → Navigate /settings/profile
│   ├── profile                      ProfilePanel
│   ├── preferences                  PreferencesPanel
│   ├── consent                      ConsentPanel
│   └── privacy                      PrivacyPanel
├── /settings/delete                 DeleteAccountPage          (gatedPage, outside the shell)
├── /account/restore                 RestoreAccountPage         (protectedPage)
├── /account/blocked                 AccountBlockedPage         (protectedPage)
└── /privacy                         Navigate /settings/privacy  (replaces the old page)

web/src/features/signup-gates/
└── gateStatus.ts                    status → destination (pending_deletion now → restore),
                                     reading D0 instead of the denying principal read

web/src/components/
└── StatusPill                       shared (U8 hand-off, U9, U10)

SettingsLayout
├── SettingsNav                      four links, aria-current="page"
├── QueryBoundary                    loading / error (existing component)
└── <Outlet/>                        the active panel

ProfilePanel
├── PageHeader                       (existing)
├── DisplayNameForm                  input + Save + "Use my provider name again"
├── EmailField                       read-only + Verified badge
└── StatusPill                       from the profile read

PreferencesPanel
├── SelectField × 3                  locale, base currency, timezone (searchable)
├── SwitchField × 2                  email, push
└── StatusRegion                     save feedback (existing)

ConsentPanel
├── RequiredAcceptanceList           terms / risk / age — read-only, dated
└── AnalyticsConsentSwitch           switch + withdrawal date

PrivacyPanel
├── ConsentHistoryTable              DataTable (existing)
├── ExportControl                    unchanged behaviour
└── DangerZone                       link to /settings/delete

DeleteAccountPage
├── DeleteConsequences               "what happens" list
├── RetainedCategories               service list when non-empty, else the policy sentence
├── ConfirmDialog                    extended shared component (§5)
└── OpenRequestNotice                request date + exact erase date + Restore

RestoreAccountPage
├── RecoveryBanner                   role="status", exact date (Q6)
└── RestoreActions                   Restore (primary) + Sign out

AccountBlockedPage
└── variant: "disabled" | "deleted"  notice + Sign out
```

## 2. Components, props, and state

Interface-level contracts (the implementation lands in Code Generation, TDD):

```ts
// web/src/api/types.ts — the new reads (spec §8)
export interface AccountStatusView {
  userId: string;
  status: "active" | "disabled" | "pending_deletion" | "deleted";
}

export interface ProfileView {
  userId: string;
  displayName: string | null;
  displayNameOverridden: boolean;
  providerDisplayName: string | null;
  email: string | null;
  emailVerified: boolean;
  status: "active" | "disabled" | "pending_deletion" | "deleted";
}

export interface DeletionView {
  deletionId: string;
  status: string;
  requestedAt: string;
  recoveryEndsAt: string;      // the exact date the banner shows
  retainedCategories: Array<{ category: string; basis: string }>;
  cancelledAt?: string | null;
  erasedAt?: string | null;
}

// web/src/features/signup-gates/gateStatus.ts — U7's refinement (spec §2.1)
type Destination = "verify-email" | "enrol-mfa" | "restore-account" | "account-blocked";
// status "pending_deletion" → "restore-account";  "disabled" | "deleted" → "account-blocked"
```

| Component | Props | Internal state / notes |
|---|---|---|
| `SettingsLayout` | — | Renders `SettingsNav` + `<QueryBoundary>` around `<Outlet/>`; the shell itself reads nothing, so a panel's failure does not blank the navigation. |
| `SettingsNav` | `items` | Plain router links in a labelled `nav`; the active link carries `aria-current="page"`. Keyboard order is DOM order; no `tablist` roles (these are navigations, not tabs). |
| `ProfilePanel` | — | `useProfile()` (P1). Owns the input's draft value only while dirty; on save it re-renders from the query's fresh data. The reset action renders only when `displayNameOverridden` and is disabled with a hint when `providerDisplayName === null`. |
| `DisplayNameForm` | `profile: ProfileView`, `onSave(name)`, `onUseProviderName()`, `pending`, `error?` | Trims before submit; keeps the typed value on a 422 so the person can correct it; the empty/too-long check mirrors the server's 1–200 rule but the server's answer wins. |
| `EmailField` | `email`, `verified` | Read-only text plus a `Verified` badge; no input element, so it cannot be posted. A missing email renders an em dash. |
| `PreferencesPanel` | — | `usePreferences()` (X2). Holds a draft of all five fields; Save sends only the changed ones. Each field shows the server's `source` as a quiet label. Timezone options come from `Intl.supportedValuesOf("timeZone")`, filtered by the typed prefix — no bundled dataset. |
| `SelectField` | `label`, `value`, `options`, `onChange`, `error?`, `searchable?` | Native `select` for the closed sets; a `combobox` (WAI-ARIA APG) only for timezone, where the list is long. |
| `SwitchField` | `label`, `checked`, `onChange`, `description?` | `role="switch"` + `aria-checked`; label is the accessible name; the state is also written in text beside it, never colour alone. |
| `ConsentPanel` | — | Reads X3 + X4. Required kinds render read-only with their latest `granted` date. Analytics renders a switch; after a mutation both queries are invalidated and the switch re-renders from the server (never optimistically). |
| `AnalyticsConsentSwitch` | `granted`, `withdrawnAt?`, `pending`, `onToggle(granted)` | The withdrawal date line appears only when the latest analytics event is a withdrawal. |
| `PrivacyPanel` | — | Consent history via the existing `DataTable`, the existing export control, and a `DangerZone` link. It initiates no deletion call. |
| `DeleteAccountPage` | — | Reads D1. Two states: `no-request` (consequences + retained categories + request control) and `open-request` (`OpenRequestNotice`). On D3 `202` it invalidates D1 and the principal and shows the open-request state in place. |
| `ConfirmDialog` (existing, extended) | `open`, `title`, `description`, `confirmLabel`, `onConfirm`, `onCancel` | Additive change: focus moves into the dialog on open, Tab is trapped, Escape calls `onCancel`, and focus returns to the trigger on close. The delete page is its first user; the privacy export dialog keeps its behaviour. |
| `OpenRequestNotice` | `request: DeletionView`, `onRestore()`, `pending`, `error?` | Shows `requestedAt` and the exact `recoveryEndsAt`; Restore is the primary action. Shared by the delete page and the restore page so the two render identically. |
| `RestoreAccountPage` | — | Reads D1; on `404` re-resolves the principal and routes by status (spec §2.1). On `410` swaps the banner for the terminal "window closed" state. |
| `RecoveryBanner` | `recoveryEndsAt`, `children?` | `role="status"` + `aria-live="polite"`; the date is formatted with `Intl.DateTimeFormat` in the profile locale and timezone. |
| `AccountBlockedPage` | `variant: "disabled" \| "deleted"` | Terminal notice + support contact (disabled only) + Sign out. Renders no settings control. |
| `StatusPill` | `status: UserStatus`, `restoreDate?: string` | One label + dot per status; for `pending_deletion` the restore date is text beside the pill, not inside it. |

## 3. Data flow and hooks

- `useAccountStatus()` — `GET /v1/me/account-status` (D0), key `["me","account-status"]`. The
  routing read: it answers for every status, including the three the principal read refuses, and
  it is what `gateStatus` consumes. Invalidated by the restore and deletion mutations.
- `useProfile()` — `GET /v1/me/profile` (P1), key `["me","profile"]`. Invalidated by the profile
  mutation and by the restore mutation (status changes).
- `useSaveDisplayName()` / `useUseProviderName()` — `PATCH /v1/me/profile` (P2). On success the
  response *is* the new `ProfileView`; it is written into the cache and the query invalidated.
  `422` → field error; `409` → an inline hint on the reset action.
- `usePreferences()` / `useSavePreferences()` — X2. The mutation sends the changed subset only.
- `useConsentState()`, `useConsentHistory()`, `useGrantConsent()`, `useWithdrawConsent()` — X3–X6.
  Every mutation invalidates both reads and the panel re-renders from the server.
- `useDeletionRequest()` — `GET /v1/me/deletion-request` (D1), key `["me","deletion"]`. A `404` is
  a valid answer, not an error: the hook returns `null` and the page renders its no-request state.
- `useRequestDeletion()` — `POST /v1/privacy/deletions` (D3, no body). `202` → invalidate D1 and
  the principal.
- `useCancelDeletion()` — `DELETE /v1/me/deletion-request` (D2). `200` → invalidate D1 and the
  principal; `410` → the terminal state.
- `client.ts` additions: `getAccountStatus()`, `getProfileView()`, `saveDisplayName(name)`,
  `useProviderName()`, `getOpenDeletion()`, `cancelDeletion()`, `requestDeletion()` (rewired to
  `/v1/privacy/deletions`).
  Dead calls removed: the old `getProfile()` (`/users/me`), `getPreferences()`/`updatePreferences()`
  (`/preferences`), `getConsent()`/`postConsent()` (`/consent`), and `listSessions()`/
  `revokeSession()` (`/sessions`). RFC 7807 mapping to typed results happens here — status and
  correlation id only, never `detail`.

## 4. Form validation

Client-side (presentation only — the server re-checks everything):

| Check | Trigger | Message key (EN/VI) |
|---|---|---|
| Display name non-empty after trim, ≤200 chars | save | `settings.profile.name.invalid` |
| Locale in `{en, vi}` | save | `settings.preferences.locale.invalid` |
| Base currency in `{USD, EUR, GBP, VND}` | save | `settings.preferences.currency.invalid` |
| Timezone parses via `Intl.DateTimeFormat` | save | `settings.preferences.timezone.invalid` |
| Deletion confirmed in the dialog | request | `settings.delete.confirm` |

A server `422` renders per-field messages from `errors[].field` and focus moves to the first
invalid field. Switches have no client rule beyond a failed write. The deletion dialog is the
only destructive gate and needs no typed phrase (spec §5).

## 5. Accessibility and design-system conformance

- WCAG 2.2 AA (NFR25): labelled `nav` for settings, `aria-current="page"` on the active link;
  every control labelled; `role="switch"` + `aria-checked` for the switches; `aria-describedby`
  ties each error to its field; save feedback via the existing `StatusRegion` (`role="status"`,
  `aria-live="polite"`); the recovery banner is a live region so a screen reader announces it on
  arrival; the delete dialog traps focus, closes on Escape, and returns focus to its trigger;
  focus rings are visible; errors pair colour with an icon and text.
- Parchment Ledger tokens only: `--accent`, `--surface`, `--surface-warm`, `--border`, `--fg`,
  `--fg-2`, `--muted`, `--success`, `--warn`, `--danger`, `--text-xs` (the 10px overline badge
  label), `--font-display` / `--font-body`. No raw value anywhere; a `tailwind` utility that would
  need one gets a token instead.
- Missing values render an em dash, never `0` or `$0.00`. No computed financial figure appears in
  this unit.
- Status is never colour alone: every `StatusPill` pairs its colour with a word and a dot.
- Dates and numbers go through `Intl` with the profile locale; all copy is EN + VI through the
  existing i18n module.

## 6. Testing surface (MSW-backed, per the story map's build order)

Build order: shared components and their contracts first, then each panel, then the two
account-status screens, then the router wiring — every test lands with its code (TDD). MSW mocks
the gateway; no test reaches a real service.

| Test area | Cases (floor: happy path + ≥2 error/edge) |
|---|---|
| `ProfilePanel` | saves a new name (PATCH body exact) and re-renders from the response; 422 keeps the typed value and shows the field error; reset action hidden when not overridden, disabled when no provider name, and sends `useProviderName` when used; a 409 renders the hint with nothing changed |
| `PreferencesPanel` | Save sends only the changed fields; a 422 names each invalid field; server `source` labels render; a read failure shows the panel error without blanking the nav |
| `ConsentPanel` | analytics toggles on with the policy version and off via withdraw; `409` re-reads and renders the server state; required kinds render read-only with their dates; the withdrawal date appears only after a withdrawal |
| `DeleteAccountPage` | no-request state renders consequences; the dialog must be confirmed before D3 is called; `202` switches to the open-request state; an existing request renders the exact date and Restore; `retainedCategories` empty → the policy sentence, non-empty → the list |
| `RestoreAccountPage` | D1 renders the exact date in the profile locale; Restore calls D2 and routes to `/settings/profile`; `410` renders the terminal state with no restore control; D1 `404` re-resolves and routes by status |
| `AccountBlockedPage` | disabled variant names support and offers no self-service; deleted variant is terminal; both offer Sign out only |
| `gateStatus` routing | `pending_deletion` → `restore-account`; `disabled` and `deleted` → `account-blocked`; the previous `DestinationUnavailable` fallback is gone for both routes |
| `StatusPill` | every status renders its label and dot; the pending-deletion variant carries the restore date as text beside the pill, so the state never reads by colour alone |
| `useAccountStatus` client | maps D0 `200` to the typed view and a `401` to a signed-out result; a `503` is retryable and never routed as a status |
| `requestDeletion` client | posts to `/v1/privacy/deletions` with no body; maps `409` to "already open" and `503` to a retryable result |
| jest-axe | every rendered state of the shell, the four panels, the delete page, and both account-status screens |