# Design System Mapping — Users, Roles & Permissions

The mockups reuse the Parchment Ledger design system and add exactly three components: **RoleBadge**, **StatusPill**, and **GrantBanner**. Everything else uses existing tokens and components.

## Token mapping

### Colour

| Role in these screens | Token | Notes |
|---|---|---|
| App canvas | Background `#F5F4ED` | Never pure white |
| Cards, tables, panels, drawers | Surface `#FAF9F5` | Ivory |
| Selected row, warm callouts, grant banner body | Surface warm `#E8E6DC` | Warm sand |
| Headings and primary copy | Foreground `#141413` | |
| Secondary copy and links | Foreground 2 `#3D3D3A` | |
| Labels, support text | Muted `#5E5D59` | |
| Timestamps, table headers | Meta `#6F6E67` | Footnote scale only on warm surfaces |
| Card edges, input borders | Border `#F0EEE6` | |
| Row separators | Border soft `#E8E6DC` | |
| Primary action (Save, Request access, Continue) | Accent `#C96442` | One brand moment per screen |
| Success (Active, Synced) | Success `#15803D` | Always paired with a label |
| Warning (Disabled, expiring grant, pending deletion) | Warn `#A16207` | Always paired with a label |
| Danger (delete, revoked) | Danger `#B53333` | Written message, never colour alone |
| Keyboard focus ring | Focus blue `#3898EC` | The only cool colour |

### Typography

| Use | Token | Rule |
|---|---|---|
| Page titles (Settings, Delete your account) | Fraunces 500–600, 32–52px | Serif for titles only |
| Section headings | Fraunces 500, 25px | |
| Body, labels, controls | Instrument Sans 400–500, 14–16px | Body 16px minimum |
| Badge and pill labels | Instrument Sans 10px/500 uppercase, 0.12px tracking | Overline treatment |
| Countdown and dates | JetBrains Mono 400 | Tabular numerals, mono for figures only |
| Sentence case everywhere | — | All-caps only for 10px overline labels |

### Spacing, radius, elevation

- 8px scale (4–48px; 64/80 for section pacing); section rhythm 48px phone / 64px tablet / 96px desktop.
- Radii: buttons 8–12px, inputs 12px, cards 8px, callouts 12px.
- Elevation: Contained (1) for cards and panels; Ring (2) for interactive states; Whisper (3) only for the admin drawer and confirmation modals.

## Component reuse

| Screen element | Component | Change |
|---|---|---|
| Buttons (Save, Request, Approve, Continue) | Existing button set (primary/secondary/ghost) | None |
| Inputs (country, reason, reason-for-change) | Existing inputs with label above | None |
| Tables (metadata panel, admin list, grants) | Existing table: 44px rows, mono right-aligned numerics | None |
| Callouts (metadata-only reminder, what-is-kept) | Existing warm-sand callout | None |
| Empty/loading/error states | Existing empty-state and skeleton patterns, wrapped by `ScreenStatePanel` | None |
| Drawer (admin user record) | Existing modal/drawer pattern; focus trap, Escape, focus return | None |
| Role badge | **New — RoleBadge** | Pill, dot + label, tinted with `color-mix` |
| Status pill | **New — StatusPill** | Four labelled states with dot and text |
| Active-grant header | **New — GrantBanner** | Warm-sand panel, countdown in mono, End now action |

## The three additions

- **RoleBadge** — display only; role names are words (`Owner`, `Admin`, `Support`), never icons alone.
- **StatusPill** — display only; every state pairs its colour with a label and a dot.
- **GrantBanner** — feedback; announces state changes politely, updates the countdown at most once a minute, and never moves focus.

## Do's and don'ts applied

- **Do** lead with the number (countdown, date) and qualify it in text.
- **Do** pair every status colour with a word and a shape.
- **Do** use a missing-value em dash on any figure, never `0` or `$0.00`.
- **Don't** use terracotta as body text without weight and underline.
- **Don't** use heavy shadows, gradients, or glows.
- **Don't** animate numbers counting up, and respect `prefers-reduced-motion`.

## Responsive

- 12-column grid, 1200px max width, 24/16/12px gutters; app shell 64px header, 240px sidebar.
- SPA surfaces are mobile-first; support console and admin are desktop-first with card fallbacks at phone width.
- Touch targets ≥44×44px; 48×48px for primary actions on mobile.
