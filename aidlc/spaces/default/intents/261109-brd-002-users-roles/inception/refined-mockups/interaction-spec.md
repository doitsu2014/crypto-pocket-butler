# Interaction Specification — Users, Roles & Permissions

Component-level specifications and the four interaction flows the mockups depend on. Component specs follow `.claude/knowledge/aidlc-design-agent/component-spec-template.md`.

## Components

### RoleBadge

| Field | Value |
|---|---|
| Component | RoleBadge |
| Description | Read-only pill naming the signed-in person's role |
| Category | display |

**States**

| State | Description | Trigger |
|---|---|---|
| default | Tinted pill with a dot, uppercase 10px label: `Owner`, `Admin`, `Support` | render |
| none | Renders nothing when the role is unknown | missing claim |

**Props**

| Prop | Type | Required | Default | Description |
|---|---|---|---|---|
| role | `owner \| admin \| support` | yes | — | Which label and tint |
| showDot | boolean | no | true | Dot pairs the colour with a shape |

**Accessibility:** `role="status"` is not used; the badge is plain text so it is read in order. Colour is always paired with the word, never colour alone.

### StatusPill

| Field | Value |
|---|---|
| Component | StatusPill |
| Description | Read-only pill naming the account status |
| Category | display |

**States**

| State | Description | Trigger |
|---|---|---|
| active | Labelled `Active`, success tint | status = active |
| disabled | Labelled `Disabled`, warn tint + reason tooltip | status = disabled |
| pending_deletion | Labelled `Scheduled for deletion`, warn tint + date | status = pending_deletion |
| deleted | Labelled `Deleted`, danger tint | status = deleted |

**Accessibility:** each state carries its own label and shape (dot), so meaning never depends on colour. The pending-deletion state exposes the restore date in text.

### GrantBanner

| Field | Value |
|---|---|
| Component | GrantBanner |
| Description | Persistent banner showing an active support grant, its reason, and time remaining |
| Category | feedback |

**States**

| State | Description | Trigger |
|---|---|---|
| hidden | No active grant | no grant |
| active | Countdown, reason, metadata-only reminder, End now | approved grant in window |
| expiring | Countdown under 5 minutes, warn tint and a written warning | <5 minutes left |
| expired | Banner closes and the panel locks | window ends |
| revoked | Banner closes with a one-line "access ended by an admin" notice | admin revokes |

**Props**

| Prop | Type | Required | Default | Description |
|---|---|---|---|---|
| grantId | string | yes | — | Identifies the grant |
| expiresAt | ISO timestamp | yes | — | Drives the countdown |
| reason | string | yes | — | The recorded reason |
| onEnd | function | yes | — | Ends the grant early |

**Accessibility:** `role="status"` with `aria-live="polite"` announces the state change on grant and expiry, not the ticking countdown. Countdown text updates at most once a minute. Focus is never moved by the countdown.

### ScreenStatePanel

| Field | Value |
|---|---|
| Component | ScreenStatePanel |
| Description | The shared empty/loading/error/partial presentation for a panel |
| Category | feedback |

**States:** empty (one icon, serif title, one sentence, one primary action), loading (skeleton rows), error (plain-language message, Retry, no raw code), partial (message naming what loaded and what did not, retry for the failed part only).

**Accessibility:** error text is associated with the affected region with `aria-describedby`; Retry is reachable by keyboard; loading announces "Loading" once.

## Flows

### F1 — Support grant: request, approval, use, expiry

- **Persona:** Support Agent requests; Admin approves.
- **Trigger:** a support agent needs account metadata to answer a ticket.
- **Steps:**
  1. Support searches for a user → selects one record → enters a reason (1–500) → Request access.
  2. System creates a pending grant and audits the request.
  3. Admin sees the pending grant → checks requester and reason → Approve (disabled for the requester) or Deny.
  4. On approval the system sets an expiry no later than 60 minutes and audits it.
  5. Support sees the GrantBanner with countdown and opens the metadata panel.
  6. At expiry, or on End now, the banner closes and the panel locks.
- **Success outcome:** support reads only metadata; every access is audited.
- **Error paths:** reason too short/long → inline error, request not sent; self-approval → Approve disabled with explanation; grant expired mid-read → next request refused, panel locks, banner closes; grant revoked → same with a written notice.

### F2 — Role or status change

- **Persona:** Admin.
- **Trigger:** an access problem needs a role change or a disable.
- **Steps:**
  1. Admin opens a user record → changes Role or Status.
  2. Save asks for a reason (required).
  3. System writes the change, refreshes the user's enforcement at their next request, and audits actor, target, previous/new value, reason, outcome.
- **Success outcome:** the record shows the new value and an audit note.
- **Error paths:** missing reason → Save refused with an inline message; concurrent change by another admin → conflict message naming the current value, no silent overwrite; disabled → the user is denied at the next request.

### F3 — Delete and recover

- **Persona:** Portfolio Owner.
- **Trigger:** the owner chooses to leave.
- **Steps:**
  1. Settings → Delete account → dedicated page listing what happens and what is kept.
  2. Second explicit confirm → account enters `pending_deletion`.
  3. Every authenticated screen shows the recovery banner until the 30-day window ends.
  4. Sign-in during the window offers Restore account → active.
  5. At day 31 the account becomes `deleted`; erasure runs subject to legal holds.
- **Success outcome:** no accidental irreversible deletion; legal holds preserved.
- **Error paths:** deletion request with an active legal hold → page states which records are kept; restore after day 31 → refused with the explanation that the account is deleted; connection attempt while pending deletion → refused with the restore path named.

### F4 — Jurisdiction gate

- **Persona:** Anonymous visitor; existing owner on continued use.
- **Trigger:** sign-up, or a periodic continued-use check.
- **Steps:**
  1. Visitor selects a country and confirms age/terms.
  2. System checks the country against the maintained list and IP geolocation.
  3. Pass → continue to the first source; fail → written unavailability notice, nothing stored.
- **Success outcome:** no personal data stored for a restricted attempt; a newly restricted owner is blocked and audited.
- **Error paths:** check unavailable → retry offer, nothing stored; conflicting signals (self-declared allowed, IP restricted) → the stricter signal wins and the notice names the country.

## Feedback conventions

- **Success:** inline near the action, auto-dismiss after 5s, no immediate redirect for settings.
- **Error:** inline at the cause with a written message (never colour alone) and the input preserved.
- **Destructive:** confirmation naming what is affected; for deletion, a dedicated page plus explicit confirm.
- **Announcements:** dynamic changes (grant state, save result) use `aria-live="polite"`.
- **Motion:** 150ms micro-states, 200ms state changes, `prefers-reduced-motion` respected; no animated counters.
