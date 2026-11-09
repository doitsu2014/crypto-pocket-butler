# Design System Mapping — Parchment Ledger

Every screen maps to the existing Parchment Ledger system (`design-system/DESIGN.md` and
`design-system/tokens.css`). No new tokens are introduced; new components are built from
existing tokens only.

## Tokens used

| Role | Token | Value |
|---|---|---|
| App canvas | `--bg` | `#F5F4ED` parchment |
| Cards, tables, modals | `--surface` | `#FAF9F5` ivory |
| Selected rows, callouts, insets | `--surface-warm` | `#E8E6DC` warm sand |
| Headings and primary copy | `--fg` | `#141413` |
| Secondary copy, links | `--fg-2` | `#3D3D3A` |
| Support text, labels | `--muted` | `#5E5D59` |
| Timestamps, table headers | `--meta` | `#6F6E67` |
| Card and input borders | `--border` | `#F0EEE6` |
| Row separators | `--border-soft` | `#E8E6DC` |
| Primary action, active tab | `--accent` | `#C96442` terracotta |
| Label on terracotta | `--accent-on` | `#FAF9F5` |
| Success | `--success` | `#15803D` |
| Warning | `--warn` | `#A16207` |
| Danger | `--danger` | `#B53333` |
| Keyboard focus | `--focus-blue` | `#3898EC` (the only cool colour) |

## Typography

| Use | Family | Weight / size |
|---|---|---|
| Product name, page titles, section headings | Fraunces (serif) | 500–600, scale 25/32/52 |
| Body, labels, navigation, buttons | Instrument Sans | 400–500, 14–16px |
| Prices, balances, quantities, drift | JetBrains Mono | 400–500, tabular numerals |
| Overline labels | Instrument Sans | 10px uppercase, 0.5px tracking |

Rules applied: sentence case everywhere; body 16px minimum, 1.6 line-height; no serif in table
cells or numbers; mono strictly for figures and code.

## Component mapping by screen

| Screen | Parchment Ledger components | Notes |
|---|---|---|
| S1 Onboarding | Header, form inputs, primary button, step progress text | One terracotta moment: the primary button |
| S2 Connect account | Form inputs, checkbox, primary button, inline error | Read-only key note uses muted body text |
| S3 Dashboard | Sidebar shell, KPI tiles, table, badges, callout | Sidebar is the 240px shell; KPI tiles at 25–32px mono |
| S4 Asset detail | KPI tiles, chart (SVG/CSS), badges, tabs | Chart series use the fixed chart palette, never semantic colours |
| S5 Drift and order intent | Drift bars, tabs, dialog/panel, inputs, toast | Drift in percentage points; guardrail state from portfolio data |
| S6 Brief | Cards, KPI tile, badge | Email uses the same card structure |
| S7 Settings | Tabs, switches, table (sessions), destructive buttons | Delete uses a confirmation dialog |
| S8 Support | Search input, cards, status badge, callout | Status card uses warm sand |

## Responsive behaviour

| Breakpoint | Shell | Content |
|---|---|---|
| Desktop ≥1024px | 64px header + 240px sidebar | 12-column grid, 1200px max, 24px gutters |
| Tablet 640–1023px | Header only, top navigation | 2-column grids, 16px gutters |
| Phone <640px | Header with menu button | Single column, 12px gutters, KPI tiles stack, tables scroll horizontally |

Financial tables never wrap a number onto a second line; they scroll inside their container with
an edge fade.

## Depth and motion

- Cards: 1px `--border`, 8px radius, 24px padding, whisper shadow.
- Interactive states: 1px ring shadow in warm greys.
- Modals and popovers: whisper elevation only; no glass, gradient, or glow.
- Motion: 150ms micro-states, 200ms transitions, `cubic-bezier(0.2, 0, 0, 1)`, reduced-motion
  respected.

## Design-system compliance

| Rule | How the mockups comply |
|---|---|
| Warm neutrals only; no cool blue-greys | All neutrals are warm-toned; Focus Blue appears only as the focus ring |
| One terracotta moment per view | The primary button or active tab, never both prominently |
| Say the number, then the context | Every figure carries an as-of time and a status badge |
| Status never colour alone | Every badge pairs a dot with a word; drift carries sign, arrow, colour, and label |
| Missing value is an em dash | Specified in the KPI tile, table, and drift bar |
| Unpriced assets excluded and disclosed | Dashboard shows the unpriced count and a review action |
| Sample data labelled | The empty brief state labels its sample |
| Charts built in code | Chart series use the fixed chart palette |
| No new tokens | Every component spec names existing tokens |

## Gaps and follow-ups

- The design system does not define a data-table sort indicator; the interaction spec uses
  `aria-sort` plus a caret built from `--meta` and `--fg`.
- The design system does not define a switch; the specification uses a native checkbox styled
  with `--accent` for the on state and `--border` for the off state.
- Both additions are composed from existing tokens and should be added to `DESIGN.md` when the
  SPA is built.
