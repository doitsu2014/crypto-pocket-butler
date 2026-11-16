# Refined Mockups — Authentication & Session Management

**Scope (interview Q1):** every screen the stories touch. Existing pages are documented as **current
state** so the change is visible against what ships today; new and changed surfaces are specified in
full. The classic scope skipped Ideation, so there are no rough wireframes to refine — these are
designed from `stories.md` and `requirements.md`.

**Format (interview Q2):** annotated ASCII wireframes with all five screen states, plus the exact
token each value uses. Tokens are from `design-system/tokens.css`; no raw values are introduced.

**Copy (interview Q6):** exact user-facing copy is written for every state, including the
non-revealing variants that criteria depend on.

**Navigation (interview Q3):** Sessions becomes a fifth settings destination.

**Limited session (interview Q4):** a persistent banner plus an explained redirect.

---

## Screen inventory

| # | Screen | Route | State today | Work in this intent |
|---|---|---|---|---|
| S1 | Sign-in | `/` | Email/password only; hard-coded gate values on sign-up | Add magic link, neutral error copy, remove the raw gate-token field, add gate choices |
| S2 | Magic-link sent | `/` (same route, sent state) | Does not exist | New state on S1 |
| S3 | Auth callback | `/auth/callback` | Handled by `detectSessionInUrl` on `/` | New explicit route with success, expired and error states |
| S4 | Sign-up gates | `/gates` | Exists; five consent gates, full state machine | Unchanged visually; the jurisdiction refusal copy is reviewed |
| S5 | Verify email | `/verify-email` | Exists | Add the rate-limited resend and the neutral sent message |
| S6 | MFA settings & enrolment | `/settings/mfa` | Enrol button, secret, recovery codes, decline | Add QR, code entry, verification, step-up and the limited-session explanation |
| S7 | Sessions | `/settings/sessions` | Does not exist | **New** — list, revoke one, revoke all |
| S8 | Limited-session banner | All routes while limited | Redirect only | **New** — persistent banner on the allowed screens |
| S9 | Account recovery request | `/recover` | Does not exist | **New** — support-assisted entry |
| S10 | Notifications (login alerts) | `/notifications` | Exists | Add the new-device alert kind and its empty state |
| S11 | Outage state | All auth screens | Error text only | **New** — one shared unavailable state |
| S12 | Unsupported browser | All routes | None | **New** — one message |

Existing screens not otherwise changed by this intent (dashboard, portfolios, briefs, connect, help,
settings profile/preferences/consent/privacy) are out of scope for these mockups.

---

## S1 — Sign-in

Route `/`: a two-column editorial split on desktop, stacking on phone. The left column carries the
product voice; the right is the form card. This is the layout that ships today and it is kept.

### Desktop, default (sign-in mode)

```
+-------------------------------------------------------------------------------+
|  (64px header, parchment --bg, brand lockup left)                    [Help]   |
+-------------------------------------------------------------------------------+
|                                                                               |
|  +---------------------------------+  +-------------------------------------+  |
|  | CRYPTO POCKET BUTLER            |  |  Sign in                            |  |
|  | (10px overline, --meta)         |  |  (Fraunces 25px 500, --fg)          |  |
|  |                                 |  |                                     |  |
|  | Your portfolio, on the record.  |  |  Email                              |  |
|  | (Fraunces 52px 500, --fg)       |  |  [________________________________] |  |
|  |                                 |  |  (input 12px radius, --border,      |  |
|  | Bring every account into one    |  |   --surface fill, 14px --fg)        |  |
|  | ledger you can audit.           |  |                                     |  |
|  | (Instrument Sans 16px, --muted) |  |  Password                           |  |
|  |                                 |  |  [________________________________] |  |
|  |                                 |  |                                     |  |
|  |                                 |  |  [ Sign in            ] (primary)   |  |
|  |                                 |  |  (terracotta --accent, 14px/500)    |  |
|  |                                 |  |                                     |  |
|  |                                 |  |  Email me a sign-in link instead    |  |
|  |                                 |  |  (14px --ink-2, underlined)         |  |
|  |                                 |  |                                     |  |
|  |                                 |  |  ─────────────────────────────      |  |
|  |                                 |  |  New here? Create an account        |  |
|  +---------------------------------+  +-------------------------------------+  |
|                                                                               |
+-------------------------------------------------------------------------------+
```

Region notes:
- Card: `--surface`, 1px `--border`, `--radius-sm` (8px), 24px padding, `--elev-raised`.
- Section title: Fraunces 25px weight 500, `--fg`, `--leading-tight`.
- Labels above inputs, 14px weight 500, `--fg-2`. Inputs 12px radius, `--border`, `--surface`.
- Primary button: `--accent` fill, `--accent-on` label, `--radius-sm`, 14px/500.
- Exactly one terracotta moment per screen: the primary button.
- "Email me a sign-in link instead" is a secondary text action at `--ink-2` with underline. It is
  not a fifth route — it switches S1 into S2.

### States

| State | Trigger | What renders |
|---|---|---|
| **Default** | Page load | As drawn |
| **Loading** | Submit in flight | Primary button shows `Signing in…`, is `disabled` at 45% opacity; inputs stay enabled but read-only; a `role="status"` region announces "Signing in" |
| **Error — bad credentials** | Provider refuses | One inline message under the password field, `--danger` with a leading icon shape; password field cleared; email kept |
| **Error — provider unavailable** | Provider unreachable | The outage panel (S11) replaces the form's error slot |
| **Edge — field validation** | Blur with invalid input | Message under the field, `--danger`, naming the fix |

**Exact copy**

- Bad credentials (non-revealing — must not distinguish a missing account from a wrong password):
  **"That email and password do not match an account. Check both and try again."**
- Field: email invalid → **"Enter an email address in the form name@example.com."**
- Field: password short → **"Passwords are at least 8 characters."**
- Submit in flight → **"Signing in…"**
- Empty-password guard (no request sent) → **"Enter your password."**

Deliberate wording notes:
- The bad-credentials message is identical for an unknown address and a wrong password. It names
  "an account" rather than "your account".
- No message reveals whether the address exists, and no message names the provider.

### Sign-up mode (switched from S1)

The current page takes a **raw gate-token text field** (`signup-gate-token`) and sends fixed
`jurisdiction: "supported"` and `ageConfirmed: true`. Both are replaced (stories US4.1, US4.2):

- The raw token field is **removed from the UI**. The gate authorisation is obtained by the page from
  the signup gate after the user's real choices, and passed to `signUp`.
- The three acceptance controls and the country select come from the existing sign-up-gates
  components (`AcceptanceCheckbox`, `CountrySelect`) rather than a second implementation.

```
+-------------------------------------+
|  Create an account                  |
|                                     |
|  Country                            |
|  [ United Kingdom          v ]      |
|                                     |
|  [ ] I accept the terms of use      |
|  [ ] I understand the risk          |
|      disclosure                     |
|  [ ] I confirm I am 18 or over      |
|                                     |
|  [ Continue            ] (primary)  |
|                                     |
|  Already have an account? Sign in   |
+-------------------------------------+
```

- All three checkboxes start **unchecked** (criterion AC4.1.1 — no pre-selected value).
- Country has no default; it is a real choice.
- Submit is enabled, and validation runs on submit so each unanswered control is named
  (criterion AC4.1.2). Disabling until valid hides *which* item is missing.

**Exact copy**

- Title: **"Create an account"**
- Terms label: **"I accept the terms of use"**
- Risk label: **"I understand the risk disclosure"**
- Age label: **"I confirm I am 18 or over"**
- Missing items summary (rendered as a `role="alert"` list above the submit button):
  **"Before you can continue: accept the terms of use, acknowledge the risk disclosure, confirm your age."** — the list names only the unanswered items, in that order.
- Jurisdiction refusal (from S4's existing `BlockedJurisdictionNotice`, copy reviewed):
  **"Crypto Pocket Butler is not available in your region yet. We cannot open an account from {country}."**
- Jurisdiction undetermined: **"We could not confirm your region, so we cannot open an account right now. Try again from your usual network, or contact support."**

---

## S2 — Magic-link sent

A state of `/`, not a new route. It replaces the form card so the user cannot resubmit blindly.

```
+-------------------------------------+
|  Check your email                   |
|  (Fraunces 25px 500, --fg)          |
|                                     |
|  We sent a sign-in link to          |
|  maya@example.com                   |
|  (Instrument Sans 16px, --ink-2)    |
|                                     |
|  The link works once and expires    |
|  in 15 minutes.                     |
|  (14px, --muted)                    |
|                                     |
|  [ Send it again ] (secondary)      |
|  [ Use a password instead ] (ghost) |
|                                     |
|  Sent 30 seconds ago                |
|  (10px overline, --meta)            |
+-------------------------------------+
```

States:

| State | Rendering |
|---|---|
| **Sent** | As drawn; "Send it again" enabled |
| **Cooling down** | "Send it again" disabled at 45%, with a live countdown in `--meta` beneath it |
| **Rate limited** | The button is replaced by: **"Too many requests. You can ask for another link in 5 minutes."** |
| **Edge — resend succeeded** | The "Sent … ago" line resets and a `role="status"` announcement says **"A new link is on its way."** |

**Exact copy**

- Title: **"Check your email"**
- Body: **"We sent a sign-in link to {email}."**
- Sub: **"The link works once and expires in 15 minutes."**
- Resend: **"Send it again"**
- Cooling down label: **"Send it again in {n}s"**
- Rate limited: **"Too many requests. You can ask for another link in 5 minutes."**

Note: the confirmation is the **same** whether or not an account exists for the address
(criterion AC1.2.1). No "we could not find that address" variant exists.

---

## S3 — Auth callback

Route `/auth/callback`. Renders a centred card, no navigation, because the user arrives from an
email client and has no context.

```
+-------------------------------------------+
|                                           |
|            (parchment --bg)               |
|                                           |
|        +--------------------------+       |
|        |  [ ● ● ● ]  (spinner,     |       |
|        |            --muted)      |       |
|        |                          |       |
|        |  Signing you in…         |       |
|        |  (Fraunces 25px, --fg)   |       |
|        |                          |       |
|        |  Hold on a moment.       |       |
|        |  (14px, --muted)         |       |
|        +--------------------------+       |
|                                           |
+-------------------------------------------+
```

States:

| State | Rendering |
|---|---|
| **Loading** | Spinner + "Signing you in…" |
| **Success** | Redirect to the dashboard (or the remembered destination) with no intermediate flash |
| **Link expired** | The card swaps to the expired state |
| **Error** | The card swaps to the failure state |

```
Expired state                              Error state
+--------------------------+               +--------------------------+
|  !  This link has        |               |  !  We could not finish  |
|     expired              |               |     signing you in       |
|                          |               |                          |
|  Sign-in links last      |               |  This can happen if the  |
|  15 minutes and work     |               |  link was opened in a    |
|  once. Ask for a new     |               |  different browser.      |
|  one.                    |               |                          |
|                          |               |                          |
|  [ Send a new link ]     |               |  [ Try signing in ]      |
|  [ Sign in ]             |               |  [ Send a new link ]     |
+--------------------------+               +--------------------------+
```

Icon treatment: the `!` glyph is an **exclamation inside a circle**, rendered in `--warn` for
expired and `--danger` for the failure state, always paired with the words (colour never alone).
The card uses `--radius-sm`, `--surface`, `--border` and 32px padding.

**Exact copy**

- Loading: **"Signing you in…"** / **"Hold on a moment."**
- Expired title: **"This link has expired"**
- Expired body: **"Sign-in links last 15 minutes and work once. Ask for a new one."**
- Expired actions: **"Send a new link"**, **"Sign in"**
- Failure title: **"We could not finish signing you in"**
- Failure body: **"This can happen if the link was opened in a different browser."**
- Failure actions: **"Try signing in"**, **"Send a new link"**

---

## S4 — Sign-up gates (current state, reviewed)

Route `/gates`. The screen and its state machine exist and work. Only the jurisdiction refusal copy
is reviewed here; the layout is unchanged.

```
+-------------------------------------------------+
|  Before you start                               |
|  (Fraunces 25px 500)                            |
|                                                 |
|  A few confirmations are required before we     |
|  can set up your account.                       |
|  (16px, --muted)                                |
|                                                 |
|  +-------------------------------------------+  |
|  | [x] Terms of use               recorded   |  |
|  | [ ] Risk disclosure            required   |  |
|  | [ ] Age confirmation           required   |  |
|  | [ ] Privacy consent            required   |  |
|  | [ ] Marketing consent          optional   |  |
|  |                                           |  |
|  | [ Confirm                ]   (primary)    |  |
|  +-------------------------------------------+  |
+-------------------------------------------------+
```

States: **loading** (pulse text "Checking your account…"), **form** (as drawn), **submitting**
(button disabled, "Saving…"), **blocked** (`BlockedJurisdictionNotice`), **error** with a
correlation id and a retry.

**Reviewed copy for the refusal states**

- Blocked jurisdiction title: **"We cannot open an account from {country}"**
- Blocked body: **"Crypto Pocket Butler is available in a limited set of regions today. If you
  believe this is a mistake, contact support."**
- Blocked actions: **"Contact support"** (ghost), **"Sign out"** (secondary)
- "Nothing saved" error: **"Nothing was saved. Try again."** + correlation id at `--meta`
- "Incomplete" error: **"Part of your answer did not reach us. Check the highlighted items and
  confirm again."** + correlation id

---

## S5 — Verify email (current state, extended)

Route `/verify-email`.

```
+-------------------------------------------------+
|  Verify your email                              |
|                                                 |
|  We need to confirm your address before you     |
|  can connect an exchange.                       |
|                                                 |
|  +-------------------------------------------+  |
|  |  !  Not verified yet                      |  |
|  |     (warn glyph + words, --surface-warm)  |  |
|  +-------------------------------------------+  |
|                                                 |
|  [ Resend the email ] (secondary)               |
|  [ I have verified — check again ] (ghost)      |
|                                                 |
|  Sent 2 minutes ago  (10px overline, --meta)    |
+-------------------------------------------------+
```

States: **unverified**, **resent** (`role="status"` "A new email is on its way."), **cooling down**
(button disabled with a countdown), **rate limited**, **verified** (redirect to the exchange form),
**error** (panel with retry).

**Exact copy**

- Title: **"Verify your email"**
- Body: **"We need to confirm your address before you can connect an exchange."**
- Unverified callout: **"Not verified yet"**
- Resend: **"Resend the email"**
- Recheck: **"I have verified — check again"**
- Resent: **"A new email is on its way."**
- Rate limited: **"Too many requests. Try again in 10 minutes."**
- Still unverified after recheck: **"We have not received the confirmation yet. It can take a minute."**

The rate-limited and resent messages are the same whether or not the address has an account
(criterion AC4.3.2).

---

## S6 — MFA settings and enrolment

Route `/settings/mfa`. This screen carries three jobs: enrol, verify, and explain the limited
session. It is reachable while limited, which is why the banner (S8) appears here too.

### Step 1 — landing

```
+-------------------------------------------------------------------+
|  !  Your account is limited until you add a second factor.        |
|     (persistent banner, --surface-warm, --border, --radius-md)    |
|     You have an exchange connection, so we ask for a code as      |
|     well as your password. Add an authenticator app to continue.  |
|     [ Learn more ] (ghost)                                        |
+-------------------------------------------------------------------+
|  Two-factor authentication                                        |
|  (Fraunces 25px 500)                                              |
|                                                                   |
|  Not set up yet             (—meta, with a ○ ring glyph)          |
|                                                                   |
|  +-------------------------------------------------------------+  |
|  | Use an authenticator app (recommended)                      |  |
|  | Time-based codes from Authy, 1Password, Google              |  |
|  | Authenticator, or any TOTP app.                             |  |
|  |                                                             |  |
|  | [ Set up authenticator ]  (primary)                         |  |
|  +-------------------------------------------------------------+  |
+-------------------------------------------------------------------+
```

### Step 2 — scan and verify

```
+-------------------------------------------------------------------+
|  Set up your authenticator                                         |
|                                                                   |
|  +--------------------+   1. Scan this code in your app           |
|  |  █▀▀▀▀▀█  █ ▀▀█▀▀  |      (14px, --ink-2)                      |
|  |  █ ███ █  ▀█▀ █▀  |                                           |
|  |  █▀▀▀▀▀█  █▀▀█ ▀  |   2. Or enter this key by hand             |
|  |  ▀▀▀▀▀▀▀  ▀ ▀▀▀▀  |      ABCDEF GHJKLM NPQRST UVWXYZ          |
|  +--------------------+      (mono, --fg, break-all)              |
|  (128x128, --surface, --border)                                   |
|                                                                   |
|  +--------------------+   Enter the 6-digit code                  |
|  |  0 0 0 0 0 0       |      (label above, 14px/500)              |
|  +--------------------+      [____________________]               |
|  (mono 20px, tabular,         (input, inputmode=numeric)          |
|   letter-spaced)                                                  |
|                                                                   |
|  [ Verify and finish ]  (primary)                                 |
|  [ Cancel ]  (ghost)                                              |
+-------------------------------------------------------------------+
```

### Step 3 — recovery codes

```
+-------------------------------------------------------------------+
|  Recovery codes                                                    |
|                                                                   |
|  !  These are shown once. Store them somewhere safe and           |
|     offline — each one signs you in if you lose your              |
|     authenticator.     (--surface-warm callout, --warn glyph)     |
|                                                                   |
|  +---------------------------+  +---------------------------+     |
|  |  4F2K-9QX7  (mono)        |  |  8M3P-2TWK  (mono)        |     |
|  |  7Y6D-4HJR                |  |  2Q9X-6BVD                |     |
|  |  ...                      |  |  ...                      |     |
|  +---------------------------+  +---------------------------+     |
|  (two columns, 16px gap, --surface inset)                         |
|                                                                   |
|  [ Download codes ] (secondary)   [ I have saved them ] (primary) |
+-------------------------------------------------------------------+
```

### States

| State | Rendering |
|---|---|
| **Loading** | Skeleton bars in the card; `role="status"` "Loading your security settings" |
| **Empty — not enrolled** | Step 1 |
| **Enrolling** | Step 2; the QR and key shown **once** |
| **Verifying** | "Verify and finish" disabled, label "Checking…" |
| **Error — wrong code** | Inline under the code input: **"That code did not work. Codes change every 30 seconds — try the current one."** |
| **Success** | Step 3, then the banner disappears and the screen shows "Two-factor authentication is on" |
| **Edge — declined** | The limited notice persists: **"Without a second factor, your portfolio stays unavailable while you have an exchange connection."** |

**Exact copy**

- Banner title: **"Your account is limited until you add a second factor."**
- Banner body: **"You have an exchange connection, so we ask for a code as well as your password. Add an authenticator app to continue."**
- Section title: **"Two-factor authentication"**
- Empty status: **"Not set up yet"** with a `○` ring glyph (shape + word)
- Enrolled status: **"On, added {date}"** with a `●` dot glyph
- Enrol card: **"Use an authenticator app (recommended)"** / **"Time-based codes from Authy, 1Password, Google Authenticator, or any TOTP app."**
- Enrol button: **"Set up authenticator"**
- Step 2 title: **"Set up your authenticator"**
- QR instruction: **"Scan this code in your app"**
- Manual instruction: **"Or enter this key by hand"**
- Code label: **"Enter the 6-digit code"**
- Verify button: **"Verify and finish"**
- Wrong code: **"That code did not work. Codes change every 30 seconds — try the current one."**
- Recovery title: **"Recovery codes"**
- Recovery warning: **"These are shown once. Store them somewhere safe and offline — each one signs you in if you lose your authenticator."**
- Recovery actions: **"Download codes"**, **"I have saved them"**
- Declined notice: **"Without a second factor, your portfolio stays unavailable while you have an exchange connection."**

Step-up reuse: the same code-entry block appears inline on the password, MFA-method and
recovery-code change forms, with the heading **"Confirm it's you"** and the body
**"Enter a code from your authenticator to continue."**

---

## S7 — Sessions (new)

Route `/settings/sessions`, the fifth settings destination. Added to `SettingsNav` after Privacy.

```
+-------------------------------------------------------------------+
|  Settings                                                         |
|  +----------------+  +------------------------------------------+  |
|  | Profile        |  |  Sessions                                |  |
|  | Preferences    |  |  Devices signed in to your account.      |  |
|  | Consent        |  |  End any you do not recognise.           |  |
|  | Privacy        |  |                                          |  |
|  | Sessions   ◀   |  |  +------------------------------------+  |  |
|  +----------------+  |  | This device          ● Current     |  |  |
|  (240px sidebar)     |  | Chrome on macOS                    |  |  |
|                      |  | London, GB · Active now            |  |  |
|                      |  | (device 16px --fg, meta 14px)      |  |  |
|                      |  +------------------------------------+  |  |
|                      |  | iPhone 15                       ○  |  |  |
|                      |  | Safari on iOS                      |  |  |
|                      |  | London, GB · Last seen 2 hours ago |  |  |
|                      |  |                      [ End session ]|  |  |
|                      |  +------------------------------------+  |  |
|                      |  | Windows PC                      ○  |  |  |
|                      |  | Edge on Windows                    |  |  |
|                      |  | Berlin, DE · Last seen 3 days ago  |  |  |
|                      |  |                      [ End session ]|  |  |
|                      |  +------------------------------------+  |  |
|                      |                                          |  |
|                      |  End every session except this one       |  |
|                      |  [ Sign out everywhere else ] (secondary)|  |
|                      |  (danger-tinted label, --danger text)    |  |
|                      |                                          |  |
|                      |  Why a revoked device may still appear   |  |
|                      |  briefly active     (disclosure, ghost)  |  |
+-------------------------------------------------------------------+
```

Row anatomy: a bordered `--surface` card per session (`--radius-sm`, 16px padding), device label in
16px `--fg`, platform and location in 14px `--meta`, the action right-aligned on desktop and
stacked on phone. The current session carries a **dot glyph + the word "Current"**, never colour
alone.

### States

| State | Rendering |
|---|---|
| **Loading** | Three skeleton cards; `role="status"` "Loading your sessions" |
| **Populated** | As drawn |
| **Exactly one session** | Current card only; heading reads **"1 device signed in"**; "Sign out everywhere else" is absent, not disabled |
| **Empty** | Not expected (the caller is a session), so the copy is defensive: **"We could not find any sessions. Refresh to try again."** with a retry |
| **Error** | `ErrorPanel` with a correlation id and retry; the list area keeps its height so the page does not jump |
| **Partial — one revocation failed in a bulk action** | A `role="alert"` panel above the list: **"2 of 3 sessions ended. Safari on iOS is still active — try again."** |
| **Edge — stale row** | If a listed session was already revoked elsewhere, revoking it returns success and the row is removed with a `role="status"` note **"That session had already ended."** |

**Exact copy**

- Page title: **"Sessions"**
- Description: **"Devices signed in to your account. End any you do not recognise."**
- Current badge: **"This device"** + **"Current"**
- Last seen: **"Last seen {relative}"** / **"Active now"**
- End one: **"End session"**
- End all: **"Sign out everywhere else"**
- Revocation window disclosure (this is the accepted limit from requirements FR3.8, made visible
  rather than implicit — criterion AC5.2.6):
  **"A device you end may keep working for up to an hour, until its current access pass expires. To
  cut that short, sign out everywhere else and change your password."**
- Empty/defensive: **"We could not find any sessions. Refresh to try again."**
- Bulk partial failure: **"{n} of {total} sessions ended. {device} is still active — try again."**
- Already ended: **"That session had already ended."**

### Confirmations (interview Q5 — confirm both)

**End a non-current session**

```
+---------------------------------------------+
|  End this session?                          |
|                                             |
|  Safari on iOS in London will be signed     |
|  out. It can sign in again with your        |
|  password.                                  |
|                                             |
|  [ End session ]  (danger)  [ Cancel ]      |
+---------------------------------------------+
```

**End the current session**

```
+---------------------------------------------+
|  End your current session?                  |
|                                             |
|  You will be signed out of this browser     |
|  and taken to the sign-in page. Your        |
|  other 2 devices stay signed in.            |
|                                             |
|  [ End session and sign out ]  (danger)     |
|  [ Cancel ]                                 |
+---------------------------------------------+
```

**Sign out everywhere else**

```
+---------------------------------------------+
|  Sign out everywhere else?                  |
|                                             |
|  2 other sessions will end. You stay        |
|  signed in here.                            |
|                                             |
|  Chrome on macOS and iPhone 15 are          |
|  affected.                                  |
|                                             |
|  [ Sign out 2 sessions ]  (danger)          |
|  [ Cancel ]                                 |
+---------------------------------------------+
```

Dialog behaviour: focus enters on **Cancel** (the least destructive control), Escape closes,
Tab is trapped, focus returns to the trigger on close. This is the existing `ConfirmDialog`
behaviour and it is reused unchanged.

---

## S8 — Limited-session banner (new)

A persistent banner rendered inside the app shell whenever the session is limited. It is not a
modal: the user must be able to reach the MFA screen, and a modal on every route would trap them
away from the one screen that lifts the restriction (interview Q4).

```
+-------------------------------------------------------------------+
|  !  Your account is limited.                    (--surface-warm)  |
|     Add a second factor to see your portfolio.  (--fg, 14px/500)  |
|                                              [ Add a second      |
|                                                factor ]  (primary)|
+-------------------------------------------------------------------+
```

Position: directly beneath the 64px header, full content width, `--surface-warm` fill,
1px `--border`, `--radius-md`, 12px vertical padding, sticky so it stays visible while scrolling.

Behaviour:
1. Any attempt to reach a gated route while limited **redirects** to `/settings/mfa` and the page
   opens with the same explanation, in full, at the top of the content (S6 step 1).
2. The banner is not dismissible. It is not an error and it carries no `role="alert"`; it is a
   `role="status"` region announced once on arrival.
3. Once the factor is verified, the banner disappears on the next render — no reload needed.

Routes reachable while limited: `/settings/mfa`, `/settings` (and its four children),
`/settings/sessions`, `/help`, and sign-out. Everything else redirects.

**Exact copy**

- Banner: **"Your account is limited. Add a second factor to see your portfolio."**
- Banner action: **"Add a second factor"**
- The full explanation on the destination screen is S6's banner body (above).

---

## S9 — Account recovery request (new)

Route `/recover`. Linked from the sign-in page ("Cannot sign in?") and from the expired-link screen.
This is the **entry only** — the re-proof and completion happen with a support agent, not in this
screen.

```
+-------------------------------------------------------------------+
|                                                                   |
|        +--------------------------------------------------+       |
|        |  Recover your account                            |       |
|        |  (Fraunces 25px 500)                             |       |
|        |                                                  |       |
|        |  Use this if you cannot reach the email or the   |       |
|        |  device on your account. A support agent will    |       |
|        |  verify who you are before anything changes.     |       |
|        |  (16px, --muted)                                 |       |
|        |                                                  |       |
|        |  Email on the account                            |       |
|        |  [________________________________]              |       |
|        |                                                  |       |
|        |  [ Start recovery ]  (primary)                   |       |
|        |                                                  |       |
|        |  ─ what happens next ───────────────             |       |
|        |  1  We check the details you gave us             |       |
|        |  2  A support agent contacts you                 |       |
|        |  3  Every existing session is ended              |       |
|        |  (ordered list, 14px, --ink-2)                   |       |
|        |                                                  |       |
|        |  This can take up to one business day.           |       |
|        |  (14px, --meta)                                  |       |
|        +--------------------------------------------------+       |
|                                                                   |
+-------------------------------------------------------------------+
```

States: **default**, **submitting** ("Starting your recovery…"), **received**, **rate limited**,
**error**.

**Received state — the same response for every input (criterion AC7.3.1):**

```
+--------------------------------------------------+
|  !  Your request is with our support team        |
|                                                  |
|  If an account matches what you entered, we      |
|  will contact you at the recovery address        |
|  within one business day.                        |
|                                                  |
|  [ Back to sign in ]  (secondary)                |
+--------------------------------------------------+
```

**Exact copy**

- Title: **"Recover your account"**
- Intro: **"Use this if you cannot reach the email or the device on your account. A support agent will verify who you are before anything changes."**
- Field: **"Email on the account"**
- Submit: **"Start recovery"**
- Steps heading: **"What happens next"**; items: **"We check the details you gave us"**, **"A support agent contacts you"**, **"Every existing session is ended"**
- Duration note: **"This can take up to one business day."**
- Received title: **"Your request is with our support team"**
- Received body: **"If an account matches what you entered, we will contact you at the recovery address within one business day."**
- Rate limited: **"Too many requests. Try again in an hour."**
- Error: **"We could not start your recovery. Try again, or contact support."**

The received copy is intentionally conditional ("If an account matches…"). It must render
identically for a known and an unknown address.

---

## S10 — Login alerts in notifications (current screen, new kind)

Route `/notifications`, which exists. A new-device alert is a new row kind, not a new screen.

```
+-------------------------------------------------------------------+
|  Notifications                                                    |
|                                                                   |
|  +-------------------------------------------------------------+  |
|  | ●  New device signed in                        (--fg, 16px)   |  |
|  |    Chrome on macOS · Berlin, DE · 2 hours ago                |  |
|  |    (14px, --meta)                                            |  |
|  |                                                              |  |
|  |    If this was not you, review your sessions.                |  |
|  |    [ Review sessions ]  (ghost)                              |  |
|  +-------------------------------------------------------------+  |
|  (--surface, --border, --radius-sm; the ● is decorative,         |
|   marked aria-hidden — the heading carries the meaning)          |  |
+-------------------------------------------------------------------+
```

States: **empty** — `EmptyState` with **"No notifications yet"** / **"Sign-ins from a new device will
appear here."**; **populated**; **loading**; **error** with retry.

**Exact copy**

- Alert heading: **"New device signed in"**
- Alert detail: **"{browser} on {platform} · {location} · {relative time}"**
- Alert body: **"If this was not you, review your sessions."**
- Alert action: **"Review sessions"** → `/settings/sessions`
- Empty title: **"No notifications yet"**
- Empty body: **"Sign-ins from a new device will appear here."**

Forbidden in this row (criterion AC6.1.3): no token, no credential, no full device fingerprint, no
IP address. Location is city and country code only.

---

## S11 — Provider outage (new shared state)

Not a route. One component rendered in place of the error slot on S1, S2, S3, S5 and S6, and as the
panel on S9.

```
+-------------------------------------------------------------+
|  !  Sign-in is unavailable                                  |
|     (--danger glyph + heading, 16px/500)                    |
|                                                             |
|  We cannot reach the sign-in service right now. If you are  |
|  already signed in, you can keep working until this page    |
|  asks you to sign in again.                                 |
|  (14px, --ink-2)                                            |
|                                                             |
|  [ Try again ]  (secondary)     (--meta) ref 4f2a-…         |
+-------------------------------------------------------------+
```

Behaviour: the retry button is disabled for 5 seconds after a failed retry and shows a countdown, so
a user cannot amplify the outage (criterion AC9.3.3). The message never names the provider, a
hostname, a status code, or any internal detail (criterion AC9.3.2).

**Exact copy**

- Title: **"Sign-in is unavailable"**
- Body: **"We cannot reach the sign-in service right now. If you are already signed in, you can keep working until this page asks you to sign in again."**
- Action: **"Try again"**
- Cooling down: **"Try again in {n}s"**

---

## S12 — Unsupported browser (new)

Rendered instead of the app shell when the required browser features are missing.

```
+-------------------------------------------------------------------+
|       +--------------------------------------------------+        |
|       |  This browser is not supported                   |        |
|       |  (Fraunces 25px 500)                             |        |
|       |                                                  |        |
|       |  Crypto Pocket Butler needs a current version of  |        |
|       |  Chrome, Edge, Firefox or Safari.                |        |
|       |                                                  |        |
|       |  [ Learn what works ]  (secondary)               |        |
|       +--------------------------------------------------+        |
+-------------------------------------------------------------------+
```

**Exact copy**

- Title: **"This browser is not supported"**
- Body: **"Crypto Pocket Butler needs a current version of Chrome, Edge, Firefox or Safari."**
- Action: **"Learn what works"**

---

## Responsive behaviour

| Screen | Phone (< 640px) | Tablet (640–1023px) | Desktop (≥ 1024px) |
|---|---|---|---|
| S1 | Single column; form card first, product voice hidden below; 12px gutters | Single column, 16px gutters | Two columns, 24px gutters |
| S2 | Full-width card | Centred 480px card | Card in the right column of the split |
| S3 | Centred, 16px page padding | Centred 480px | Centred 480px |
| S4 | Full-width form | Centred 560px | Centred 560px |
| S6 | QR stacks above the code input; step list becomes vertical | Two columns for the QR and instructions | Two columns, 24px gap |
| S7 | Row actions stack under the device line; the location wraps to its own line | Cards full-width in a single column | Cards in a single column, action right-aligned |
| S8 | Banner body truncates to the first sentence; action moves below the text | Banner single line + action | As drawn |
| S9 | Full-width card | Centred 560px | Centred 560px |
| S10 | Row meta wraps | As drawn | As drawn |

Touch targets: every interactive element ≥ 44×44 CSS px; 8px minimum spacing between adjacent
targets. The device row's "End session" is 44px tall on phone even though the label is 14px.

---

## Screens deliberately not designed here

- **Administrator session management** (acting on another user's sessions). The requirements place
  it on the admin surface in the support console, which already has its own screens; this intent's
  SPA work is self-service. This is recorded as a coverage note against story US5.1's review finding
  R-01 and is carried to Functional Design.
- **Support-assisted recovery internals.** S9 is the user-facing entry; the support console side is
  a separate surface described in the interaction spec by contract, not by mockup.
