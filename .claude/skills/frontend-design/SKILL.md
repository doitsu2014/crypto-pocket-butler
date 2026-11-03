---
name: frontend-design
description: >
  Build Crypto Pocket Butler UI against the Parchment Ledger design system in
  design-system/ — warm parchment canvas, terracotta accent, warm-toned
  neutrals, serif display / sans UI split, tabular mono figures. Use when
  creating or styling any frontend surface: React SPA screens, HTML artifacts,
  mockups, emails, or dashboards. Covers token usage, component selection,
  data/figure conventions, and the do's and don'ts.
argument-hint: "[screen or component to build]"
---

# Frontend Design — Parchment Ledger

All UI in this repo follows the **Parchment Ledger** design system (themed on the Claude design language). The system's own files are the source of truth — read them before building:

| File | Use it for |
| --- | --- |
| [`design-system/DESIGN.md`](../../design-system/DESIGN.md) | The written system: palette roles, typography, components, layout, data conventions, motion, do's and don'ts |
| [`design-system/tokens.css`](../../design-system/tokens.css) | The `:root` token block — paste verbatim, never edit values ad hoc |
| [`design-system/index.html`](../../design-system/index.html) | Visual style guide — open in a browser |
| [`design-system/app.html`](../../design-system/app.html) | Working app prototype — consolidated portfolio view, drift, accounts, briefs |
| `design-system/logo.svg` / `logo-mark.svg` | Brand lockup / emblem (emblem doubles as favicon) |

## Non-negotiables

1. **Tokens only.** Every color, size, radius, and duration is `var(--name)` from `tokens.css`. Never write a raw hex value, font family, or made-up spacing step. In the React SPA, import `tokens.css` once at the app root; in standalone HTML artifacts, paste the `:root` block into the first `<style>`.
2. **One terracotta accent.** `--accent` marks the primary action and at most one detail per page — like a bookmark, only where importance lives. Never a wash.
3. **Warm neutrals only.** Every gray carries a yellow-brown undertone; no cool blue-grays, no pure white page backgrounds — the warm cream is the personality.
4. **Mono figures, right-aligned.** Every number (balances, prices, drift, percentages) is `JetBrains Mono` with tabular numerals, right-aligned in its column. Pair every value with its data status and as-of time — stale or unpriced assets are labelled, not silently shown.
5. **Semantic color is state only** (`--success`, `--warn`, `--danger`), always paired with a text label. Chart series colors (`--chart-1…5`) keep their fixed order and a legend — color is never the only signal.
6. **Fonts from Google Fonts only:** Fraunces (display), Instrument Sans (UI/body), JetBrains Mono (figures).

## Workflow

1. Skim `DESIGN.md` sections for the surface type you're building (components, layout, data conventions, responsive, accessibility); open `app.html` to see the app shell and portfolio screens rendered.
2. Build from the documented components. If none fits, compose the new one from tokens and propose adding it to the guide rather than freelancing outside the system.
3. Verify contrast holds: the foreground ramp (`--fg`, `--fg-2`, `--muted`, `--meta`) is pre-cleared at 4.5:1 on the parchment surfaces — don't place token colors on untested backgrounds.
4. When in doubt, add space and remove chrome.

## React SPA specifics

- React 18 + TypeScript 5, Vite, TanStack Query for data, Recharts for charts.
- Charts read the `--chart-*` tokens; the SPA displays backend-provided numbers only — no business math in components (see the development rules).
- Sample figures in the style guide and prototype are illustrative; never ship them as real data.
