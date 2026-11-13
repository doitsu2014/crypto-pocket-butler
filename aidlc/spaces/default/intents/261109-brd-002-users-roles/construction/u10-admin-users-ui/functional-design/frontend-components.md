# Frontend Components — U10 Admin Users UI

**Unit:** `u10-admin-users-ui` · **Stage:** Functional Design · **Package:** `support-console/`
**Stack:** React 18 · TypeScript 5 · Vite 6 · TanStack Query 5 · `@supabase/supabase-js` · Tailwind
CSS v4 mapped to the Parchment Ledger tokens · Yarn 4.18.1
**Upstream:** `unit-of-work.md` (U10 — "Embedded in the support console admin section"),
`unit-of-work-story-map.md` (US7.1 → U10; U1; U2), `stories.md` (US7.1, AC7.1.1–AC7.1.2),
`requirements.md` (FR15.4, FR15.9, FR17.2, FR20.1–FR20.4, NFR24–NFR26), `components.md`
(`AdminUsersUI`), `contract-summary.md` (C1/C2, C3, C4, C5, C7), and this unit's
`functional-spec.md` (§ references below are to that file).
**Test tooling:** Vitest 3 + Testing Library + MSW 2 + jest-axe, as the console already uses.
**No dependency is added by this unit.**

## 1. Component hierarchy

The console is a hash-routed shell (`src/screens/SupportShell.tsx`) with section components. U10
adds one section and its parts; the approval queue is reused unchanged.

```
SupportShell
├── GrantBanner                       (unchanged — active grant clock)
├── nav: sections from SECTIONS[]     (+ { id: "admin-users", label: "Admin users", adminOnly: true })
├── AdminUsersSection                 (new, lazy — src/screens/admin/AdminUsersSection.tsx)
│   ├── AdminUserLookup               (new — exact-email form, §3.1)
│   ├── AdminUserRecord               (new — the read-only record view, §3.2)
│   │   ├── RoleBadge                 (new shared — src/components/RoleBadge.tsx)
│   │   ├── StatusBadge               (reused)
│   │   ├── RoleChangeForm            (new, §3.3)
│   │   └── StatusChangePanel         (new, §3.4 — disable / restore with confirmation)
│   └── GrantApprovalQueue            (reused as-is — the Approvals panel, §3.5)
└── StatusFooter                      (unchanged)
```

`AdminUsersSection` replaces nothing: the existing `grants`, `search`, `tickets`, `incidents` and
`help` sections keep their behaviour. It is rendered lazily (`React.lazy`) like
`IncidentConsole`, because only administrators open it.

## 2. Interfaces and state

### 2.1 Types

```ts
// src/api/types.ts — added
export type ConsoleRole = "owner" | "admin" | "support";

// The five user statuses the admin answer reports. The console has no shared
// account-status union today (AccountMetadata.status is a bare `string`), so
// this unit defines it here and owns the presentation mapping (§5).
export type AdminUserStatus =
  | "pending"
  | "active"
  | "disabled"
  | "pending_deletion"
  | "deleted";

export interface AdminUserRecord {
  id: string;                 // internal user id — the identifier every change targets
  displayName?: string;
  email?: string;
  emailVerified: boolean;
  status: AdminUserStatus;
  role: ConsoleRole;          // W1 — the provider's current role at lookup time
  createdAt: string;
  updatedAt: string;
}

export interface RoleChangeBody {
  newRole: ConsoleRole;
  reason: string;             // trimmed, 1–500
}

export interface RoleChangeResult {
  userId: string;
  previousRole: ConsoleRole;
  newRole: ConsoleRole;
}

export interface AdminStatusBody {
  reason: string;             // trimmed, 1–500
}

export interface AdminStatusResult {
  userId: string;
  status: AdminUserStatus;    // W2/W3 answer — the delivered status
  transitionId: string;
  occurredAt: string;
}
```

`ConsoleRole` is deliberately distinct from `SupportRole` (`"support" | "admin"`) in
`src/auth/session.ts`: `SupportRole` is what can sign into this console (BR1.1), `ConsoleRole` is
what an administrator can *assign*. Nothing converts one into the other. The role the record shows
is the provider's current `app_metadata.role`, read by the lookup (W1), not a stored copy; a
lookup whose provider read fails renders the `503` message and no record rather than a stale role.

### 2.2 Components

| Component | Props | State |
|---|---|---|
| `AdminUsersSection` | `{ token: string; role: SupportRole; currentUserId: string; signedInEmail?: string; onGrantChanged: () => void }` | `record: AdminUserRecord \| null`; `lookup: {state: "idle" \| "loading" \| "empty" \| "error", error?: unknown}`; `notice: string \| null` (the announcement text) |
| `AdminUserLookup` | `{ onSubmit: (email) => void; loading: boolean; error?: unknown; notFound?: boolean; onRetry?: () => void }` | `email: string`; `fieldError: string \| null` |
| `AdminUserRecord` | `{ record: AdminUserRecord; signedInEmail?: string; token: string; onChanged: () => void }` | per-form state (below); no record state of its own — the section owns it |
| `RoleChangeForm` | `{ record; signedInEmail?; token; onChanged }` | `newRole: ConsoleRole`; `reason: string`; `note: string \| null` |
| `StatusChangePanel` | `{ record; token; onChanged }` | `reason: string`; `confirming: boolean`; `note: string \| null` |
| `RoleBadge` | `{ role: ConsoleRole }` | none — pure |

The shell owns the session and passes `currentUserId` and `signedInEmail` in, plus
`onGrantChanged` so an approve or revoke keeps the shell's active-grant banner and clock accurate.
The grants the panel lists come from `usePendingGrants` (A5′), not from the shell's session-scoped
array — a fresh admin session must see the pending requests it did not create.
`AdminUsersSection` passes `signedInEmail` down only so `RoleChangeForm` can show the self-account
warning (§3.3 of the spec); it is a string comparison against the record's email, presented as one,
and the console never claims an identity it cannot establish.

### 2.3 Rendering rules shared with the console

- The record view renders `displayName ?? "—"` and `email ?? "—"`; never `0` or `$0.00`
  (design-system rule), and no financial field exists on this surface.
- `RoleBadge` and `StatusBadge` render a **label** with a **shape** and a tone, so neither reads by
  colour alone. `StatusBadge` exists in this package; the mapping from the five `AdminUserStatus`
  values to label, tone and shape is defined in §5.
- No component renders `ProblemDetails.detail`. Status codes and the correlation id reach the UI.
- The section has a visible `h2` ("Admin users") and is a labelled `<section aria-labelledby>`.

## 3. Data flow and hooks

### 3.1 `src/api/client.ts` additions

The client module owns every URL (the console's one-call-module rule). It gains five functions and
no other file constructs a U10 URL:

```ts
// existing shape: request<T>(path, init) adds the bearer token and x-correlation-id
export function lookupAdminUser(token: string, email: string): Promise<AdminUserRecord>;
//   POST /admin/users/lookup  body {email}   (D-S1: the address never reaches a URL)

export function changeUserRole(token: string, userId: string, body: RoleChangeBody)
  : Promise<RoleChangeResult>;
//   PUT  /admin/users/<userId>/role

export function disableUser(token: string, userId: string, body: AdminStatusBody)
  : Promise<AdminStatusResult>;
//   POST /admin/users/<userId>/disable

export function restoreUser(token: string, userId: string, body: AdminStatusBody)
  : Promise<AdminStatusResult>;
//   POST /admin/users/<userId>/restore

export function listPendingGrants(token: string): Promise<SupportAccessGrant[]>;
//   GET  /support-grants?status=pending_approval     (W5)
```

The reasons travel in the JSON body, never in the URL or a header. `ApiError` keeps its existing
shape (status, problem, correlation id); `NetworkError` and `GrantScopeError` are unchanged and not
raised by these paths. The unused `getSupportGrant` by-id call is removed: no such route exists in
the gateway table, in C5 or in the Identity router.

### 3.2 Hooks

| Hook | Key | Behaviour |
|---|---|---|
| `useAdminLookup` | `["admin", "user", email]` | one-shot query, `enabled` only after a submit, `staleTime: 0`, `retry: false` — an admin lookup is an audited action, never auto-retried |
| `useChangeRole` | mutation | on success: `onChanged(result)` then re-read `["admin","user",email]` via `refetch()`; on `503`: re-read before reporting |
| `useSetUserStatus` | mutation | same shape, calling `disableUser` or `restoreUser` by the record's current status |
| `usePendingGrants` | `["admin","pending-grants"]` | `GET` through `listPendingGrants`, `staleTime: 0`, refetched after an approve or revoke; a failure renders an inline error with a retry, never an empty list |

The mutations do **not** seed the cache optimistically and do **not** write the record themselves.
NFR24 says a change takes effect at the target's next request; the console shows only what the
server answers, which is why every success path re-reads the record and every ambiguous failure
(`503`) re-reads before it renders anything. Because W1 reads the role from the provider, the
re-read after a role change returns the value the provider now holds.

The lookup query is not kept after the section is left (`queryClient.removeQueries(["admin"])` on
unmount) so a previously looked-up address is not left in the console's cache.

### 3.3 Shell wiring

`SupportShell` gains the section to its `SECTIONS` list and its render switch, and **widens its
`visibleSection` guard**: today it is hard-coded to `section === "incidents" && role !== "admin" ?
"grants" : section`, so the new section does not inherit the fallback until the guard reads
`adminOnly` from `SECTIONS` instead of naming `incidents`. The approvals panel gets `token`,
`currentUserId` and `role` from the shell and calls the shell's `onGrantChanged` after an approve or
revoke, exactly as the `grants` section does — one banner, one clock — while its list comes from the
pending-grant route (W5). It also receives `signedInEmail` from the session for the self-account
warning.

## 4. Form rules and message text

Client checks are presentation only; the server re-checks every rule and its answer wins. The
console renders English strings today (it has no i18n module — spec §12, open item 5), so the
message text below is what the component renders; `data-testid` ids are stable for tests.

| Form | Rule | Message | testid |
|---|---|---|---|
| Lookup | email non-empty | "Enter the user's email address." | `admin-lookup-email-error` |
| Lookup | email syntactically valid | "Enter a valid email address." | `admin-lookup-email-error` |
| Role change | role differs from current | "Choose a different role." | `admin-role-error` |
| Role change | reason non-empty after trim | "Enter a reason of up to 500 characters." | `admin-role-reason-error` |
| Role change | reason ≤ 500 | as above | `admin-role-reason-error` |
| Status change | reason non-empty after trim | "Enter a reason of up to 500 characters." | `admin-status-reason-error` |
| Status change | reason ≤ 500 | as above | `admin-status-reason-error` |
| Both | submit disabled while a mutation is in flight | — | — |

Server `422` answers map `errors[].field` to the same slots (`email`, `newRole`, `reason`); any
error without a known field renders once in the panel's error line with the correlation id.

The three statements the unit must make to the administrator are fixed copy: **"This change is
audited."** after any successful change; the disable confirmation's "The user cannot sign in while
the account is disabled. History is not deleted."; and the restore confirmation's "The user can
sign in again at their next request."

## 5. Accessibility and design-system conformance

- **Labels and errors:** every control has a visible `<label htmlFor>`; each error paragraph has an
  id and the control points at it with `aria-describedby` (the pattern the preferences panel
  already uses).
- **Announcements:** a completed change (role, disable, restore) is written to a
  `<p role="status" aria-live="polite">` so a screen reader hears it without a focus move; a
  failure is a `role="alert"`.
- **Confirmation step:** disable/restore replaces the action buttons with a confirmation block;
  focus moves to **Confirm**, Escape cancels, Tab stays inside the block while it is open, and on
  cancel or completion focus returns to the action that opened it. There is no shared modal
  component in this package, so the block is inline — get the focus behaviour right rather than
  introducing a modal.
- **Badges:** `RoleBadge` and `StatusBadge` render text plus a shape; colour is never the only
  carrier. `RoleBadge` gets the role word in the accessible name.

### 5.1 Status presentation

`StatusBadge` takes a label, a tone and a shape; the console has no shared account-status mapping,
so U10 defines this one for the five values the admin answer reports. The label is always rendered,
which is what lets a shape repeat without colour carrying meaning.

| Status | Label | Tone | Shape |
|---|---|---|---|
| `pending` | Pending | neutral | ring |
| `active` | Active | success | dot |
| `disabled` | Disabled | warn | square |
| `pending_deletion` | Pending deletion | danger | diamond |
| `deleted` | Deleted | neutral | square |

`RoleBadge` uses the same vocabulary: `owner` — neutral, ring; `admin` — success, dot; `support` —
warn, diamond.
- **Contrast and focus:** the console's tokens (`--ink`, `--ink-2`, `--accent`, `--danger`,
  `--success`, `--warn`) carry the existing contrast; focus rings stay visible on every control,
  including the confirm block.
- **Localisation:** dates use `Intl.DateTimeFormat` in the console's locale; numbers do not appear
  on this surface. (NFR26 has no console implementation yet; spec §12, open item 5.)
- **Class tokens only:** `inputClass`, `labelClass`, `cardClass`, `sectionTitleClass`,
  `overlineClass`, `primaryButtonClass`, `secondaryButtonClass`, `dangerButtonClass`,
  `fieldErrorClass`, `alertClass`, `calloutClass`, `linkClass` from `src/components/ui.ts`.
  `dangerButtonClass` is used only for **Disable**, never for **Restore** or a role change.

## 6. Testing surface

Floor per suite: one happy path and at least two error or edge cases, with `jest-axe` on every
rendered state (the team's construction rule and the console's existing test style). MSW mocks the
gateway; no test reaches a service.

| Suite | Cases |
|---|---|
| `AdminUsersSection.test.tsx` | section renders for `admin`, is absent from nav for `support`; `#admin-users` hash for a support session falls back; record load then a role change re-reads the record |
| `AdminUserLookup.test.tsx` | happy: exact email answers a record; `404` renders "No user has that address." and shows no record; `403` renders the not-an-administrator line with the correlation id; `503` renders "This user cannot be looked up right now." and no record; empty and malformed input are refused before any request |
| `AdminUserRecord.test.tsx` | renders name, email + Verified/Unverified, id, role badge, status badge, timestamps; renders an em dash for a missing name; renders no entitlement control and no financial field; jest-axe clean |
| `RoleChangeForm.test.tsx` | happy: `200` renders "admin → support" and "This change is audited." then re-reads; `422` on `newRole` and on `reason` render per-field; `403` renders the session line; `503` re-reads and renders the server's role without claiming failure; submit disabled when the role is unchanged; self-email warning appears only for the signed-in address |
| `StatusChangePanel.test.tsx` | disable offers confirmation, nothing is sent until Confirm; cancel sends nothing; `200` renders the new status and the audited line; `409` re-reads and renders the server's status; restore is offered only for `disabled`; `pending_deletion` and `deleted` show the explanatory line and no action; Escape cancels and focus returns |
| `RoleBadge.test.tsx` | renders each of the three roles by label and shape; never colour alone; accessible name carries the role |
| `client.admin.test.ts` | the five calls build the documented method, path, body and headers; the reason never appears in the URL; a `403` and a `503` surface as `ApiError` with the correlation id; the approvals list sends `status=pending_approval` |
| `AdminApprovals.test.tsx` | a fresh admin session renders the grants the list answers, not an empty panel; a `403` renders the not-an-administrator line; a list failure renders an inline error and a retry rather than an empty state; an approve or revoke refetches the list and calls `onGrantChanged` |
| `GrantApprovalQueue` | unchanged; its existing suite already covers the self-approval disabled state and the `403` — U10 relies on it and adds no duplicate |

## 7. Files this unit expects to create or change

| Path | Change |
|---|---|
| `support-console/src/screens/admin/AdminUsersSection.tsx` | new |
| `support-console/src/screens/admin/AdminUserLookup.tsx` | new |
| `support-console/src/screens/admin/AdminUserRecord.tsx` | new |
| `support-console/src/screens/admin/RoleChangeForm.tsx` | new |
| `support-console/src/screens/admin/StatusChangePanel.tsx` | new |
| `support-console/src/components/RoleBadge.tsx` | new (shared) |
| `support-console/src/api/client.ts` | five functions added; `getSupportGrant` kept, unused, and not called by U10 (no by-id grant route exists) |
| `support-console/src/api/types.ts` | the admin types added (`ConsoleRole`, `AdminUserStatus`, `AdminUserRecord`, the request and answer bodies) |
| `support-console/src/screens/SupportShell.tsx` | section registered and rendered; `visibleSection` guard widened to read `adminOnly` from `SECTIONS` |
| the matching `*.test.tsx` files | new, per §6 |

No file is deleted; no dependency, index, migration or threshold is added by this unit.