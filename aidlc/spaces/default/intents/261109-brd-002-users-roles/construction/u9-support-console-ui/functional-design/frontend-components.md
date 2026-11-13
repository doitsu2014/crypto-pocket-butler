# Frontend Components — U9 Support Console UI

**Unit:** `u9-support-console-ui` · **Stage:** Functional Design · **Package:**
`support-console/` · **Spec:** `functional-spec.md` (this file is its component contract)

## 1. Component hierarchy

```
SupportApp
└─ SupportSignInGate            (existing; role = support | admin, BR1.1)
   └─ SupportShell              (existing; owns the session's own grant)
      ├─ GrantBanner            (reworked; renders above every section)
      ├─ #grants                (support only)
      │  └─ GrantRequestForm    (reworked)
      ├─ #metadata              (renamed from #search; #search redirects)
      │  └─ SupportMetadataPanel (new; replaces AccountSearch + AccountMetadataPanel)
      ├─ #tickets               (existing TicketQueue — unchanged by this unit)
      ├─ #incidents             (existing IncidentConsole — admin only)
      ├─ #admin-users           (U10: AdminUsersSection → AdminApprovals)
      └─ #help                  (existing HelpPanel)
```

- The shell keeps **one** grant: the signed-in session's own. Admin grant approvals live in
  `AdminUsersSection` (U10) and update the shell only when the approved grant is the session's own
  (§3.3).
- `AccountSearch`, `AccountMetadataPanel`, `useSupportGrant`, and the client functions
  `searchAccounts`, `fetchAccountMetadata`, `getSupportGrant` are removed, with their tests.
- `GrantApprovalQueue` stays a component but is mounted only by `AdminApprovals`; the `grants`
  section no longer imports it.

## 2. Interfaces and state

### 2.1 Types (`src/api/types.ts`)

```ts
export type GrantStatus = "pending_approval" | "active" | "expired" | "revoked";

export interface SupportAccessGrant {
  id: string; userId: string; requestedBy?: string | null; reason: string;
  requestedAt: string; expiresAt: string | null; status: GrantStatus;
  approvedBy: string | null; approvedAt: string | null;
}

export interface GrantRequestInput { userId: string; reason: string; }

export interface SupportMetadata {
  userId: string;
  displayName: string | null;
  emailVerified: boolean;
  accounts: { id: string; label: string; sourceType: string;
              syncStatus: string; lastSyncedAt: string | null }[];
  portfolios: { id: string; label: string }[];
}
```

Removed types: `AccountMetadata`, `AccountSearchResult`, `MetadataReadInput`, `AccountSearchInput`,
`GrantRequestValues`. `GrantRequestValues` is folded into the form's local state; the domain
validator takes `{ targetUserId, reason }`.

### 2.2 Components

```ts
interface GrantRequestFormProps {
  token: string | null;
  onCreated: (grant: SupportAccessGrant) => void;
}
interface GrantBannerProps {
  grant: SupportAccessGrant;
  now?: number;
  onRequestNew: () => void;
  onCheckStatus?: () => void;
  checking?: boolean;
}
interface SupportMetadataPanelProps {
  grant: SupportAccessGrant;      // the session's active grant
  token: string | null;
  onAccessEnded: () => void;
}
```

- **`GrantRequestForm`** owns `targetUserId`, `reason`, field errors, and the request mutation.
  It renders no duration and no approver input. On `201` it clears both fields, calls
  `onCreated`, and shows the success line. `409` offers **Check your current grant** through an
  `onCheckStatus` callback supplied by the shell (added to the props as
  `onCheckStatus?: () => void`).
- **`GrantBanner`** owns nothing but rendering; `now` is passed by the shell so the banner and the
  rest of the shell share the same clock tick. It receives the session's grant only.
- **`SupportMetadataPanel`** owns the filter text and nothing else; the projection is server state
  in the query cache. It reads only while `grant.status === "active"` and the local clock says the
  window is open; otherwise it renders the not-active callout and issues no request.
- **`GrantApprovalQueue`** gains one rule: **Approve** is disabled when `role !== "admin"`, with
  the line "Only an administrator can approve a grant." Revoke already renders for admins only.

### 2.3 Rendering rules shared with the console

- Missing values are an em dash; an empty list is "No accounts." / "No portfolios."; a missing
  display name or approver is an em dash.
- Status is always `StatusBadge` — label plus tone plus shape, never colour alone.
- No component renders ProblemDetails `detail` text; the status and correlation id are shown.
- Every list and table has a heading; every input has a visible label.
- Only Parchment Ledger tokens and the `ui.ts` class helpers; no raw value anywhere.

## 3. Data flow and hooks

### 3.1 `src/api/client.ts` changes

| Function | Change |
|---|---|
| `requestSupportGrant(input, token)` | body is `{userId, reason}` only (W1) |
| `listMyGrants(token)` | **new** — `GET /support-grants/mine`, deadline 15 s, maps each row with `pickSupportGrant` |
| `fetchSupportMetadata(grantId, token)` | **new** — `GET /support-grants/{grantId}/metadata`, deadline 15 s, maps with `pickSupportMetadata` |
| `approveSupportGrant`, `revokeSupportGrant`, `listPendingGrants` | unchanged |
| `getSupportGrant`, `searchAccounts`, `fetchAccountMetadata` | **removed** (W3, W7) |

- `pickSupportGrant` accepts `grantId` or `id` as the identifier (W2) and validates the four
  statuses it models (W5).
- `pickSupportMetadata(raw)` narrows the projection to `SUPPORT_METADATA_FIELDS` and throws on a
  malformed projection rather than rendering a partial one.
- No call adds a URL, header, or body field the contract does not declare.

### 3.2 Hooks

| Hook | Query key | Behaviour |
|---|---|---|
| `useMyGrants(token)` | `["support", "grants", "mine"]` | `staleTime: 0`, `retry: false`; disabled without a token; a 401/403 clears the support cache |
| `useSupportMetadata(grant, token, clock)` | `["support", "metadata", grant.id]` | enabled only when the local status is `active`; `staleTime: 10_000`, `retry: false`; a 403 calls `onAccessEnded` and clears the cache |
| `useGrantClock(expiresAt)` | — | existing; 30-second tick plus an expiry timeout |
| `useGrantAccess(grant)` | — | existing; the clock-derived status the shell and panel share |

### 3.3 Shell wiring (`src/screens/SupportShell.tsx`)

- On mount, a support or admin session reads its own grants once (`useMyGrants`) and selects the
  newest one whose `requestedBy` matches the signed-in user (a just-created grant is adopted
  directly from the request answer). Nothing is persisted.
- `onGrantCreated(grant)` sets the session's grant and shows the pending banner.
- `onCheckStatus()` re-reads `useMyGrants`; the banner moves to its server state.
- `onGrantChanged(grant)` (from the admin section) updates the pending list and sets the banner
  **only** when the changed grant belongs to the session (`grant.requestedBy === session.user.id`
  or it was the session's own created grant). An admin approving another agent's grant never
  becomes the admin's banner.
- `onAccessEnded()` marks the local grant `expired` and clears the support query cache; the
  metadata panel and the banner both read that state.
- Section map: `search` is removed from `SECTIONS`; `metadata` is added; `readSectionFromHash`
  maps a `search` hash to `metadata` so old links still land. The `grants` section renders
  `GrantRequestForm` only for `role === "support"`, and for `role === "admin"` a line linking to
  `#admin-users` instead of the queue.
- Sign-out clears the cache and the session as it does today.

## 4. Form rules and message text

| Field | Rule | Message |
|---|---|---|
| Target user id | required | "Enter the target user's id." |
| Target user id | server `400` | "Enter a valid user id." |
| Reason | required after trimming | "Enter a reason." |
| Reason | ≤ 500 characters | "Use 500 characters or fewer." (client) / per-field `422` |
| Reason | counter | `{count}/500`, not a live region |

Screen messages:

- Success: "Requested. A different administrator must approve it within an hour; the grant then
  lasts at most 60 minutes."
- Conflict: "You already have a live grant for this user." + **Check your current grant**
- Missing user: "No user has that id."
- Wrong role: "Your session is not a support agent." + correlation id
- Audit unavailable: "The request could not be recorded right now." + correlation id
- Not active: "Metadata reads open once a different administrator approves your grant."
- Access ended: "Access ended. The grant expired, was revoked, or was never approved." +
  **Request a new grant**
- Metadata unavailable: "The metadata cannot be read right now." + correlation id + **Refresh**
- Read audit statement: "This read is recorded in the audit log."

## 5. Accessibility and design-system conformance

- **WCAG 2.2 AA (NFR25):** labelled regions and headings; `aria-describedby` ties each error to
  its field; `role="status"` for the request result, status changes and the read completion;
  `role="alert"` for access-ended and failures; the accounts table has `<caption>` and
  `scope="col"` headers; the portfolio list is a `<ul>`; the filter is a labelled input; focus is
  visible and order is logical; the banner's remaining time is expressed in text; `jest-axe`
  passes on every touched surface.
- **Tokens only:** colour comes from `--success`, `--warn`, `--danger`, `--accent`, `--ink`,
  `--ink-2`, `--muted`, `--surface`, `--surface-warm`, `--border`, `--border-soft` through
  `ui.ts` classes; no raw value.
- **Missing values:** em dash, never `0` or `$0.00`; dates via `Intl`.

### 5.1 Status presentation

| Status | Label | Tone | Shape | Reads |
|---|---|---|---|---|
| `pending_approval` | Pending approval | warn | diamond | disabled |
| `active` | Active | success | dot | enabled |
| `expired` | Expired | danger | square | disabled |
| `revoked` | Revoked | danger | ring | disabled |

The mapping stays in `src/domain/grants.ts` (`grantStatusPresentation`) and is shared by the
banner, the admin queue and any badge on the admin side.

## 6. Testing surface

| Suite | What it must cover |
|---|---|
| `src/api/client.support.test.ts` (new/extended) | request body is exactly `{userId, reason}`; `grantId` key mapping; `listMyGrants` path and empty list; `fetchSupportMetadata` path, projection whitelist, a malformed projection rejecting; the retired calls are gone |
| `src/domain/grants.test.ts` | 1–500 bound; clock-derived expiry; the four-status presentation; fail-closed on an unparseable `expiresAt` |
| `src/domain/metadata.test.ts` | `pickSupportMetadata` keeps exactly the whitelist; missing display name → null; over-sent fields dropped |
| `src/components/GrantRequestForm.test.tsx` | happy path; empty id; empty reason; 501-character reason; `409` → check-current; `404`; `403`; `422` per-field |
| `src/components/GrantBanner.test.tsx` | four status copy variants; remaining-minutes text; expired clock; the metadata-only reminder; check-status and request-new callbacks |
| `src/components/SupportMetadataPanel.test.tsx` | happy path renders user, accounts, portfolios; empty lists; filter narrows without a request; 403 → access ended and `onAccessEnded`; 401; 503 + refresh; no read while inactive |
| `src/hooks/useMyGrants.test.tsx` / `useSupportMetadata.test.tsx` | disabled state, success, 403 cache clear |
| `src/screens/SupportShell.test.tsx` | mount restores the session's own grant; admin approval of another grant does not take the banner; section rename and `#search` alias; support never sees approvals; admin sees the admin link instead of the form |
| `src/components/GrantApprovalQueue.test.tsx` | approve disabled for non-admin; approve enabled for a different admin; self-approval copy |

Every suite covers the happy path and at least two error or edge cases (construction phase rule).
The MSW-backed tests stand in for the gateway; no test asserts on a route that is not in §8 of the
spec.

## 7. Files this unit expects to create or change

**Console (`support-console/`)**

- `src/api/types.ts` — types in §2.1; remove the retired shapes
- `src/api/client.ts` — functions in §3.1
- `src/api/client.support.test.ts` — **new**
- `src/api/client.test.ts` — remove the retired-call cases
- `src/api/queryClient.ts` — comment only (the cache policy is unchanged)
- `src/domain/grants.ts` / `grants.test.ts` — 1–500 bound, remove the 20/480 constants
- `src/domain/metadata.ts` / `metadata.test.ts` — metadata whitelist and picker
- `src/components/GrantRequestForm.tsx` / `.test.tsx` — reworked
- `src/components/GrantBanner.tsx` / `.test.tsx` — reworked
- `src/components/SupportMetadataPanel.tsx` / `.test.tsx` — **new**
- `src/components/AccountSearch.tsx` / `.test.tsx` — **removed**
- `src/components/AccountMetadataPanel.tsx` / `.test.tsx` — **removed**
- `src/components/GrantApprovalQueue.tsx` / `.test.tsx` — non-admin disabled state
- `src/hooks/useMyGrants.ts` / `.test.tsx` — **new**
- `src/hooks/useSupportMetadata.ts` / `.test.tsx` — **new**
- `src/hooks/useSupportGrant.ts` / `.test.tsx` — **removed**
- `src/screens/SupportShell.tsx` / `.test.tsx` — wiring in §3.3

**Service and gateway (only the W4 read)**

- `services/identity/application/src/support.rs` — a read-only `mine` use case over U3's store
- `services/identity/application/src/ports.rs` — the store read if it does not exist
- `services/identity/http/src/{routes.rs,handlers.rs,dto.rs}` — `GET /v1/support-grants/mine`
- `services/identity/tests/http.rs` — the route's happy path and role refusal
- `services/gateway/domain/src/route.rs` — the declared route and its budget
- `services/gateway/tests/{domain.rs,http.rs}` — the route's declaration and forwarding

**Docs**

- `inception/contract-design/contract-summary.md` — C1/C2 and C5 entries for S2 and the `grantId`
  key (landed by the code-generation change, per §12 item 1)
- `docs/architecture/README.md` and `docs/architecture/workspace.dsl` — only if a container or
  relationship changes; route additions alone do not change the C4 model