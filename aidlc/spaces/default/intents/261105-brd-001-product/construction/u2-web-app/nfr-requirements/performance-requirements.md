# Performance Requirements — u2-web-app

Sources: `functional-design/functional-spec.md` (route and data behaviour) and
`functional-design/rules.md` (BR1 render-only, BR2 degraded values).

All targets are **derived for this unit** (the inception NFR2 speaks of one p95 dashboard target);
each names its condition and the tool that measures it.

| ID | Requirement | Condition and measure |
|---|---|---|
| NFR2.1 | The dashboard shows its first figures within 2 s p95 (cold cache: 4 s p95) | Lighthouse CI, mobile preset, 4× CPU throttle, Fast 3G, 10-asset portfolio fixture; the cold path is bounded separately, never hidden inside the 2 s claim |
| NFR2.2 | A route shows a skeleton within 100 ms of navigation and never blocks the shell | Testing Library timing on mocked deferred queries |
| NFR2.3 | Initial JS ≤ 300 kB gzipped; any single route chunk ≤ 150 kB | `vite build` output + a `size-limit` script (`yarn size`) with the entry and per-route chunk set defined in its config |
| NFR2.4 | A locale switch re-renders the dashboard's 30 labels and 10 figures within 200 ms without a reload | React Profiler measurement in a component test over that fixed fixture |
| NFR2.5 | Lists request 50 rows per page and never fetch an unbounded page | API-client test |
| NFR6.1 | Onboarding to the first dashboard figure: ≤ 2 min by address, ≤ 10 min by exchange key | Playwright onboarding flow against a seeded gateway |
| NFR2.6 | Supported browsers: last two versions of Chrome, Edge, Firefox and Safari (desktop and iOS/Android Safari/Chrome), verified by Playwright projects; older engines render a supported-browser notice | CI matrix; axe and keyboard checks run in Chromium, Firefox and WebKit |
| NFR2.7 | Offline: the service worker serves the app shell and shows an offline notice for data routes; it never serves stale financial data | Handler test; Playwright offline check |
