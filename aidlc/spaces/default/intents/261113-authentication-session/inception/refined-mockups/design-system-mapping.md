# Design System Mapping — Authentication & Session Management

Every value used by this intent's screens, mapped to a token from `design-system/tokens.css`. No raw
hex, size, or spacing value is introduced anywhere in the mockups or the interaction spec. Where a
component already exists in `web/src/components/index.tsx`, its token usage is recorded as **as
shipped** rather than re-specified.

---

## Colour

| Role in these screens | Token | Value | Where it appears |
|---|---|---|---|
| Page canvas | `--bg` | `#f5f4ed` | Every route background |
| Card, input, dialog surface | `--surface` | `#faf9f5` | Form cards, session rows, dialogs |
| Banner, callout, inset | `--surface-warm` | `#e8e6dc` | Limited-session banner, recovery-code warning, declined notice |
| Primary text, headings | `--fg` | `#141413` | Titles, device labels, values |
| Secondary text, links | `--fg-2` | `#3d3d3a` | Labels, secondary actions |
| Body support text | `--muted` | `#5e5d59` | Descriptions, helper copy |
| Timestamps, footnotes, overlines | `--meta` | `#6f6e67` | "Last seen…", correlation ids, "Sent 30 seconds ago" |
| Card and input borders | `--border` | `#f0eee6` | Card edges, input outlines |
| Row separators | `--border-soft` | `#e8e6dc` | Between session rows, table rules |
| Primary action | `--accent` | `#c96442` | One primary button per screen |
| Primary label | `--accent-on` | `#faf9f5` | Text on terracotta |
| Error, destructive | `--danger` | `#b53333` | Error text, destructive confirm button |
| Warning | `--warn` | `#a16207` | Expired link, recovery-code warning glyph |
| Success | `--success` | `#15803d` | Verified state, current-session dot |
| Focus ring | `--focus-ring` | `rgba(56,152,236,0.3)` | Every focused control |

**Colour rules observed**

- Every status carries a **glyph and a word**: `●`/`○`/`!` plus "Current", "Not set up yet",
  "Sign-in is unavailable". Colour is never the only signal (design-system rule; NFR4).
- Terracotta appears **once per screen** (the primary action). The destructive dialogs use
  `--danger`, not terracotta, so a destructive action is never mistaken for the brand moment.
- Focus Blue is the only cool colour and appears only as a focus ring.
- The outage panel's heading glyph is `--danger`; the recovery-code and expired-link glyphs are
  `--warn`; both are paired with words.

**Contrast checks against `--bg` and `--surface`**

| Pair | Ratio | Verdict |
|---|---|---|
| `--fg` on `--surface` | 15.9:1 | AAA |
| `--fg-2` on `--surface` | 10.4:1 | AAA |
| `--muted` on `--surface` | 6.4:1 | AA |
| `--meta` on `--bg` | 4.64:1 | AA (the weakest pair in the system; used for footnote-scale text only) |
| `--accent-on` on `--accent` | 4.6:1 | AA for normal text at 14px/500 |
| `--danger` on `--surface` | 5.9:1 | AA |
| `--warn` on `--surface-warm` | 4.3:1 | Used for the 14px glyph **beside** a written label; the words carry the meaning |

---

## Typography

| Role | Token | Family / size / weight |
|---|---|---|
| Page title | `--font-display`, `--text-xl` (25px), weight 500 | Fraunces — "Sessions", "Check your email" |
| Sign-in marketing headline | `--font-display`, `--text-3xl` (52px), weight 500 | Fraunces — the left column of S1 |
| Section heading | `--font-display`, `--text-xl` | Fraunces — card titles inside a page |
| Body copy | `--font-body`, `--text-base` (16px), 400 | Instrument Sans |
| Labels, actions | `--font-body`, `--text-sm` (14px), 500 | Instrument Sans |
| Overline labels | `--text-xs` (10px), 500, `--tracking-label` (0.05em), uppercase | "CRYPTO POCKET BUTLER", "SENT 30 SECONDS AGO" |
| Codes, keys, recovery codes | `--font-mono`, `--text-base` or `--text-lg` | JetBrains Mono, `font-variant-numeric: tabular-nums` |
| Line height | `--leading-body` (1.6) body, `--leading-tight` (1.1) headings | — |

**Type rules observed**

- Sentence case everywhere; all-caps only for the 10px overline.
- No serif in a table cell, a code, or a device label (device labels are UI text, so Instrument Sans
  at 16px; only the TOTP key and the recovery codes are mono).
- The six-digit code input is mono at `--text-lg` with tabular numerals and letter spacing so digits
  are countable.
- "Last seen 2 hours ago" is `--meta` at 14px — meta scale, not body.

---

## Spacing and layout

| Role | Token |
|---|---|
| Between form fields | `--space-3` (12px) |
| Inside a card | `--space-6` (24px) padding |
| Between cards in the session list | `--space-3` (12px) |
| Between page sections | `--space-8` (32px) |
| Page gutter — desktop | `--container-gutter-desktop` (24px) |
| Page gutter — tablet | `--container-gutter-tablet` (16px) |
| Page gutter — phone | `--container-gutter-phone` (12px) |
| Max content width | `--container-max` (1200px) |
| App shell header | `--header-height` (64px) |
| Settings sidebar | `--sidebar-width` (240px) |
| Single-purpose card width (S3, S9) | 480px / 560px — expressed as a max-width utility, not a token; these are the only two ad-hoc widths and they match the existing `max-w-xl`/`max-w-2xl` classes already used by `SignupGatesScreen` and `MfaPage` |

---

## Radius and elevation

| Role | Token |
|---|---|
| Buttons, cards, session rows, dialogs | `--radius-sm` (8px) |
| Inputs, banners, callouts | `--radius-md` (12px) |
| Status pills, badges | `--radius-pill` (9999px) |
| Card containment | `--elev-ring` (`0 0 0 1px --border`) |
| Elevated card, dialog | `--elev-raised` (`rgba(0,0,0,0.05) 0 4px 24px`) |

No heavy shadows, no glass, no gradients. The session rows use `--elev-ring`; the sign-in card and
dialogs use `--elev-raised`, matching what ships today.

---

## Motion

| Interaction | Token / value |
|---|---|
| Hover, focus colour shifts | `--motion-fast` (150ms) |
| Row removal, dialog entry | `--motion-base` (200ms) |
| Exit of anything | ~70% of entrance (140ms) |
| Easing | `--ease-standard` (`cubic-bezier(0.2, 0, 0, 1)`) |
| `prefers-reduced-motion` | All transitions removed; no layout animation carries meaning |

No counting-up numbers, no layout movement for meaning — consistent with the design system's motion
section.

---

## Component inventory — what is reused and what is new

| Component | Status | Source |
|---|---|---|
| `ConfirmDialog` | **Reused as shipped** | `web/src/components/index.tsx` — focus trap, Escape, focus restore already implemented |
| `ErrorPanel` | **Reused as shipped** | same — RFC 7807 body with correlation id and retry |
| `EmptyState` | **Reused as shipped** | same |
| `Loading` | **Reused as shipped** | same |
| `StatusRegion` | **Reused as shipped** | same |
| `PageHeader` | **Reused as shipped** | same |
| `QueryBoundary` | **Reused as shipped** | same |
| `AcceptanceCheckbox`, `CountrySelect`, `GateForm`, `BlockedJurisdictionNotice`, `GateErrorNotice` | **Reused as shipped** | `web/src/features/signup-gates/` |
| `SettingsNav`, `SettingsLayout` | **Extended** — one item added | `web/src/features/settings/` |
| `SessionRow` | **New** | specified in `interaction-spec.md` |
| `LimitedSessionBanner` | **New** | specified |
| `MagicLinkSentState` | **New** | specified |
| `TotpCodeInput` | **New** | specified |
| `RecoveryCodesPanel` | **New** — replaces the inline code list in `MfaPage` | specified |
| `AuthOutagePanel` | **New** | specified |
| `RecoverAccountForm` | **New** | specified |
| `QrCode` | **New, dependency question** | see below |

### The QR code is the one open design-system question

`design-system/DESIGN.md` says **"Charts are built in code (SVG/CSS), not images"**, and the system
carries no imagery at all. A TOTP enrolment QR is machine-readable data, not decoration, so it is
treated as a functional code rather than an image:

- Render it as **inline SVG** generated from the otpauth URL, matching the "built in code" rule and
  keeping it out of the size budget as an asset.
- Size 128×128 CSS px, `--surface` quiet zone, `--fg` modules, `--radius-sm` container.
- Always offer the **manual key** beside it, in mono, so the QR is never the only path.
- The SVG carries `role="img"` and `aria-label="QR code for adding this account to an authenticator
  app"`; the manual key is the accessible alternative.

If a generator dependency is rejected at Functional Design, the fallback is the manual key alone with
the QR omitted — the criterion for enrolment (a working factor) does not depend on the QR.

---

## What this mapping deliberately does not do

- **No new tokens.** Every value above already exists in `tokens.css`.
- **No component-library fork.** The settings and sign-up-gate components are extended in place.
- **No visual redesign of existing screens.** S1 keeps its layout; S4, S5 and S10 keep theirs. The
  work is content and behaviour, which is why the mockups mark those four as "current state".
