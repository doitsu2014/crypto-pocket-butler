# Parchment Ledger

> Category: Fintech · Warm editorial
> The design system for **Crypto Pocket Butler**, themed on the Claude
> (Anthropic) design language — a literary salon reimagined as a financial
> product: warm parchment, terracotta restraint, and the gravitas of a
> published title. Numbers lead; chrome recedes.

## Visual Theme & Atmosphere

A private reading room, not a trading terminal. A parchment-toned canvas
that evokes quality paper rather than a screen, ivory cards floating on it,
exclusively warm-toned neutrals (every gray carries a yellow-brown
undertone — no cool blue-grays anywhere), and terracotta as the single
chromatic accent, used like a bookmark: one per page, only where importance
lives. The interface should feel unhurried, considered, and auditable —
every figure carries a time and a state.

Three principles govern every screen:

1. **Say the number, then the context.** A value is never naked — it carries
   an as-of time and a source state.
2. **Quiet chrome, loud data.** Parchment surfaces, cream hairline borders,
   whisper-soft depth. Hierarchy comes from the type scale and space.
3. **Color confirms, words decide.** Status always has a label. Green, amber,
   and red never carry meaning alone — and terracotta is never decoration.

## Color Palette & Roles

- **Background:** `#F5F4ED` — parchment app canvas; never pure white
- **Surface:** `#FAF9F5` — ivory cards, tables, modals
- **Surface warm:** `#E8E6DC` — warm sand: recommendation panels, selected
  rows, insets, prominent interactive surfaces
- **Foreground:** `#141413` — headings and primary copy (warm near-black)
- **Foreground 2:** `#3D3D3A` — secondary copy and text links
- **Muted:** `#5E5D59` — olive gray: body support text, labels
- **Meta:** `#6F6E67` — stone gray: timestamps, footnotes, table headers
- **Border:** `#F0EEE6` — cream card edges, input borders
- **Border soft:** `#E8E6DC` — row separators and dividers
- **Accent:** `#C96442` (terracotta) — primary CTA, the active tab, one
  brand moment per screen
- **Accent on:** `#FAF9F5` — ivory label on terracotta fill
- **Success:** `#15803D`, **Warn:** `#A16207`, **Danger:** `#B53333` (error
  crimson — warm, serious without alarming)
- **Charts:** `#C96442`, `#5E5D59`, `#67764F`, `#8A6D3B`, `#6F6E67` —
  terracotta, olive, muted green, ochre, stone; fixed order, largest series
  first
- **Focus blue:** `#3898EC` — the only cool color in the system, reserved
  exclusively for keyboard focus rings

Every body-text pairing clears WCAG AA on `--bg` and `--surface`; the
weakest pair (`--meta` on `--bg`) measures 4.64:1. `--meta` on
`--surface-warm` (4.09:1) is for footnote-scale text only — prefer
`--muted` inside warm panels. Terracotta as link or ghost-button text
measures 3.54:1 — brand-supplied value; pair it with weight and underline,
and prefer secondary buttons for long labels. Green means synced / within
guardrail / up; it is never decorative.

## Typography Rules

- **Display / page titles:** `"Fraunces"` — a modern soft serif — weight
  500–600 (Georgia fallback), loaded from Google Fonts
- **Body / interface:** `"Instrument Sans"`, weight 400–500 (system
  fallback), loaded from Google Fonts
- **Figures:** `"JetBrains Mono"`, weight 400–500,
  `font-variant-numeric: tabular-nums`, loaded from Google Fonts
- Scale (px): 10 · 14 · 16 · 20 · 25 · 32 · 52 · 64
- Line height: 1.6 body — closer to a book than a dashboard; 1.1 headings.
  No compression tracking on serif display.
- Body copy 16px minimum, 60–75 characters per line; 1.6 line-height
  throughout.
- Sentence case everywhere. All-caps only for 10px overline labels with
  0.5px letter-spacing.
- The serif is for the product name, page titles, and section headings.
  Never set a table cell or a number in serif. Mono is strictly for figures
  and code.

**Financial figures.** Use mono with tabular numerals for prices, balances,
weights, and drift. Right-align numeric columns; keep the unit on the same
line as the number. USD amounts show two decimals; crypto quantities show
up to six significant digits. A missing value is an em dash (`—`), never
`0` or `$0.00`. Use "pp" for percentage points, "%" for relative change,
and never mix them.

## Component Stylings

- **Buttons:** 8–12px radius, sans 14px/500. Primary = terracotta fill with
  ivory label and a terracotta ring shadow. Secondary = warm sand fill with
  charcoal label and a `#D1CFC5` ring. Ghost = no fill, terracotta label.
  Hover shifts the fill darker, never lightens the label. Disabled at 45%
  opacity.
- **Inputs:** 12px radius, cream border, ivory fill, 14px text. Label above
  at 14px/500. Focus = Focus Blue border + `--focus-ring` — the only cool
  color moment. Error adds a crimson border and a written message; never
  color alone.
- **Cards:** ivory on parchment, 1px cream border, 8px radius, 24px
  padding, whisper shadow (`rgba(0,0,0,0.05) 0 4px 24px`).
- **KPI tiles:** overline label, mono value at 25–32px, delta with arrow and
  signed value, meta line with as-of time.
- **Badges:** pill, 10px/500 uppercase with 0.12px tracking, tinted
  background derived with `color-mix`, plus a dot and a text label.
- **Tables:** 44px rows, soft separators, uppercase headers in meta,
  numeric columns right-aligned in mono.
- **Tabs:** terracotta underline on the active tab; inactive labels muted;
  no pill background.
- **Callouts:** warm sand panel with a cream border, 12px radius, and a
  written heading. No colored left-border strips.
- **Empty states:** centered, one icon, serif title at 25px, one sentence
  of explanation, one primary action.

## Layout Principles

- 12-column grid, 1200px max width, 24px gutters desktop / 16px tablet /
  12px phone.
- App shell: 64px header, 240px sidebar, content max 1200px. Sticky chrome
  reserves matching padding on the content it covers.
- Spacing uses the 8px scale (4–48px, extended to 64/80 for section
  pacing). Section rhythm: 96px desktop, 64px tablet, 48px phone —
  editorial pacing, each section breathes like a magazine spread.
- Use whitespace as the primary separator. Hairline rules only between
  unrelated regions or table rows.
- Keep primary content, actions, and status within the central 70–80% of
  the viewport, at least 48px from edges on desktop.

## Depth & Elevation

Depth comes from warm surfaces and light, not from shadow:

- **Flat (0):** parchment background, inline text.
- **Contained (1):** `1px solid var(--border)` — standard cards and sections.
- **Ring (2):** `0 0 0 1px` ring shadows in warm grays — interactive states;
  a shadow pretending to be a border.
- **Whisper (3):** `rgba(0,0,0,0.05) 0 4px 24px` — elevated cards, modals,
  popovers only.

No heavy drop shadows, no glassmorphism, no gradient washes, no glow.

## Data & Financial Conventions

- Every portfolio view states **as of** time and data status (live, delayed,
  stale, unpriced).
- Unpriced assets are labelled **Unpriced**, excluded from the total, and
  disclosed next to it. They are never silently valued at zero.
- Positive and negative changes use sign + arrow + color + label.
- Drift is reported in percentage points against the configured target.
  Guardrail thresholds come from the portfolio, not from this design system;
  the UI only renders the three states: within, outside, breached.
- Charts are built in code (SVG/CSS), not images. Series order is fixed; the
  legend always shows the label and value. Do not use semantic colors as
  chart series.
- Sample data is always labelled "Sample data — illustrative only."

## Motion

- Micro-states (hover, focus): 150ms. General state changes: 200ms. Exits
  run at roughly 70% of the entrance duration.
- Easing: `cubic-bezier(0.2, 0, 0, 1)`. Enter with ease-out, exit with
  ease-in. The editorial warmth does not need animation energy — calm.
- List items may stagger 40ms; never animate numbers counting up.
- Respect `prefers-reduced-motion` by removing transitions; never animate
  layout position for meaning.

## Do's and Don'ts

- ✅ Lead with the number, then qualify it.
- ✅ Let whitespace and the serif scale carry hierarchy.
- ✅ Pair every status color with a word and a shape.
- ✅ Use one primary action and one terracotta moment per view.
- ✅ Keep every neutral warm-toned — yellow-brown undertones only.
- ❌ No cool blue-grays anywhere except the focus ring.
- ❌ No gradients, glows, glass, or neon "crypto" styling.
- ❌ No bold serif beyond weight 600, no sans headlines.
- ❌ No sharp corners (< 6px) on buttons or cards — softness is the identity.
- ❌ No pure white page background — parchment or ivory, always warmer.
- ❌ No green/red without a sign and a label.
- ❌ No invented metrics, percentages, or balances — label samples.

## Responsive Behavior

- **Desktop ≥ 1024px:** 12-column grid, sidebar visible, 24px gutters.
- **Tablet 640–1023px:** 2-column grids, 16px gutters, sidebar collapses to
  a top navigation.
- **Phone < 640px:** single column, 12px gutters, KPI tiles stack, tables
  scroll horizontally inside their own container with a visible edge fade
  or scrollbar.
- Financial tables never wrap a number onto a second line; allow horizontal
  scroll instead.

## Accessibility Baseline

- Body text ≥ 4.5:1; large text, icons, and essential graphics ≥ 3:1.
- Every interactive element has a visible focus state using `--focus-ring`
  (Focus Blue) and an accessible name.
- Icons are decorative unless they carry meaning; meaningful icons get a
  text label or `aria-label`.
- Tables use real `<table>`, `<th scope>`, and a caption.
- Motion respects reduced-motion; layout survives 200% text zoom.

## Brand Mark

- **Emblem (`logo-mark.svg`):** the butler's pocket watch — a terracotta
  double-ring case, the chain bow and stem at twelve o'clock, two dial
  ticks at three and nine, and the "PB" monogram set on the dial in
  Fraunces, on an ivory tile with a cream hairline edge. It doubles as the
  favicon.
- **Lockup (`logo.svg`):** emblem + "Parchment Ledger" wordmark in Fraunces,
  separated by a warm hairline, with "CRYPTO POCKET BUTLER · DESIGN SYSTEM"
  tracked in uppercase Instrument Sans below.
- The shipped files carry the token values baked in (`--accent`, `--fg`,
  `--surface`, `--border`); regenerate rather than recolor.
- Terracotta on parchment only — never on terracotta, ivory fills, or
  photography.
- Clear space around the emblem equals the ring's radius; the monogram
  never renders below 24px.
- Never redraw, recolor, rotate, or add effects to the mark.

## Agent Prompt Guide

- Paste the `:root` block from `tokens.css` verbatim, then use `var()` only.
  Do not invent hex values, font families, or spacing steps.
- Prefer the components above; if a new one is needed, build it from tokens
  and add it to the guide.
- Use mono for every figure and right-align numeric columns.
- Show data status and as-of time wherever a value appears.
- When in doubt, remove chrome and add space. When in doubt about
  terracotta, use less.
