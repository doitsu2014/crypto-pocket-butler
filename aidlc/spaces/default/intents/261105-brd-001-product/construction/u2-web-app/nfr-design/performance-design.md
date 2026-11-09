# Performance Design — u2-web-app

Sources: `nfr-requirements/performance-requirements.md`, `functional-design/functional-spec.md`.

- **Code splitting.** React Router lazy routes: the shell plus sign-in in the entry chunk; each
  journey a route chunk. Recharts and WalletConnect lazy-loaded only on the dashboard and connect
  routes. `size-limit` measures the **full initial chunk set** (the entry plus every chunk Vite
  eagerly preloads from `index.html`) at ≤ 300 kB gzip, and an explicit route-chunk list at
  ≤ 150 kB each. The `size-limit` dev dependency and a `yarn size` script are added in code
  generation (NFR2.3), before the first release.
- **Browser and offline.** Playwright projects run Chromium, Firefox and WebKit; browsers outside
  the last two versions render a supported-browser notice. Offline serves the shell and an offline
  notice only; financial data is never served stale.
- **Data fetching.** TanStack Query with `staleTime` per key (valuation 30 s, portfolios 60 s,
  briefs 5 min, notifications 30 s), `retry: 1` for 5xx and no retry for 4xx, and a skeleton after
  100 ms (`placeholderData` for the previous page).
- **Dashboard call shape.** `GET /v1/portfolios` lists the user's portfolios (id, name,
  baseCurrency); each portfolio's figures come from `GET /v1/portfolios/{portfolioId}` (the
  valuation with lines), fetched with concurrency 4 and rendered per portfolio. For an N-portfolio
  dashboard the sequence is 1 list + N valuations; the 2 s p95 budget is measured against N = 1
  with 10 assets, and N > 1 renders panels progressively. Per-portfolio aggregation is a U6/U1
  dependency recorded in the traceability.
- **401 rule.** A 401 signs out and redirects to `/`; a 403 renders the refusal; a 5xx retries
  once. This is the one rule all views implement.
- **Localisation.** Both locale JSON files are imported statically (two small files), so a switch
  re-renders without a network call; the timing fixture is fixed (30 labels, 10 figures).
- **Auth.** The supabase client initialises once; `getSession()` is cached; the API client awaits
  it per request without blocking the shell.
- **Onboarding.** Sign-in → verification → consent → preferences → first account → dashboard; the
  NFR6.1 timing runs this flow in Playwright against a seeded gateway (≤ 2 min by address,
  ≤ 10 min by exchange key).
- **Accessibility.** axe checks run in Chromium, Firefox and WebKit on every core flow; a keyboard
  walkthrough verifies focus order, 2.5.8 target size, 2.4.11 focus not obscured and 3.3.8
  accessible authentication.
- **Dependency pinning.** `yarn.lock` is committed; CI runs `yarn audit` and the image scan
  before release (NFR12.1).
