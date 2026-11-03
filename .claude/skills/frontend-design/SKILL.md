---
name: frontend-design
description: >
  Build Crypto Pocket Butler UI against the Gilded Ledger design system in
  design-system/ — black-lacquer surfaces, ivory Didone display type, one
  champagne-gold accent, tabular mono figures. Use when creating or styling
  any frontend surface: React SPA screens, HTML artifacts, mockups, emails, or
  dashboards. Covers token usage, component selection, data/figure
  conventions, and the do's and don'ts.
argument-hint: "[screen or component to build]"
---

# Frontend Design — Gilded Ledger

All UI in this repo follows the **Gilded Ledger** design system. The system's
own files are the source of truth — read them before building:

| File | Use it for |
| --- | --- |
| [`design-system/DESIGN.md`](../../design-system/DESIGN.md) | The written system: palette roles, typography, components, layout, data conventions, motion, do's and don'ts |
| [`design-system/tokens.css`](../../design-system/tokens.css) | The `:root` token block — paste verbatim, never edit values ad hoc |
| [`design-system/index.html`](../../design-system/index.html) | Visual style guide — open in a browser to see the components rendered |
| `design-system/logo.svg` / `logo-mark.svg` | Brand lockup / emblem (emblem doubles as favicon) |

## Non-negotiables

1. **Tokens only.** Every color, size, radius, and duration is `var(--name)` from
   `tokens.css`. Never write a raw hex value, font family, or made-up spacing step. In the
   React SPA, import `tokens.css` once at the app root; in standalone HTML artifacts, paste
   the `:root` block into the first `<style>`.
2. **One gold accent.** Gold (`--accent`) marks the primary action and at most one detail per
   screen. It is a signal of importance, never a wash. When in doubt about gold, use less.
3. **Mono figures, right-aligned.** Every number (balances, prices, drift, percentages) is
   `IBM Plex Mono`, tabular, right-aligned in its column. Pair every value with its data
   status and as-of time — stale or unpriced assets are labelled, not silently shown.
4. **Semantic color is state only** (`--success`, `--warn`, `--danger`), always paired with a
   text label, under 5% of visible surface. Chart series colors (`--chart-1…5`) keep their
   fixed order and a legend — color is never the only signal.
5. **Fonts from Google Fonts only:** Playfair Display (display), Jost (interface),
   IBM Plex Mono (figures).

## Workflow

1. Skim `DESIGN.md` sections for the surface type you're building (components, layout, data
   conventions, responsive, accessibility).
2. Build from the documented components (cards, tables, tabs, inputs, buttons, badges,
   charts). If none fits, compose the new one from tokens and propose adding it to the guide
   rather than freelancing outside the system.
3. Verify contrast holds: the ramp (`--fg`, `--fg-2`, `--muted`, `--meta`) is pre-cleared at
   4.5:1 on `--bg`, `--surface`, `--surface-warm` — don't place token colors on untested
   backgrounds.
4. When in doubt, remove chrome and add space.

## React SPA specifics

- React 18 + TypeScript 5, Vite, TanStack Query for data, Recharts for charts.
- Charts read the `--chart-*` tokens; the SPA displays backend-provided numbers only — no
  business math in components (see the development rules).
- Sample figures in the style guide are illustrative; never ship them as real data.
