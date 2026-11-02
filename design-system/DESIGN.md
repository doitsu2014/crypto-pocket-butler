# Gilded Ledger

> Category: Fintech · Luxury
> The design system for **Crypto Pocket Butler** — a read-only portfolio
> aggregator with a private-banking temperament. Black lacquer, ivory type,
> one champagne-gold accent. Numbers lead; chrome recedes.

## Visual Theme & Atmosphere

A private vault, not a trading terminal. Black-lacquer grounds, ivory
Didone serif display, hairline bronze borders, and one champagne-gold
accent used like gilding: sparingly, and only where importance lives. The
interface should feel unhurried, discreet, and auditable — every figure
carries a time and a state, and nothing decorative competes with the data.

Three principles govern every screen:

1. **Say the number, then the context.** A value is never naked — it carries
   an as-of time and a source state.
2. **Quiet chrome, loud data.** Lacquer-dark surfaces, 1px hairline borders,
   no glow, no gradients. Hierarchy comes from type and space.
3. **Color confirms, words decide.** Status always has a label. Green, amber,
   and red never carry meaning alone — and gold is never decoration.

## Brand Mark

- **Emblem (`logo-mark.svg`):** the "PB" monogram (Pocket Butler) in the display
  serif, set in champagne gold inside a double gold ring on a lacquer tile with
  a hairline edge — an engraved seal, not a badge. It doubles as the favicon.
- **Lockup (`logo.svg`):** emblem + "Gilded Ledger" wordmark in the display
  serif, separated by a hairline, with "CRYPTO POCKET BUTLER · DESIGN SYSTEM"
  tracked in uppercase sans below.
- The shipped files carry the token values baked in (`--accent`, `--fg`,
  `--bg`, `--border`); regenerate rather than recolor.
- Gold on lacquer only — never on gold, ivory, or photography.
- Clear space around the emblem equals the ring's radius; the monogram never
  renders below 24px.
- Never redraw, recolor, rotate, or add effects to the mark. It is gilded,
  not glowing.

## Color Palette & Roles

- **Background:** `#080706` — black-lacquer app canvas
- **Surface:** `#151310` — cards, tables, modals
- **Surface warm:** `#241E14` — recommendation panels, selected rows, insets
- **Foreground:** `#FFF8EA` — headings and primary copy (ivory, never white)
- **Foreground 2:** `#D8CDB7` — secondary copy
- **Muted:** `#9F927C` — labels, helper text
- **Meta:** `#B3A58C` — timestamps, footnotes, table headers
- **Border:** `#3A3020` — card edges, input borders
- **Border soft:** `#282217` — row separators only
- **Accent:** `#C6A15B` (champagne gold) — primary CTA, links, active tab, one
  gilded detail per screen
- **Accent on:** `#080706` — label on gold fill
- **Success:** `#6FAE7A`, **Warn:** `#D8A94F`, **Danger:** `#E06A60`
- **Charts:** `#C6A15B`, `#D8CDB7`, `#7FB0A4`, `#A8875A`, `#8C9DB0` — fixed
  order (gold, ivory, sage, bronze, slate), largest series first

Every text pairing clears WCAG AA on `--bg`, `--surface`, and
`--surface-warm`; the weakest pair (`--muted` on warm) measures 5.4:1 and
`--meta` on `--bg` measures 8.3:1. Gold reads as a link or an action, never
as a wash: if more than ~5% of a screen is gold, the design is wrong. Green
means synced / within guardrail / up; it is never decorative.

## Typography Rules

- **Display / page titles:** `"Playfair Display"` (Didone), weight 500–600
- **Body / interface:** `"Jost"` (geometric sans), weight 400–500
- **Figures:** `"IBM Plex Mono"`, weight 400–500, `font-variant-numeric:
  tabular-nums`
- Scale (px): 12 · 14 · 16 · 20 · 24 · 32 · 44 · 60
- Line height: 1.55 body, 1.12 display. Tracking −0.015em on display sizes,
  +0.06em on 12px uppercase labels.
- Body copy 16px minimum, 60–75 characters per line.
- Sentence case everywhere. All-caps only for 12px overline labels.
- The serif is for the product name, page titles, and section headings. Never
  set a table cell or a number in serif — luxury lives in the restraint.

**Financial figures.** Use mono with tabular numerals for prices, balances,
weights, and drift. Right-align numeric columns; keep the unit on the same line
as the number. USD amounts show two decimals; crypto quantities show up to six
significant digits. A missing value is an em dash (`—`), never `0` or `$0.00`.
Use "pp" for percentage points, "%" for relative change, and never mix them.

## Component Stylings

- **Buttons:** 40px tall, 8px radius, 16px inline padding, 14px/600 sans. Primary
  = gold fill with `--accent-on` label. Secondary = surface fill + 1px border.
  Ghost = no fill, gold label. Destructive = danger fill, confirmation required.
  Hover deepens the fill, never lightens the label. Disabled at 45% opacity.
- **Inputs:** 40px tall, 8px radius, 1px border, 12px inline padding, 16px text
  (prevents mobile zoom). Label above at 13px/500; hint below at 12px muted.
  Focus = gold border + `--focus-ring`. Error adds a danger border and a
  written message; never color alone.
- **Cards:** `--surface` on `--bg`, 1px border, 14px radius, 24px padding, no
  shadow.
- **KPI tiles:** overline label, mono value at 28–32px in ivory, delta with
  arrow and signed value, meta line with as-of time.
- **Badges:** pill, 12px/500, tinted background derived with `color-mix`, plus a
  dot and a text label.
- **Tables:** 44px rows, soft separators, 12px uppercase headers in meta,
  numeric columns right-aligned in mono.
- **Tabs:** 2px gold underline on the active tab; inactive labels muted; no
  pill background.
- **Callouts:** surface-warm panel with a bordered icon tile, 14px radius, and a
  written heading. No colored left-border strips.
- **Empty states:** centered, one icon, serif title at 20px, one sentence of
  explanation, one primary action.

## Layout Principles

- 12-column grid, 1240px max width, 24px gutters desktop / 16px tablet / 12px
  phone.
- App shell: 64px header, 240px sidebar, content max 1240px. Sticky chrome
  reserves matching padding on the content it covers.
- Spacing uses the 4px scale only. Section rhythm: 96px desktop, 64px tablet,
  40px phone — luxury breathes; when in doubt, add space.
- Use whitespace as the primary separator. Hairline rules only between
  unrelated regions or table rows.
- Keep primary content, actions, and status within the central 70–80% of the
  viewport, at least 48px from edges on desktop.

## Depth & Elevation

Two real levels, plus a ring:

- **Flat (0):** default for everything, including cards and inputs.
- **Ring (1):** `0 0 0 1px var(--border)` where a border would shift layout.
- **Raised (2):** dropdowns, modals, popovers only — a deep black lacquer
  shadow (`0 1px 2px` and `0 16px 48px` at 50–60% black), never a light glow.

No glassmorphism, no gradient washes, no glow, no neon.

## Data & Financial Conventions

- Every portfolio view states **as of** time and data status (live, delayed,
  stale, unpriced).
- Unpriced assets are labelled **Unpriced**, excluded from the total, and
  disclosed next to it. They are never silently valued at zero.
- Positive and negative changes use sign + arrow + color + label.
- Drift is reported in percentage points against the configured target.
  Guardrail thresholds come from the portfolio, not from this design system; the
  UI only renders the three states: within, outside, breached.
- Charts are built in code (SVG/CSS), not images. Series order is fixed; the
  legend always shows the label and value. Do not use semantic colors as chart
  series.
- Sample data is always labelled "Sample data — illustrative only."

## Motion

- Micro-states (hover, focus): 140ms. General state changes: 200ms. Exits run
  at roughly 70% of the entrance duration.
- Easing: `cubic-bezier(0.2, 0, 0, 1)`. Enter with ease-out, exit with ease-in.
- List items may stagger 40ms; never animate numbers counting up.
- Respect `prefers-reduced-motion` by removing transitions; never animate
  layout position for meaning.

## Do's and Don'ts

- ✅ Lead with the number, then qualify it.
- ✅ Let whitespace and the type scale carry hierarchy.
- ✅ Pair every status color with a word and a shape.
- ✅ Use one primary action and one gilded detail per view.
- ❌ No gradients, glows, glass, or neon "crypto" styling — gold is not a
  license for shine.
- ❌ No green/red without a sign and a label.
- ❌ No icon beside every heading or emoji as an icon.
- ❌ No invented metrics, percentages, or balances — label samples.
- ❌ No shadow on cards, inputs, buttons, or tables.
- ❌ No gold text on gold-tinted surfaces; keep the accent on lacquer.

## Responsive Behavior

- **Desktop ≥ 1024px:** 12-column grid, sidebar visible, 24px gutters.
- **Tablet 640–1023px:** 8-column grid, 16px gutters, sidebar collapses to a
  top navigation.
- **Phone < 640px:** 4-column grid, 12px gutters, KPI tiles stack, tables scroll
  horizontally inside their own container with a visible edge fade or scrollbar.
- Financial tables never wrap a number onto a second line; allow horizontal
  scroll instead.

## Accessibility Baseline

- Body text ≥ 4.5:1; large text, icons, and essential graphics ≥ 3:1.
- Every interactive element has a visible focus state using `--focus-ring` and
  an accessible name.
- Icons are decorative unless they carry meaning; meaningful icons get a text
  label or `aria-label`.
- Tables use real `<table>`, `<th scope>`, and a caption.
- Motion respects reduced-motion; layout survives 200% text zoom.

## Agent Prompt Guide

- Paste the `:root` block from `tokens.css` verbatim, then use `var()` only. Do
  not invent hex values, font families, or spacing steps.
- Prefer the components above; if a new one is needed, build it from tokens and
  add it to the guide.
- Use mono for every figure and right-align numeric columns.
- Show data status and as-of time wherever a value appears.
- When in doubt, remove chrome and add space. When in doubt about gold, use
  less.
