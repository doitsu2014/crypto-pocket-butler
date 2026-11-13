# Functional Spec — U7 Settings UI (`u7-settings-ui`)

**Stage:** Functional Design · **Unit kind:** ui (embedded in the SPA build, `web/`) · **Complexity:** M
**Stories:** US1.4, US4.3, US6.1 · **Requirements:** FR14.4, FR17.3, FR19.1, AC4.3.3
**Upstream:** `unit-of-work.md` (U7), `unit-of-work-story-map.md`, `stories.md`, `requirements.md`,
`components.md` (`AccountSettingsUI`), `contract-summary.md` (C1/C2, C3, C6, C7), `mockups.md` §1
and §1a, `interaction-spec.md`, `design-system-mapping.md`, `accessibility-checklist.md`,
u4/u5 functional specs (the services this unit presents)
**Decisions on record:** Q1 — this unit adds the missing owner-scoped routes and their gateway
entries inside Identity, with no new service, transport, or table; Q2 — display name editable with
a provider-name reset; Q3 — email and push switches only; Q4 — analytics-only consent control;
Q5 — a dedicated deletion page without a reason field; Q6 — the recovery banner shows the exact
date read from the open request; Q7 — the two existing pages are rewired in place. (See
`functional-design-questions.md`.)

## 1. Purpose

U7 is the person's own account surface. It shows and lets the person change the profile
(display name), the preferences (locale, base currency, timezone, the two notification channels),
and the analytics consent; it shows the terms, risk, and age acceptances read-only with the date
each was recorded; it offers the deletion request on its own page; and it hosts the recovery
screen a pending-deletion person returns to and the terminal screen a disabled or deleted
account sees. This spec is self-contained: it is the authoritative description of U7's screens,
their states, and the record rules the new endpoints apply.

### What this unit does not own

- **Recording preferences and consent.** `u4-preferences-consent` owns the preference set, the
  defaults, and the consent state machine (contract C6). U7 writes through `/v1/me/preferences`
  and `/v1/me/consent*` and never re-derives a default or a consent decision in the browser.
- **The deletion state machine.** `u5-account-lifecycle` owns `requested → cancelled |
  erased`, the 30-day window, the erasure job, and the status transitions (contract C7). U7
  opens and cancels a request and renders the dates it is given.
- **Authorisation of a pending-deletion account.** AC4.3.3's refusal is already enforced:
  `UserStatus::denies_use` (BR1.7) covers `disabled`, `pending_deletion`, and `deleted`, and
  `PrincipalService` answers 403 for those statuses (a distinct `identity.principal.erased` audit
  for `deleted`). U7 adds no enforcement and no relaxation. What it adds is the two reads a
  denied account must still have — its own status (D0) and its open deletion request (D1/D2) —
  resolved from the stored record by auth subject the way the existing deletion handlers resolve
  their caller, so the recovery path works without weakening the predicate.
- **Provider provisioning.** Populating the profile from the provider claim is U1's
  `identity.principal.provisioned` path. U7 only marks a field overridden and clears the
  override (§4).
- **Device sessions.** The SPA shows a session list today and calls a route the server does not
  serve. Sessions appear in no U7 story and no gateway route exists, so the panel is removed
  with its dead client call (§12, open item 8); it returns with the unit that adds a sessions
  route.
- **Deleting the account.** U7 requests deletion; the erasure itself runs in
  `identity-worker` (U5).

## 2. Journey and screen flow

The SPA shell signs the person in through Supabase Auth (unchanged). Entry routing resolves the
principal and sends the person to one of four U7 surfaces or back into the app (Q6, Q7):

```mermaid
flowchart TD
  A[Signed-in entry] --> B{Account status (D0)}
  B -->|active| C[App screens]
  C --> D[/settings/profile]
  D --> E[/settings/preferences]
  D --> F[/settings/consent]
  D --> G[/settings/privacy]
  G --> H[/settings/delete]
  B -->|pending_deletion| I[/account/restore]
  B -->|disabled| J[/account/blocked — disabled]
  B -->|deleted| K[/account/blocked — deleted]
  I -->|Restore succeeds| D
  I -->|window closed — 410| K
  H -->|request accepted — 202| I
```

Text fallback: signed-in entry → principal status → {active: the app, and from it
`/settings/{profile,preferences,consent,privacy}` → `/settings/delete` | `pending_deletion`:
`/account/restore` | `disabled` or `deleted`: `/account/blocked`}; the restore screen returns to
`/settings/profile` on success and to the deleted screen on `410`; a deletion request lands on the
restore screen.

### 2.1 Entry routing

The SPA resolves the signed-in person before rendering a gated page, but the existing principal
read refuses exactly the three statuses this unit routes on: `PrincipalService` returns 403 for
`disabled`, `pending_deletion`, and `deleted` (BR1.7). A denied person therefore cannot learn
why, and no destination table can be reached through that read. U7 adds **D0,
`GET /v1/me/account-status`** — a non-denying self-read that returns only the caller's own
status, resolved from the stored record by auth subject (the bypass `caller_for_deletion` already
uses in the deletion handlers) — and routes on it:

| Account status (D0) | Destination | Why |
|---|---|---|
| `active` | the app; `/settings/*` and `/privacy` are reachable | — |
| `pending_deletion` | `/account/restore` | AC4.3.1 — the person must be able to restore; a terminal screen would strand them |
| `disabled` | `/account/blocked` (disabled variant) | no self-service path exists; support is named |
| `deleted` | `/account/blocked` (deleted variant) | terminal |

Rows are evaluated before any gate screen, and the pending-deletion row is what refines U8's
table: U8 sends `status: pending_deletion` to "U7's blocked-account screen for pending deletion";
U7 splits that hand-off so a pending-deletion person lands on the **restore** surface, which
carries the recovery banner, and only a disabled or deleted account reaches the terminal screen.
D0 answers for `active` and `pending` too, so one read decides the whole table; those two statuses
then continue into the existing gate flow unchanged.

`nextStep: restore_account` is not used: `identity_domain::NextStep` has no such variant and the
server never reports one. Routing keys on status alone, which already distinguishes the three
states. `deriveGateStatus` keeps its dormant branch for forward compatibility and removes the
local `DestinationUnavailable` fallback for the two U7 destinations now that both routes exist.

The SPA's existing gate hook calls `GET /v1/me/principal`, a path no service or gateway entry
serves; the served principal read is `GET /v1/principals/{authUserId}`. That mismatch is U8/U1's
follow-up, not this unit's: U7 routes on D0 and leaves the gate hook's own read alone (§12,
open item 5).

### 2.2 Settings shell

`/settings` is one shell with a left navigation of four destinations, as in `mockups.md` §1:

| Tab | Route | Contents |
|---|---|---|
| Profile | `/settings/profile` | display name, email, account status |
| Preferences | `/settings/preferences` | locale, base currency, timezone, notification channels |
| Consent | `/settings/consent` | analytics switch (editable) + terms/risk/age (read-only) |
| Privacy | `/settings/privacy` | data export, consent history, the Danger zone link to `/settings/delete` |

`/settings` redirects to `/settings/profile`. `/privacy` redirects to `/settings/privacy`, so an
existing bookmark keeps working (Q7). The Sessions tab in the mockup is not built (§1).

## 3. Screen rules

### 3.1 Profile

Reads `GET /v1/me/profile` (§8 P1). Content, in reading order:

1. **Display name** — a text input pre-filled with the effective value. Save writes
   `PATCH /v1/me/profile {"displayName": "<value>"}` (P2).
2. **Use my provider name again** — a secondary action, rendered only when
   `displayNameOverridden` is `true`, and disabled with an explanatory hint when
   `providerDisplayName` is `null`. It writes `PATCH /v1/me/profile {"useProviderName": true}`.
3. **Email** — read-only, with a `Verified` badge from `emailVerified`. Changing an email is an
   authentication flow, not a settings form; the field has no input.
4. **Account status** — the `StatusPill` for `status` (a pending-deletion or deleted account does
   not reach this page, but a disabled one may be re-enabled by support while the page is open,
   so the pill is rendered from the read, not hardcoded).

Save feedback is inline and stays on the page (§6 feedback convention in `interaction-spec.md`:
no immediate redirect). A successful save re-reads P1 and re-renders from the server; the input
is never the source of truth after a save.

### 3.2 Preferences

Reads `GET /v1/me/preferences` (existing, both sides). One Save writes the whole panel through
`PUT /v1/me/preferences` with only the fields that changed (the route is a partial update; the
server merges). Fields:

| Field | Control | Value set | Failure |
|---|---|---|---|
| Locale | select | `en`, `vi` (the server's closed set) | 422 `locale` |
| Base currency | select | `USD`, `EUR`, `GBP`, `VND` (the server's closed set) | 422 `baseCurrency` |
| Timezone | searchable select over `Intl.supportedValuesOf("timeZone")` | any IANA zone the browser knows | 422 `timezone` |
| Email notifications | `role="switch"` | boolean | 422 `notifications.email` |
| Push notifications | `role="switch"` | boolean | 422 `notifications.push` |

Each field renders the server's `source` (`user` or `default`) as a quiet subordinate label, so a
person can tell a value they chose from one the service defaulted (AC1.4.2's observable effect).
The panel offers no third channel and no per-topic toggle: the service stores `email` and `push`
only, and a topic the service cannot store is not drawn (Q3).

BCP-47 interaction: changing locale re-renders the SPA's strings; the base currency changes only
labels, never a stored financial value (AC6.1.1's "stored financial values are unchanged" is
U4's rule; U7 sends the setting and nothing else).

### 3.3 Consent

Reads `GET /v1/me/consent/state` and `GET /v1/me/consent` (history). Two groups:

- **Required acceptances — read-only.** `terms`, `risk`, and `age`, each with its state from
  `kinds` and the date of its latest `granted` event from the history. Turning one off here is
  not offered: they gate source connections and are re-accepted in that flow (Q4).
- **Analytics — one switch.** `role="switch"`, `aria-checked` from `kinds.analytics`.
  - On → `POST /v1/me/consent {"kind":"analytics","policyVersion":"<ANALYTICS_POLICY_VERSION>"}`.
  - Off → `POST /v1/me/consent/analytics/withdraw`.
  - Under the switch, the withdrawal date (`Withdrawn <date>`) read from the latest `withdrawn`
    event for analytics in the history — the value the mockup shows.
  - After any mutation the panel invalidates both queries and re-renders from the server; it
    never optimistically flips the switch. The server's answers are: a grant is `201`; a
    withdrawal is `200` with `changed: true` when it was granted and `changed: false` when it was
    already withdrawn (a recorded no-op, not an error); `409` only when the kind was never
    granted. The switch therefore renders `changed: false` as success plus a hint that nothing
    needed changing, and re-reads on `409` rather than showing a failure.

`ANALYTICS_POLICY_VERSION` is a constant held by the settings feature module, matching the
analytics notice the panel links to. The server accepts any trimmed string of 1–120 characters
and validates no version against a published policy (§12, open item 3).

### 3.4 Privacy

Reads the same consent history as §3.3 plus the export flow that already exists (`POST /v1/exports`
and its poll — unchanged by this unit). Contents: the consent history table, the export control,
and a **Danger zone** panel whose only control links to `/settings/delete`. Deletion is no longer
initiated by a dialog on this page (Q5); the exported and history behaviour is preserved from the
current page (Q7 — rewired, not rewritten).

### 3.5 Delete account (`/settings/delete`)

A dedicated page matching `mockups.md` §1a (Q5), reached from the Danger zone. It has two states:

- **No open request.** Three sections: *What happens* (the request opens a 30-day recovery
  window; the account and its data are permanently erased after it; signing in during the window
  lets the person restore the account from the restore screen, and sign-in alone does not restore
  it), *What is kept* (see below), and the request control. The request control opens a
  confirmation dialog that restates the consequence and requires a second, explicit destructive
  confirmation; Cancel closes it without a call. Confirming calls
  `POST /v1/privacy/deletions` (D3, no body) and on `202` swaps the whole page into the
  "open request" state.
- **An open request exists** (`GET /v1/me/deletion-request` returns one, D1). The page shows the
  request date and the exact `recoveryEndsAt` date, a **Restore** primary action
  (`DELETE /v1/me/deletion-request`, D2), and no deletion form. This is the same content as the
  restore screen (§3.6); the page is reachable directly, so it renders it rather than redirecting.

**What is kept.** The service's `retainedCategories` list is **empty at request time** — the
categories are recorded while the account is erased — so the page renders the list only when it
is non-empty (an open request after the erasure began). Otherwise it renders one placeholder
sentence that names no category and claims nothing the policy has not settled:

> Some records may be kept after your account is deleted where the law requires it. The exact
> list appears here once your deletion begins.

That sentence is the build's copy, held in the i18n module under `settings.delete.retained`.
The authoritative list replaces it when the retention and legal-hold policy is approved
(BRD-012 still lists it as "must define before launch"); until then neither the placeholder nor
the page may name a category. The page never invents a list, and it never claims the service
returned categories it did not (§12, open item 2).

There is **no reason field**: the deletion call takes no body and nothing stores a reason (Q5).

### 3.6 Restore account (`/account/restore`)

The recovery surface for a `pending_deletion` account. It reads `GET /v1/me/deletion-request`
(D1) and renders a banner whose primary content is the exact date the account and its data are
erased, taken from `recoveryEndsAt` and formatted with `Intl.DateTimeFormat` in the person's
locale and the profile timezone (Q6). Actions: **Restore my account** (D2) and **Sign out**.

D1, D2 and D0 resolve their caller from the stored record by auth subject — the path the
existing deletion handlers already take — so the 403 that refuses a `pending_deletion` account
everywhere else does not refuse the three reads and the one write that make up its recovery.

- Restore `200` → invalidate the principal and deletion queries → route to `/settings/profile`.
- Restore `410` → the window closed: render the terminal "the recovery window closed on <date>"
  state with Sign out only, and no restore control.
- D1 `404` (no open request — another tab restored, or the request completed) → re-resolve the
  principal and route by status (§2.1) instead of rendering an empty banner.
- No other settings content is rendered on this route: a pending-deletion account is refused
  elsewhere by design (AC4.3.3), and offering the settings shell here would be a dead end.

### 3.7 Blocked account (`/account/blocked`)

Terminal, one screen, two variants chosen from `status`:

- **disabled** — "Your account is disabled." plus how to contact support, and Sign out. No
  self-service control: re-enabling is an administrator action (U5's `restore`).
- **deleted** — "This account has been deleted." plus Sign out. Nothing else: the record is
  anonymised and no request can reopen it.

A `pending_deletion` principal never renders this screen (§2.1).

## 4. Record rules this unit adds

These are the only record-level rules U7 introduces, all inside the Identity service, all
owner-scoped from the JWT and never from a path id (Q1):

- **W1 — Save a display name.** Trim; refuse empty or longer than 200 characters with 422 naming
  `displayName`; store the value; set `display_name_overridden = true`; append one audit event
  `identity.user.profile.updated` with `context { field: "displayName", overridden: "true" }`.
  The domain validation is `identity_domain::User::validate_display_name`, already in place.
- **W2 — Adopt the provider name again.** Requires a display name in the forwarded claim. Set
  the profile's display name to that claim value, clear `display_name_overridden`, append the
  same audit event with `overridden: "false"`. When the claim carries no display name, refuse
  409 and change nothing — clearing the flag alone would not change what the person sees, so the
  action must carry a name to adopt.
- **W3 — Read the open deletion request.** Owner-only; returns the open request (the status that
  is neither cancelled nor erased) or 404 `not-found`. Read-only: no audit event, no state
  change, and it never opens a request.
- **W4 — Cancel the open deletion request.** Delegates to U5's existing `cancel_deletion`
  (BR5.4): 200 with the cancelled request, 409 when no request is open, 410 after the deadline.

The read and the cancel are the two endpoints the recovery surface needs. Both derive the user
from the JWT, so neither can address another person's account.

## 5. Form and field rules

| Screen | Field | Rule | Failure shown |
|---|---|---|---|
| Profile | Display name | Required on save; 1–200 characters after trimming | "Enter a display name of up to 200 characters." |
| Preferences | Locale | One of `en`, `vi` | "Choose a supported language." |
| Preferences | Base currency | One of `USD`, `EUR`, `GBP`, `VND` | "Choose a supported currency." |
| Preferences | Timezone | A valid IANA zone | "Choose a supported timezone." |
| Preferences | Notifications | Booleans; no validation beyond a failed write | inline error + correlation id |
| Delete | Confirmation | The dialog's explicit confirm is the only gate; no typed phrase | — |

Client-side checks are presentation only; the server re-checks every one and its 422 field list
wins. A 422 renders per-field errors from the response's `errors[].field` and never the problem
`detail` text.

## 6. Accessibility and localisation

- **WCAG 2.2 AA (NFR25):** the settings navigation is a labelled `nav` whose links carry
  `aria-current="page"`; every control has a visible label; every switch is `role="switch"` with
  `aria-checked`; every error is tied to its field with `aria-describedby`; the save confirmation
  and the recovery banner are `role="status"` with `aria-live="polite"`; the delete confirmation
  is a modal dialog with a focus trap, Escape to cancel, and focus returned to the trigger; focus
  is visible and never colour-only.
- **Parchment Ledger tokens only** (`design-system/tokens.css`): `--accent`, `--surface`,
  `--border`, `--fg`, `--muted`, `--success`, `--warn`, `--danger`, `--text-xs` for the 10px
  overline badge label. No raw colour, spacing, or type value anywhere.
- **Missing values** render as an em dash, never `0` or `$0.00` (design-system rule). U7 shows no
  computed financial figure, so no tabular-mono rule applies beyond the em-dash rule for an
  absent date or name.
- **Status is never colour alone:** every `StatusPill` pairs its colour with a label and a dot
  shape.
- **English and Vietnamese (NFR26):** every string, date, and number goes through the existing
  i18n module and `Intl`; the locale comes from the profile preference once loaded, from the
  browser before that, English as the fallback.

## 7. Security posture

- Tokens stay in memory only; nothing is written to `localStorage` or `sessionStorage` — not the
  deletion request id, not the recovery date, not the analytics consent state. Every value is
  re-read from the server, which is why §3.5 and §3.6 need D1 at all.
- Every call goes through the API Gateway; Supabase Auth is the only other browser-external call.
  No business or compliance decision is computed in the frontend.
- Every new route is `/v1/me/*` and derives the user from the forwarded JWT. No U7 route accepts
  a user id in the path, so a person cannot address another account by editing a URL.
- The email field is read-only and the frontend never posts an email; email changes stay in the
  authentication flow.
- No screen renders ProblemDetails `detail` text; only status and correlation id reach the UI.
- The delete page states the consequence before the destructive confirmation and performs the
  request only after it — and the request is reversible for 30 days by design.

## 8. API integration points (gateway `/v1` surface)

All through `web/src/api/client.ts`. Existing routes U7 consumes, unchanged:

| Id | Call | Use |
|---|---|---|
| X1 | `GET /v1/principals/{authUserId}` | the served principal read. **Not used for U7 routing**: it answers 403 for `disabled`, `pending_deletion` and `deleted`, so U7 routes on D0 instead (§2.1). The SPA's gate hook calls `/v1/me/principal`, a path no service or gateway entry serves (§12, open item 5). |
| X2 | `GET|PUT /v1/me/preferences` | the preferences panel (§3.2) |
| X3 | `GET /v1/me/consent/state` | consent states (§3.3) |
| X4 | `GET /v1/me/consent` | consent history and withdrawal dates (§3.3, §3.4) |
| X5 | `POST /v1/me/consent` | analytics grant (§3.3) — `201` |
| X6 | `POST /v1/me/consent/{kind}/withdraw` | analytics withdrawal (§3.3) — `200`, `changed` true or false |
| X7 | `POST /v1/exports` and `GET /v1/exports/{exportId}` | data export (§3.4). Served by Identity, but **no gateway entry exists**, so the panel is unreachable from the browser until this unit adds both (below). |

Routes this unit adds (Q1 — Identity only, plus its gateway entries):

| Id | Call | Behaviour |
|---|---|---|
| D0 | `GET /v1/me/account-status` | `200` `{ status }` for the signed-in person, resolved by auth subject **without** the denial predicate. Read-only, audited only for a denied status? No — a plain read, no audit event. |
| P1 | `GET /v1/me/profile` | `200` `{ userId, displayName, displayNameOverridden, providerDisplayName, email, emailVerified, status }` · `401` · `503` |
| P2 | `PATCH /v1/me/profile` | body `{"displayName":"…"}` **or** `{"useProviderName":true}`; both or neither → `422` · `200` the P1 shape · `422` `displayName` · `409` provider name unavailable (W2) |
| D1 | `GET /v1/me/deletion-request` | `200` the open request (`DeletionResponse` shape) · `404` `not-found` when none |
| D2 | `DELETE /v1/me/deletion-request` | `200` the cancelled request · `409` no open request · `410` window closed (W4) |
| D3 | `POST /v1/privacy/deletions` | **already served by Identity** (`routes.rs`); only the gateway entry is missing. `202` `DeletionResponse` · `409` already open · `503` |

D0, D1 and D2 resolve their caller with the stored-record lookup the existing deletion handlers
use (`caller_for_deletion`), not with the denying `load_caller`. P1 and P2 are ordinary owner
reads and writes and keep the normal caller resolution, because a denied account has no settings
surface to reach.

Gateway table additions (all `requires_admin: false`, all Identity): `GET /v1/me/account-status`,
`GET /v1/me/profile`, `PATCH /v1/me/profile`, `GET /v1/me/deletion-request`,
`DELETE /v1/me/deletion-request`, `POST /v1/privacy/deletions`, and — for the export flow the
privacy panel already uses — `POST /v1/exports` and `GET /v1/exports/{exportId}`. The deletion
request is a manual trigger, so its entry carries a stricter per-minute budget than the default
(NFR-SEC-12), 5/min, matching the support-grant precedent for a manual, low-frequency,
irreversible action.

## 9. Story coverage

| Story AC | Where satisfied in this spec |
|---|---|
| AC1.4.1 — provider email and display name populate the profile unless overridden | §3.1 (both values shown; email read-only with Verified) + W1 (save sets the override) — the population itself is U1 §2.4 |
| AC1.4.2 — chosen locale and base currency survive a profile refresh | §3.2 (both are editable and read back from the service with their `source`) — the refresh rule is U1/U4 |
| AC4.3.1 — restore within 30 days | §2.1 (pending-deletion routing), §3.6 (banner with the exact date, Restore), D1/D2, W4 |
| AC4.3.2 — day 31 → deleted and erased | §3.7 deleted variant + §3.6 `410` terminal state — the erasure itself is U5 |
| AC4.3.3 — pending-deletion refused elsewhere, restore path named | §1 (the refusal is `UserStatus::denies_use`, BR1.7, and already enforced at `PrincipalService`; U5 records it `OK` against BR5.7), §2.1/§3.6 (the destination the refusal names) |
| AC6.1.1 — locale and base currency save, stored values unchanged | §3.2 (the panel saves exactly these two fields and nothing numeric) |
| AC6.1.2 — withdrawal recorded with a timestamp, visible later | §3.3 (withdraw call) + §3.3/§3.4 (the recorded date read back from history) |

## 10. Error handling summary

| Failure | Classification | Surface |
|---|---|---|
| P1/X1 read fails (network/5xx) | Recoverable | Page-level retry on the settings shell; entry routing stops at a full-page retry |
| P2 422 | Recoverable | Inline field error; the entered value is kept |
| P2 409 (no provider name) | Recoverable | Inline hint on the reset action; nothing changed |
| X2 422 | Recoverable | Per-field errors from `errors[]`; values kept |
| X5/X6 409 | Recoverable | Re-read the consent state and render the server's answer |
| D2 410 | Fatal for the recovery | Terminal "window closed" state; Sign out only |
| D1 404 | Not an error | Re-resolve the principal and route by status |
| D3 409 (already open) | Not an error | Re-read D1 and render the open-request state |
| Any 503 | Recoverable | Inline error with the correlation id; the action can be retried |
| Unexpected ProblemDetails | Recoverable | Inline error + correlation id; logged, never swallowed |

## 11. Shared components

`StatusPill` ships with this unit under `web/src/components/`, with its own test.

`RoleBadge` and `GrantBanner` do **not** ship here. Their consumers are U9 and U10, which live in
`support-console/` — a separate Yarn package with its own lockfile — and that package already
carries `src/components/StatusBadge.tsx` and `src/components/GrantBanner.tsx`. A component
authored under `web/src/components/` could not be imported there without a cross-package
dependency the repository does not have, so those two belong to the units that render them
(§12, open item 9).

## 12. Open items (recorded, not resolved here)

1. **Contract additions.** D0, P1, P2, D1 and D2 are new C1/C2 gateway paths and new C3/C6/C7
   Identity routes; D3 needs only the gateway entry. `contract-summary.md` and the C4 model must
   be updated with them in the same change that lands them (project rule: the model and
   `docs/architecture/README.md` move together).
2. **"What is kept" text.** `DeletionRequest.retained_categories` is empty when the request is
   created and is filled during erasure, so the service cannot list the kept categories before
   the person decides. The page falls back to the retention statement, whose legal text comes
   from the retention and legal-hold policy that BRD-012 still lists as "must define before
   launch".
3. **Analytics policy version.** No service publishes a current policy version and
   `ConsentPolicy::parse` accepts any 1–120-character string, so `ANALYTICS_POLICY_VERSION` is an
   app-held constant that can drift from the notice. A published version endpoint is the fix;
   until then the constant and the notice link must change together.
4. **AC4.3.3 enforcement is already in place.** `UserStatus::denies_use` returns true for
   `Disabled | PendingDeletion | Deleted` (BR1.7), and `PrincipalService` refuses those statuses
   with `403` before a use case runs, auditing `identity.principal.denied` (and
   `identity.principal.erased` for `Deleted`). U5 records AC4.3.3 as `OK` against BR5.7, not
   `Deferred`. U7's contribution is only the destination the refusal names, reached through the
   stored-record lookup D0/D1/D2 use.
5. **`/v1/me/principal` has no served route.** `web/src/api/client.ts` calls
   `GET /me/principal`, but the gateway route table declares only `/v1/users/{userId}` and the
   Identity router has no `/v1/me/principal`. The settings surface therefore takes its status
   from D0 rather than from the principal read — which would `403` for exactly the statuses
   entry routing keys on, and so could never distinguish them. Wiring or retiring that call is
   a U1/U8 follow-up, recorded here rather than resolved.
6. **`POST /v1/exports` and `GET /v1/exports/{exportId}` have no gateway entry.** Identity
   serves both (`routes.rs`), but the gateway's deny-by-default table declares neither, so the
   privacy panel's export request cannot traverse `/v1` today. This unit adds the two entries
   alongside D0–D3; the export flow itself belongs to the privacy unit that owns it.
7. **`nextStep` has no `restore_account`.** Routing keys on `status` (§2.1); the union member in
   `web/src/api/types.ts` stays for forward compatibility and is not produced by the server.
8. **Sessions panel removed** (§1). The current page's session list and its `useSessions()` call
   go with it; no gateway route exists and no U7 story covers it.
9. **`RoleBadge` and `GrantBanner` belong to U9/U10.** Neither has a U7 mount, and both would be
   authored in `support-console/`, where their consumers live. U7 defines no contract for them.