# Interaction Specification — Authentication & Session Management

Component-level behaviour for every screen in `mockups.md`. Components already in
`web/src/components/index.tsx` (`ConfirmDialog`, `ErrorPanel`, `EmptyState`, `Loading`,
`StatusRegion`, `PageHeader`, `QueryBoundary`) and in the sign-up gate and settings features are
**reused, not re-specified**; this document records only what is new, what changes, and the
contract each component must meet.

---

## Journey 1 — Sign in with a password

```
[not signed in] → /  → submit → provider → session established → dashboard
```

| Step | Actor | Behaviour | Failure path |
|---|---|---|---|
| 1 | User | Opens `/` | — |
| 2 | SPA | `RequireSession` sees no session and renders `SignInPage` | — |
| 3 | User | Enters email + password, submits | Field validation names the specific problem and no request is sent |
| 4 | SPA | Disables submit, sets `submitting`, announces "Signing in" | Double submit is impossible: the handler returns early while `submitting` |
| 5 | SPA | `signInWithPassword` to Supabase Auth | Bad credentials → inline non-revealing message, password cleared, email kept |
| 5a | SPA | Provider unavailable | Outage panel (S11) replaces the error slot; retry is cooldown-gated |
| 6 | SPA | Session store updates; `RequireSession` re-renders | — |
| 7 | SPA | Redirect to `returnTo` if it is a same-app path, else `/dashboard` | An external or malformed `returnTo` is discarded |

**Contract:** the password value never leaves the browser except to Supabase Auth. No gateway call
carries it (criterion AC1.1.4).

---

## Journey 2 — Sign in with a magic link

```
/  → "Email me a link" → sent state → email → /auth/callback → dashboard
```

| Step | Actor | Behaviour |
|---|---|---|
| 1 | User | Chooses "Email me a sign-in link instead" |
| 2 | SPA | Swaps the form card for the sent state (S2); sends the request |
| 3 | SPA | Starts a cooldown timer; the resend control shows a countdown |
| 4 | User | Opens the link in the same browser |
| 5 | SPA | `/auth/callback` exchanges the code, using the persisted PKCE verifier |
| 6 | SPA | **Removes the verifier** once the exchange completes or fails |
| 7 | SPA | On success lands on the dashboard or the remembered destination |

Failure paths:

| Condition | Behaviour |
|---|---|
| Link expired or already used | S3 expired state, with "Send a new link" and "Sign in" |
| Code exchange fails | S3 error state; the verifier is removed so no stale verifier lingers |
| Opened in a different browser | Same error state; the copy explains the likely cause without naming internals |
| Resend exceeds the limit | Rate-limited copy; no email is sent; the timer shows the wait |

**Contract:** the sent state renders identically whether or not an account exists for the address.

---

## Journey 3 — Session refresh

Silent. No screen transition on the happy path.

| Condition | Behaviour |
|---|---|
| Access token expired, refresh valid | The next API call triggers a refresh first; the original call then proceeds and the user sees nothing |
| Refresh rotates the token | The previous refresh token is discarded; a concurrent tab that tries the old one receives a refusal and must re-read the session |
| Session older than 30 days from first sign-in | Refresh refused → the session is cleared → redirect to sign-in with the "sign in again" message |
| Refresh token revoked or expired | Same as above |
| Several requests discover the dead token at once | Exactly one redirect and one message; later failures are suppressed by a single-flight guard |

**Not covered by this intent:** the anchor of the 30-day cap is fixed at *first sign-in* in criterion
AC2.1.3; requirements Open Question Q3 must be closed to match (recorded in the review findings).

---

## Journey 4 — Sign out

| Step | Actor | Behaviour |
|---|---|---|
| 1 | User | Chooses sign out |
| 2 | SPA | Clears the in-memory session first |
| 3 | SPA | Calls the provider's `signOut` |
| 4 | SPA | Navigates to `/` |
| 5 | SPA | Records the sign-out for the audit trail through the gateway |

Failure path: if the provider call fails, the app session is **still cleared** and the failure is
surfaced (criterion AC2.4.3). A reload cannot resurrect the session because nothing was persisted.

---

## Journey 5 — Enrol a second factor

```
/settings/mfa → Set up → scan/copy → enter code → verify → recovery codes → done
```

| Step | Actor | Behaviour | Failure path |
|---|---|---|---|
| 1 | User | Opens "Set up authenticator" | — |
| 2 | SPA | Requests enrolment from the gateway; receives secret + otpauth URL | Error → `ErrorPanel` with retry |
| 3 | SPA | Renders the QR and the manual key once | — |
| 4 | User | Enters the six-digit code | — |
| 5 | SPA | Submits verification | Wrong or expired code → inline message under the input; the secret stays valid so the user retries without rescanning |
| 6 | SPA | On success, requests recovery codes | Error → the codes section shows a retry, and the factor is already active |
| 7 | User | Saves the codes, confirms | — |
| 8 | SPA | Banner disappears on the next render; session leaves the limited state | — |

**Contract:** the secret is rendered once and never re-fetched; the recovery codes are rendered once
and are not retrievable again. Neither is written to any persistent browser store.

---

## Journey 6 — The limited session

```
sign-in (exchange credential, no second factor) → limited → redirect /settings/mfa → enrol → full session
```

State machine on the SPA side:

| From | Event | To | Effect |
|---|---|---|---|
| any route | session is limited, route not allow-listed | `/settings/mfa` | Banner + full explanation render on arrival |
| `/settings/mfa` | factor verified | previous route or `/dashboard` | Banner removed |
| `/settings/mfa` | user declines | `/dashboard` attempt | Redirected straight back to `/settings/mfa`; the decline notice explains why |

Allow-list while limited: `/settings/mfa`, `/settings/*`, `/settings/sessions`, `/help`, sign-out.
Everything else redirects.

API side: a portfolio endpoint called while limited returns `403` with a problem body naming the
missing factor (criterion AC3.2.3). The SPA surfaces it through `ErrorPanel`, not a redirect, so a
stale tab cannot look like a crash.

---

## Journey 7 — List, revoke one, revoke all

### 7a — List

| Condition | Panel behaviour |
|---|---|
| Loading | Three skeleton rows; `role="status"` announces "Loading your sessions" |
| Loaded | Rows in the order returned by the server, current session first |
| One session | Singular heading; revoke-all absent |
| Failed | `ErrorPanel` with correlation id and retry; height reserved |

Row fields: device label, platform, coarse location, last-seen (relative), current-session marker,
action.

### 7b — Revoke a non-current session

| Step | Behaviour |
|---|---|
| 1 | User chooses "End session" on a row |
| 2 | `ConfirmDialog` opens; focus lands on Cancel |
| 3 | Confirm → the row's action becomes a busy state; the rest of the list stays interactive |
| 4 | Success → the row animates out (200ms, `--ease-standard`) and a `role="status"` note says the session ended |
| 5 | Provider failure → the row returns to its resting state and the panel shows a recoverable error naming the device |

### 7c — Revoke all others

| Step | Behaviour |
|---|---|
| 1 | User chooses "Sign out everywhere else" |
| 2 | Dialog names the count **and** the devices |
| 3 | Confirm → all non-current rows go busy together |
| 4 | Success → the list collapses to the current session; status note names the count |
| 5 | Partial failure → alert panel names the survivors; the button remains for a retry |
| 6 | The result is never reported as complete when any row remains (criterion AC5.3.2) |

### 7d — Revoke the current session

Confirm dialog → on confirm, the session is revoked and the SPA clears its token and navigates to
sign-in. No reload is required.

---

## Journey 8 — Sign-up with gates

```
/ (sign-up mode) → choices → gate authorisation → signUp → /gates → /onboarding → /dashboard
```

| Step | Behaviour | Failure path |
|---|---|---|
| 1 | The user enters email, password, country, and checks the three acceptance boxes | Submit with any unchecked → alert list names only the unanswered items |
| 2 | SPA requests a gate authorisation carrying the user's real choices | Jurisdiction refused → blocked notice with contact and sign-out |
| 3 | SPA calls `signUp` with the gate authorisation | Provider error → inline message, the choices stay checked so nothing is retyped |
| 4 | Session established; `RequireGatesPassed` sends the user to `/gates` | — |
| 5 | The gates screen confirms the remaining server-side gates | Server-named missing items re-render on the form |
| 6 | All gates pass → dashboard | — |

**Contract:** the gate authorisation is obtained after the choices, carries them, and is used once.
A replayed authorisation cannot create a second account (criterion AC4.1.5).

---

## Journey 9 — Recovery entry

```
/  → "Cannot sign in?" → /recover → submit → received (identical for every input)
```

| Step | Behaviour |
|---|---|
| 1 | The sign-in page shows a "Cannot sign in?" link beneath the form |
| 2 | `/recover` renders the entry form |
| 3 | Submit → request in flight; button disabled |
| 4 | Response → the received state, identical for a known and an unknown address |
| 5 | Rate limited → the rate-limit message; no request reaches support |

The screen never says whether a case was opened for a real account, and never names a support
action.

---

## Component specifications

Each component below follows `.claude/knowledge/aidlc-design-agent/component-spec-template.md`.

---

## SessionRow

| Field | Value |
|---|---|
| Component | `SessionRow` |
| Description | One device session with its identity, recency and revoke action |
| Category | display + action |

### States

| State | Description | Trigger |
|---|---|---|
| default | Resting row | list render |
| hover | Card lifts to `--surface-warm` | pointer over row |
| focus-within | The row's button has a `--focus-ring` | keyboard focus |
| busy | Action disabled, inline spinner on the action | revocation in flight |
| current | Row marked with dot glyph + "Current" | row is the caller's session |
| removed | Row animates out before unmount | revocation succeeded |

### Props / Inputs

| Prop | Type | Required | Default | Description |
|---|---|---|---|---|
| `deviceLabel` | string | yes | — | e.g. "Chrome on macOS" |
| `platform` | string | yes | — | e.g. "Safari on iOS" |
| `location` | string \| null | no | null | City and country code only |
| `lastSeenAt` | string (ISO) | yes | — | Rendered relatively |
| `isCurrent` | boolean | yes | — | Shows the current marker and changes the confirm copy |
| `onRevoke` | () => void | yes | — | Opens the confirmation |

### Responsive Behaviour

| Breakpoint | Behaviour |
|---|---|
| mobile (<768px) | Action stacks under the meta line, full width, 44px tall |
| tablet (768–1024px) | Action right-aligned; location may wrap |
| desktop (>1024px) | As drawn |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | Native `<li>` inside a `<ul>` labelled by the page heading |
| Keyboard interaction | Tab to the action, Enter/Space to activate |
| Label | The action's accessible name includes the device: "End session: Safari on iOS" |
| Contrast ratio | Device label `--fg` on `--surface`; meta `--meta` (4.64:1 on `--bg`) |
| Screen reader | The current marker reads as "This device, current session" |
| Focus management | On removal, focus moves to the next row's action, or to the list heading if none |

---

## LimitedSessionBanner

| Field | Value |
|---|---|
| Component | `LimitedSessionBanner` |
| Description | Persistent notice that the session cannot reach the portfolio until a second factor exists |
| Category | feedback |

### States

| State | Description | Trigger |
|---|---|---|
| hidden | Not rendered | session is not limited |
| shown | Banner with explanation and action | session is limited |
| dismissing | Never — the banner is not dismissible | — |

### Props / Inputs

| Prop | Type | Required | Default | Description |
|---|---|---|---|---|
| `reason` | `"exchange-credential"` | yes | — | Selects the copy |
| `onAct` | () => void | yes | — | Navigates to `/settings/mfa` |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | `role="status"`, `aria-live="polite"` — announced once, not repeatedly |
| Keyboard interaction | The action is a normal button, reachable by Tab |
| Label | The action reads "Add a second factor" |
| Contrast ratio | `--fg` on `--surface-warm` — clears AA |
| Screen reader | The whole banner reads as one sentence pair, not two fragments |
| Focus management | No focus is taken; the banner never steals focus on navigation |

---

## MagicLinkSentState

| Field | Value |
|---|---|
| Component | `MagicLinkSentState` |
| Description | The confirmation card after a sign-in link is requested |
| Category | feedback |

### States

| State | Description | Trigger |
|---|---|---|
| sent | Confirmation with resend enabled | first send succeeded |
| cooling | Resend disabled with a countdown | cooldown running |
| limited | Resend replaced by the wait message | rate limit hit |

### Props / Inputs

| Prop | Type | Required | Default | Description |
|---|---|---|---|---|
| `email` | string | yes | — | Echoed back, never used to confirm existence |
| `cooldownSeconds` | number | yes | 30 | Remaining wait |
| `onResend` | () => void | yes | — | Resends |
| `onUsePassword` | () => void | yes | — | Returns to the password form |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | The card is a `<section>` with an `<h2>`; the countdown is `aria-live="off"` to avoid chatter |
| Keyboard interaction | Both actions are buttons in reading order |
| Screen reader | The initial message is announced once; the countdown is not announced per second |
| Focus management | On arrival, focus moves to the card heading so the change of state is heard |

---

## TotpCodeInput

| Field | Value |
|---|---|
| Component | `TotpCodeInput` |
| Description | Six-digit code entry for enrolment and step-up |
| Category | input |

### States

| State | Description | Trigger |
|---|---|---|
| default | Empty, focused by default | enrolment reaches the verify step |
| invalid | Error border + message | wrong or expired code |
| submitting | Disabled | verification in flight |
| success | Brief success state, then advances | verified |

### Props / Inputs

| Prop | Type | Required | Default | Description |
|---|---|---|---|---|
| `onSubmit` | (code: string) => void | yes | — | Fires at six digits |
| `error` | string \| null | no | null | Rendered under the field |
| `busy` | boolean | no | false | Disables input and submit |
| `autoFocus` | boolean | no | true | Focuses on mount |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | Single `<input inputmode="numeric" autocomplete="one-time-code">` |
| Keyboard interaction | Numeric keypad on mobile; paste of a full code fills the field |
| Label | Visible "Enter the 6-digit code" above the field |
| Contrast ratio | `--fg` on `--surface`; error border `--danger` plus a written message |
| Screen reader | The error is announced via `aria-describedby`, not colour alone |
| Focus management | Focus stays in the field after an error so the user can retype |

---

## RecoveryCodesPanel

| Field | Value |
|---|---|
| Component | `RecoveryCodesPanel` |
| Description | The one-time display of recovery codes after enrolment |
| Category | display |

### States

| State | Description | Trigger |
|---|---|---|
| shown | Codes in two columns with the one-time warning | enrolment succeeded |
| downloading | Download triggered | user chooses download |
| acknowledged | Panel replaced by the "on" status | user confirms they saved them |

### Props / Inputs

| Prop | Type | Required | Default | Description |
|---|---|---|---|---|
| `codes` | string[] | yes | — | Rendered once |
| `onDownload` | () => void | yes | — | Saves a text file |
| `onAcknowledge` | () => void | yes | — | Completes the flow |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | `<ul>` of codes, each a `<li>` in mono |
| Keyboard interaction | Both actions reachable by Tab |
| Screen reader | The warning is part of the panel's accessible description, read before the codes |
| Focus management | On arrival, focus moves to the warning heading so the "shown once" fact is heard first |

---

## AuthOutagePanel

| Field | Value |
|---|---|
| Component | `AuthOutagePanel` |
| Description | The shared provider-unavailable state for every auth screen |
| Category | feedback |

### States

| State | Description | Trigger |
|---|---|---|
| shown | Message + retry | a request fails with an upstream-unavailable signal |
| cooling | Retry disabled with a countdown | a retry just failed |
| recovered | Panel removed; the screen re-renders | the next attempt succeeds |

### Props / Inputs

| Prop | Type | Required | Default | Description |
|---|---|---|---|---|
| `onRetry` | () => void | yes | — | Retries the failed action |
| `cooldownSeconds` | number | no | 5 | Retry gate |
| `correlationId` | string \| null | no | null | Shown at `--meta` |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | `role="alert"` — this is a failure, not a status |
| Keyboard interaction | Retry is a button; when cooling it is `disabled` |
| Label | "Try again" / "Try again in {n}s" |
| Contrast ratio | Heading `--fg`; body `--ink-2`; the glyph is `--danger` with the words carrying meaning |
| Screen reader | The cooling countdown is not announced per second |

---

## RecoverAccountForm

| Field | Value |
|---|---|
| Component | `RecoverAccountForm` |
| Description | The recovery entry screen's form and received state |
| Category | input + feedback |

### States

| State | Description | Trigger |
|---|---|---|
| default | Email field + submit | page load |
| submitting | Button disabled, "Starting your recovery…" | submit |
| received | Identical confirmation for every input | response |
| limited | Rate-limit message | limit hit |
| error | Recoverable error with retry | request failed |

### Props / Inputs

| Prop | Type | Required | Default | Description |
|---|---|---|---|---|
| `onSubmit` | (email: string) => Promise<void> | yes | — | Starts the case |
| `state` | `"default" \| "submitting" \| "received" \| "limited" \| "error"` | yes | — | Rendered state |

### Accessibility

| Requirement | Implementation |
|---|---|
| ARIA role | `<form>`; the received state is `role="status"` and takes focus |
| Keyboard interaction | Standard form; Enter submits |
| Label | Visible "Email on the account" above the field |
| Screen reader | The received message is announced once, and focus moves to it |

---

## Interaction rules that apply everywhere

1. **Loading, empty, error** are three distinct renderings on every async surface; a spinner where an
   empty state belongs is a defect.
2. **No destructive action without a named consequence.** Every dialog says what ends and what
   survives.
3. **Focus is managed, never lost.** Dialogs trap and restore it; route changes move it to the page
   heading; list removals move it to the next item.
4. **Announcements are polite by default.** `role="alert"` is reserved for failures the user did not
   cause and must act on.
5. **No countdown is announced per second**; only the end state is announced.
6. **Nothing sensitive is rendered in a URL.** The recovery entry takes an email in a form, never a
   query parameter; the sign-in return path is validated as a same-app path.
7. **A session state change re-renders without a reload.** Losing the limited state, ending a
   session, and signing out all take effect on the next render.

---

## Flows not specified here

- **Administrator session actions in the support console** — a separate surface; its contract is
  recorded in the requirements as FR4.7 and carried to Functional Design.
- **Supabase Auth callback internals** — the provider's own screens are not redesigned; only the
  SPA's callback route is.
