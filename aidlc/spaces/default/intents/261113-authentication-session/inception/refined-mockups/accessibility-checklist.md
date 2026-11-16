# Accessibility Checklist — Authentication & Session Management

Target: **WCAG 2.2 Level AA**, as required by requirements NFR4 and the design system's accessibility
baseline. Built 2026-11-13T15:23:55Z.

Every item is written so it can be checked in a pull request or a browser, not so it reads as
aspiration. Items already satisfied by an existing component carry "as shipped"; everything else is
work this intent must do.

Legend: `[x]` satisfied today · `[ ]` work required by this intent · `[—]` not applicable.

---

## 1. Perceivable

### 1.1 Text alternatives (WCAG 1.1.1)

- `[x]` The brand lockup already carries text; the emblem is decorative and does not need alternate text.
- `[ ]` **The TOTP QR code** must carry `role="img"` with an accessible label, and the **manual key
  in text beside it** is the real alternative. The QR is never the only way to enrol.
- `[ ]` The status glyphs (`●`, `○`, `!`, `◐`) are decorative in every case where a word sits beside
  them, and must be marked `aria-hidden="true"` so they are not announced twice. This is already the
  pattern in `DataStatusBadge` and `StatusPill`; the new session rows and banners must follow it.
- `[—]` No content images are introduced. The design system has no imagery, and this intent adds none.

### 1.2 Colour and contrast (WCAG 1.4.1, 1.4.3, 1.4.11)

- `[x]` Body text pairs clear 4.5:1 on `--bg` and `--surface`; the weakest is `--meta` on `--bg` at
  4.64:1.
- `[ ]` **Every status must pair colour with a word and a shape.** Concretely: "Current" (session row),
  "Not set up yet" / "On, added {date}" (MFA), "Sign-in is unavailable" (outage), "Not verified yet"
  (email). A red border or a green dot alone is a failure.
- `[ ]` **The destructive confirm button** uses `--danger` fill with `--accent-on` label; verify the
  pair measures ≥ 4.5:1 at 14px (it does at 5.9:1 on `--surface`; re-check on `--danger` fill).
- `[ ]` The `--warn` glyph inside the `--surface-warm` callout sits at 4.3:1 and is **permitted only
  because the words beside it carry the meaning**; do not use it as the sole indicator.
- `[ ]` Input error states carry a written message in addition to a crimson border (WCAG 1.4.1).
- `[ ]` Focus indicators use `--focus-ring` at 3:1 against the adjacent colours and are never removed
  by `outline: none` without a replacement.

### 1.3 Adaptable (WCAG 1.3.1 – 1.3.5)

- `[x]` The settings navigation is a real `<nav>` with a `<ul>` of links and `aria-current="page"` on the
  active one (as shipped in `SettingsNav`).
- `[ ]` **The session list must be a real list**: `<ul>` of `<li>`, each row's device name as a heading
  within the item, so a screen-reader user can jump between devices.
- `[ ]` The recovery codes must be a real `<ul>` of `<li>`, not a grid of styled spans.
- `[ ]` The "what happens next" steps on the recovery screen must be an ordered list (`<ol>`), because
  order carries meaning.
- `[ ]` Heading order must remain h1 → h2 → h3 on every screen. The MFA screen currently has one `h1`
  via `PageHeader`; the enrolment steps must use `h2`, not `h1`.
- `[ ]` The layout must survive 200% text zoom and 320px width with no horizontal page scroll; the
  session row's meta line wraps rather than truncating.

---

## 2. Operable

### 2.1 Keyboard (WCAG 2.1.1, 2.1.2, 2.4.3, 2.4.7)

- `[ ]` **The whole sign-in form must be completable by keyboard**: email, password, submit, and the
  magic-link switch — in that reading order.
- `[ ]` **The code input must accept a pasted six-digit code** from a password manager, and must
  advance on entry without requiring a separate submit.
- `[x]` **Dialogs already trap focus and close on Escape** (`ConfirmDialog`, as shipped), restore focus
  to the trigger, and enter focus on Cancel.
- `[ ]` **After a session row is removed**, focus moves to the next row's action, or to the list
  heading when none remains. Focus must never fall to `<body>`.
- `[ ]` **On a route change that redirects** (limited session → `/settings/mfa`), focus moves to the
  page heading so the new context is announced.
- `[ ]` The magic-link sent state takes focus on arrival, because it replaces the form the user was in.

### 2.2 Pointer and target size (WCAG 2.5.8 — AA in 2.2)

- `[ ]` Every interactive target is at least **24×24 CSS px** (2.5.8 minimum) and this project's own
  standard raises it to **44×44** on touch; the session row action is 44px tall on phone.
- `[ ]` Adjacent targets keep ≥ 8px spacing, including the three acceptance checkboxes in sign-up mode.

### 2.3 Timing (WCAG 2.2.1, 2.2.2)

- `[ ]` **The magic-link cooldown and the outage retry cooldown must not be announced per second.**
  A screen reader hears the message once and the button's changed label, not a stream of numbers.
- `[ ]` No authentication step has a time limit the user cannot extend, except the provider's own
  link expiry, which is stated in words on the screen.

### 2.4 Motion (WCAG 2.3.3)

- `[ ]` Row removal and dialog entry respect `prefers-reduced-motion`; with it set, the row is removed
  with no animation and no information is lost.

---

## 3. Understandable

### 3.1 Labels and instructions (WCAG 3.3.2)

- `[x]` The existing form fields use visible labels above the input, not placeholders.
- `[ ]` **The new code input, country select, and recovery email field** must each have a visible
  label; no placeholder-only field is permitted.
- `[ ]` The three acceptance checkboxes must each have their own visible label, and the group must be
  a `<fieldset>` with a `<legend>` ("Before you start"), so the relationship is announced.
- `[ ]` The device label must be included in each session action's accessible name ("End session:
  Safari on iOS"), because "End session" repeated four times is ambiguous when read out of context.

### 3.2 Error identification and suggestion (WCAG 3.3.1, 3.3.3)

- `[ ]` **Every error names the field and the fix**, in the copy recorded in `mockups.md`:
  email format, password length, unanswered acceptance items, wrong TOTP code, expired link.
- `[ ]` **The bad-credentials message must not disclose whether an account exists** — this is both a
  security requirement (FR-AUTH-11) and an understandable-error requirement. The recorded copy is
  identical for both causes.
- `[ ]` Errors land in an `aria-live` region and are associated with the field via
  `aria-describedby`, so the error is not conveyed by position alone.
- `[ ]` The incorrect-input state must **preserve what the user typed** except the password, which is
  cleared for security.

### 3.3 Consistency and predictability (WCAG 3.2.3, 3.2.4)

- `[ ]` The same destructive action uses the same label everywhere: "End session" for one,
  "Sign out everywhere else" for all.
- `[ ]` Nothing submits on focus change or on Enter inside a checkbox; submission is an explicit
  button press (WCAG 3.2.2).
- `[ ]` The limited-session banner appears in the same position on every allowed screen.

---

## 4. Robust

### 4.1 Markup and roles (WCAG 4.1.2, 4.1.3)

- `[ ]` **The limited-session banner is `role="status"`, not `role="alert"`** — it is context, not a
  failure, and an alert would interrupt every navigation.
- `[ ]` **The outage panel is `role="alert"`** — it is a failure the user did not cause.
- `[ ]` Every dialog has `role="dialog"`, `aria-modal="true"`, and `aria-labelledby` pointing at its
  title.
- `[ ]` Status messages that appear after an action (session ended, codes shown, link sent) are
  announced through a live region, not only rendered.
- `[ ]` No component introduces a duplicate `id`; the dialog title id is the only fixed id and it is
  unique per dialog instance.

### 4.2 Assistive-technology behaviour worth testing

- `[ ]` VoiceOver on macOS: sign in, reach the MFA screen from a limited session, and end a session
  without sighted assistance.
- `[ ]` NVDA on Windows: the same three journeys.
- `[ ]` The TOTP code field is announced with its expected format, and a paste of six digits is
  accepted.

---

## 5. Screen-by-screen checks

| Screen | Must verify |
|---|---|
| **S1 Sign-in** | Labels above inputs; error messages associated; no placeholder-only field; single terracotta moment; keyboard-completable; the sign-up mode's three checkboxes are a labelled `<fieldset>` |
| **S2 Magic link sent** | The state is announced on arrival; the countdown is not announced per second; both actions are reachable |
| **S3 Callback** | The loading state is `role="status"`; the expired and error states take focus and are `role="alert"`; the glyph is paired with words |
| **S4 Sign-up gates** | As shipped, plus: the blocked-jurisdiction notice is `role="alert"` and names the country |
| **S5 Verify email** | The resend cooldown is announced once; "Not verified yet" carries a glyph and a word |
| **S6 MFA** | QR has an accessible name and the manual key is the alternative; the code input is labelled; the one-time warning is read before the codes; the recovery-code list is a real list |
| **S7 Sessions** | Real list markup; current session announced as such; each action names its device; focus moves on removal; the confirmation dialogs trap focus; the residual-window disclosure is readable text, not a tooltip |
| **S8 Limited banner** | `role="status"`, not dismissible, no focus theft, one sentence pair when read linearly |
| **S9 Recovery** | The received state is identical for every input and is announced; the ordered steps are an `<ol>`; the email field has a visible label |
| **S10 Notifications** | The alert heading carries the meaning; the decorative glyph is `aria-hidden`; no token, credential, or full fingerprint anywhere |
| **S11 Outage** | `role="alert"`; retry cooldown announced once; no internal detail in the copy |
| **S12 Unsupported browser** | Rendered as a plain page with a heading and one action; readable with no stylesheet |

---

## 6. What is explicitly out of scope

- **Provider-hosted screens.** Supabase Auth's own confirmation and error pages are not this
  product's surface and are not assessed here.
- **The support console's recovery screens.** A separate package with its own accessibility posture;
  this intent specifies its contract, not its markup.
- **Formal WCAG conformance audit.** This checklist is a working instrument, not a certification. A
  formal audit would need real assistive-technology testing sessions, which this stage cannot run.

---

## 7. Verification method

- Automated: `jest-axe` on every new component's test, following the existing pattern (36 files
  already assert accessibility in `web/`).
- Manual: keyboard-only pass over S1, S6, S7 and S9; screen-reader pass over S6 and S7; 200% zoom and
  320px width pass over S7 and S1.
- The browser-journey suite (`yarn e2e`) covers sign-in and settings journeys and must stay green for
  any change touching the auth surface (team practice).
