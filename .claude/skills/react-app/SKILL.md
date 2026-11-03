---
name: react-app
description: >
  Implement the Crypto Pocket Butler web app with React 18 + TypeScript 5
  (Vite), Tailwind CSS v4, TanStack Query, Recharts, and @supabase/supabase-js
  for auth. Use when writing or reviewing SPA code: screens, components, data
  fetching, routing, styling with Tailwind against the Gilded Ledger tokens.
  Visual design rules live in the frontend-design skill; this skill covers the
  implementation stack.
argument-hint: "[screen, component, or feature to implement]"
---

# React Web App — Implementation Stack

Visual rules: the `frontend-design` skill (Gilded Ledger). This skill is how the SPA is built.

## Stack (fixed)

| Concern | Tool |
| --- | --- |
| Build | Vite, React 18, TypeScript 5 (strict) |
| Styling | Tailwind CSS v4 — token-mapped, see below |
| Data fetching / cache | TanStack Query (`@tanstack/react-query`) |
| Auth | `@supabase/supabase-js` — sign-up/in/MFA flows only; tokens held in memory, never localStorage |
| Charts | Recharts, colors from `--chart-1…5` tokens |
| Backend calls | **One** `src/lib/api.ts` module — the only place that talks to the API Gateway |

## Tailwind + Gilded Ledger tokens

Tailwind utilities must resolve to design tokens — never raw hex, px steps, or default-gray
classes (`bg-gray-800` etc. are banned). Set up once in `src/main.css`:

```css
@import "tailwindcss";
@import "../design-system/tokens.css";

@theme inline {
  --color-bg: var(--bg);
  --color-surface: var(--surface);
  --color-surface-warm: var(--surface-warm);
  --color-fg: var(--fg);
  --color-fg-2: var(--fg-2);
  --color-muted: var(--muted);
  --color-meta: var(--meta);
  --color-border: var(--border);
  --color-border-soft: var(--border-soft);
  --color-accent: var(--accent);
  --color-accent-on: var(--accent-on);
  --color-success: var(--success);
  --color-warn: var(--warn);
  --color-danger: var(--danger);
  --color-chart-1: var(--chart-1);
  --color-chart-2: var(--chart-2);
  --color-chart-3: var(--chart-3);
  --color-chart-4: var(--chart-4);
  --color-chart-5: var(--chart-5);
}

@layer utilities {
  .font-display { font-family: var(--font-display); }
  .font-body { font-family: var(--font-body); }
  .tabular { font-family: var(--font-mono); font-variant-numeric: tabular-nums; }
}
```

- Colors: token-named utilities — `bg-surface`, `text-fg-2`, `border-border`, `text-accent`.
- Fonts: `.font-display`, `.font-body`, `.tabular` (every figure is `.tabular`, right-aligned).
- Everything else (radius, spacing, motion, elevation) via `var()` arbitrary values:
  `rounded-[var(--radius-md)]`, `px-[var(--space-6)]`, `duration-[var(--motion-base)]`,
  `shadow-[var(--elev-raised)]`. Only token steps — no invented spacing.
- Focus: `focus-visible:shadow-[var(--focus-ring)]` on every focusable element.

## Code rules

1. **Function components + hooks only.** No classes, no HOCs.
2. **No business math in components** — valuation, drift, and allocation numbers come from
   the backend; the SPA formats and renders them (and labels stale/unpriced data).
3. **All gateway calls through `src/lib/api.ts`.** Components never `fetch` directly; they
   call TanStack Query hooks wrapping that module. One hook per endpoint.
4. **Auth exceptions**: the Supabase client is used directly only for sign-up, sign-in, MFA,
   and session refresh — the single browser-to-Supabase exception. Every other call goes to
   the gateway with the session JWT attached in `api.ts`.
5. Keep files feature-first: `src/features/<screen>/{components,hooks}.tsx(x)`; shared pieces
   in `src/components/`.
6. TypeScript strict; types for API responses live in `src/lib/api.ts` next to the calls.
