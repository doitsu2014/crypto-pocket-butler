# Code Summary — u2-web-app

The Web App SPA is built on the existing `web/` shell: React 18 + TypeScript 5 on Vite, Tailwind
CSS v4 on the Parchment Ledger tokens, TanStack Query for server state, React Router for the
route surface, `@supabase/supabase-js` for auth only, and Recharts on the dashboard. Every
browser call to the backend goes through the single C1 API client; Supabase Auth (C13) and
WalletConnect (C14) are the only direct external calls.

## Files created

### Application shell and routing
- `web/src/app/router.tsx` — lazy route chunks for every journey, protected routes, `/support`
  deliberately outside this router.
- `web/src/app/accessibility.test.tsx` — axe suite over every core flow (landmark, heading order,
  labels, table semantics).
- `web/src/main.tsx` (rewritten) — mounts `RouterProvider` with `LocaleProvider`, the 401-aware
  query client and `ErrorBoundary`.
- `web/src/app/AppShell.tsx`, `web/src/app/RequireSession.tsx`, `web/src/app/ErrorBoundary.tsx` —
  shell navigation, session gate and fatal-error recovery (present before this pass; wired now).

### API and data
- `web/src/api/client.ts` — one C1 client: base URL from `VITE_GATEWAY_URL` (same-origin `/v1`
  when unset), fresh `x-correlation-id` per request, RFC 7807 problem handling; the fetch
  implementation is resolved per request.
- `web/src/api/errors.ts`, `web/src/api/types.ts`, `web/src/api/queries.ts`,
  `web/src/api/queryClient.ts` — problem typing, C1 DTOs, TanStack Query hooks with per-key
  `staleTime`, `retry: 1` for 5xx and no retry for 4xx, and the single 401 → sign-out → cache
  clear → `/` rule.

### Auth and security
- `web/src/auth/client.ts`, `web/src/auth/storage.ts` — the one Supabase client and its memory-only
  storage adapter; only the PKCE `${storageKey}-code-verifier` reaches `sessionStorage` and is
  removed after the callback exchange.
- `web/src/auth/session.ts` — the read-only session surface and `signOut`.
- `web/src/auth/mfa.ts`, `web/src/auth/limited.tsx` — MFA requirement routing and the limited
  session banner that disables connect/trade actions.
- `web/src/support/redact.ts` — local secret detection for the support form (mnemonics, `0x`+64
  hex, `xprv`/`xpub`, `sk-`, `pk_live_`, `AKIA`).
- `web/src/telemetry.ts` — one PII-free client-error event (route, error code, correlation id)
  posted with `sendBeacon` and a fetch fallback.
- `web/src/pwa/shouldBypass.ts`, `web/src/pwa/sw.ts` — root-scoped service worker; `/support/*`,
  `/v1/*` and every cross-origin Supabase/WalletConnect request bypass it.

### Journeys
- Auth and onboarding: `web/src/features/auth/SignInPage.tsx`, `VerifyEmailPage.tsx`,
  `web/src/features/onboarding/OnboardingPage.tsx`.
- Connect: `ConnectPage.tsx`, `ExchangeCredentialForm.tsx`, `WalletAddressForm.tsx`,
  `WalletConnectButton.tsx`, `walletconnect.ts`, `CsvImport.tsx`, `AccountStatusRow.tsx`.
- Portfolio: `DashboardPage.tsx`, `PortfoliosPage.tsx`, `PortfolioDetailPage.tsx`,
  `TargetsPage.tsx`, `IntentsPage.tsx`, `TradesPage.tsx`, `HistoryPage.tsx`.
- Activity and P&L: `web/src/features/activity/ActivityPage.tsx`.
- Briefs: `BriefsPage.tsx`, `BriefDetailPage.tsx`.
- Notifications: `web/src/features/notifications/NotificationsPage.tsx` (feed plus per-type
  in-app/email preference matrix, mandatory types locked).
- Settings and security: `web/src/features/settings/SettingsPage.tsx` (profile, sessions,
  revocation warning) and `MfaPage.tsx` (enrolment, recovery codes shown once).
- Privacy: `web/src/features/privacy/PrivacyPage.tsx` (consent history, export with progress and
  download, deletion with confirmation).
- Help: `web/src/features/help/HelpPage.tsx` (articles, support request with local secret
  blocking, active incidents).

### Shared rendering and i18n
- `web/src/components/index.tsx` — `Money`/`Percent`/`Points` render decimal strings exactly as
  supplied (em dash for absent), and the status badges, coverage note, price provenance, error
  panel, empty/loading/blocking states and tables pair every status with a label and a shape.
- `web/src/i18n/index.ts`, `LocaleProvider.tsx`, `react.tsx`, `en.json`, `vi.json` — one resource
  file per locale with an English fallback; problem titles/details map to localised messages.
- `web/src/test/axe.ts`, `gateway.ts`, `render.tsx` — axe helper, gateway mock handlers returning
  RFC 7807 problems, and the render helper.

### Build and deployment
- `web/nginx.conf` — U2 CSP (WalletConnect relay/verify, fonts, `report-to csp`),
  `Reporting-Endpoints`, HSTS/nosniff/Referrer-Policy, immutable `/assets/`, `no-store` for
  `index.html`/`sw.js`/`manifest.webmanifest`, gzip, internal `stub_status` on 8081.
- `web/Dockerfile` — `VITE_GATEWAY_URL`, `VITE_SUPABASE_URL`, `VITE_SUPABASE_ANON_KEY` build args.
- `web/vite.config.ts` — PWA `injectManifest` at root scope with `route-*` lazy chunk naming.
- `web/package.json`, `web/scripts/check-size.mjs` — `yarn size` entry and per-route chunk budget
  check.
- `infra/swarm-stack.yml` — web service `read_only: true` with tmpfs for `/tmp`,
  `/var/cache/nginx` and `/var/run`.

## Files modified

- `web/src/api/client.ts` — fetch resolved per request so a test-provided `fetchImpl` is honoured.
- `web/src/api/queries.ts` — `enabled` flags on consent, sessions, notifications, preferences,
  articles and incidents hooks.
- `web/src/components/index.tsx` — `Money` accepts `testId`; `EmptyState` heading level fixed for
  axe heading order.
- `web/src/features/portfolio/DashboardPage.tsx`, `portfolio.test.tsx` — per-cell money test ids;
  test cleanup.
- `web/src/features/connect/CsvImport.tsx` — the component's own non-CSV refusal path is the
  enforced check.
- `web/src/features/activity/ActivityPage.tsx`, `web/src/features/auth/SignInPage.tsx`,
  `web/src/app/AppShell.tsx`, `web/src/app/ErrorBoundary.tsx` — import/cleanup fixes and
  `?returnTo=` handling.
- `web/src/i18n/en.json`, `vi.json`, `index.ts` — WalletConnect read-only copy, the new
  settings/privacy/help/notifications keys, and an optional `correlationId` on `ProblemLike`.
- `web/src/auth/session.ts`, `web/src/api/client.test.ts` — carried from the interrupted build.
- `web/vite.config.ts`, `web/eslint.config.js`, `web/nginx.conf`, `web/Dockerfile`,
  `web/package.json`, `web/yarn.lock`, `infra/swarm-stack.yml` — as described above.

No application-source file was deleted in this unit.

## Key implementation decisions

- **One 401 rule.** The query client owns it: sign out, clear caches, redirect to `/` with a
  return path; a 403 renders the refusal; a 5xx retries once and then renders the retry panel.
- **No business math in the SPA.** Money, percent, points, allocation, drift and P&L are rendered
  as the gateway supplies them; an absent figure is an em dash.
- **Session in memory only.** The custom adapter keeps every key in memory except the PKCE
  verifier, which is the one documented exception.
- **Degradation per panel.** Each route scopes its queries so one source outage renders its own
  panel's blocking/error state and leaves the rest of the page usable.
- **PWA isolation.** The worker ships at root scope but bypasses `/support/*` and `/v1/*`, so the
  separately deployed support console and every API call stay outside the cache.

## Test coverage summary

- 21 test files, 142 tests, all passing under `yarn --cwd web test`.
- Per-feature suites cover the happy path plus error and edge cases: sign-in and verification,
  onboarding, connect (exchange key rejection, wallet network validation, WalletConnect
  read-only and expiry, CSV partial import and per-line errors), portfolios and dashboard
  (unpriced lines, stale prices, partial valuation, single-panel degradation), targets, intents,
  trades (duplicate idempotency key), activity and P&L coverage, snapshots, briefs (failure
  reason), notifications, settings/MFA/sessions, privacy export and deletion, and help with
  secret blocking.
- `web/src/app/accessibility.test.tsx` runs axe over every core flow.
- `web/src/api/client.test.ts` covers correlation ids, RFC 7807 mapping, 401/403/409/422/500 and
  the gateway-only base URL.
- `web/src/auth/storage.test.ts` proves no token key reaches `localStorage` and that the verifier
  is cleared after the callback.
- The TypeScript coverage threshold remains unresolved in team memory, so coverage is recorded
  but not gated.

## Tests written but not executed in this stage

None. Every test written was executed.

Not written (recorded as gaps rather than run): the Playwright browser-matrix and Lighthouse
checks. The repository has no CI platform configured and Playwright/Lighthouse are not installed;
installing them is not named by the approved plan. The browser-matrix and Lighthouse stages are
recorded in `infrastructure-design/cicd-pipeline.md`, and axe runs in the unit suite.

## Deviations from the plan

1. **No Playwright/Lighthouse CI job files** — see above; no CI platform exists yet in the
   repository, so the runs are recorded in the infrastructure design instead.
2. **nginx Prometheus exporter sidecar not added** — plan Step 18 names the read-only rootfs and
   tmpfs only; the internal `stub_status` endpoint is in place on 8081 and the exporter sidecar
   remains an infrastructure-design gap.
3. **`nginx -t` not run** — no container runtime is available in this environment.
4. **`setAccessToken` retained** in `web/src/auth/session.ts` because the pre-existing
   `web/src/App.tsx` asset-resolver demo and its test still use it; the routed SPA never calls it.
   Removing it would require reworking that legacy surface, which the plan does not name.
5. **i18n timing fixture absent** — the performance design's fixed 30-label/10-figure fixture has
   no local consumer; locale switching is statically imported and covered by the i18n test.
6. **PWA config shape** — `srcDir: "src"` + `filename: "pwa/sw.ts"` fails in vite-plugin-pwa 2.0
   (`ENOENT dist/pwa/sw.mjs`), so `srcDir: "src/pwa"` + `filename: "sw.ts"` is used; the worker
   still ships at root `dist/sw.js` with scope `/`.
7. **Keyboard walkthrough** remains a manual per-flow check as the design states; automation
   covers structure through axe.
8. **Pre-existing formatting debt** — 82 `web/` files already failed `prettier --check`; this
   unit's files were formatted, and the repository-wide check remains pre-existing debt.
