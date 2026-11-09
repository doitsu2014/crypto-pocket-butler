# Functional Specification — u2-web-app

Inputs: `inception/units-generation/unit-of-work.md` (U2), `inception/units-generation/unit-of-work-story-map.md`
(the U2 rows, 30 stories and 91 acceptance criteria), `inception/requirements-analysis/requirements.md`,
`inception/user-stories/stories.md`, `inception/contract-design/contract-summary.md` (C1 routes),
`design-system/DESIGN.md`, `web/` (existing shell).

The SPA is a React 18 + TypeScript 5 Vite application. It renders every U2 journey from the
gateway's C1 surface, computes no business math, holds the Supabase session in memory only, is
available in English and Vietnamese, and meets WCAG 2.2 AA on its core flows.

## Journeys and routes

Every browser call is a **C1 gateway operation** except Supabase Auth (C13) and WalletConnect (C14).
The "C1 operation" column names the operation, not an internal service.

| Route | Journey | C1 operation (owner) |
|---|---|---|
| `/` | public landing and sign-in gate; a signed-in visit redirects to `/dashboard` | `POST /v1/signup-gate` (U3 via gateway), Supabase Auth sign-in (C13) |
| `/verify-email` | email verification status and resend (US1.1) | `POST /v1/auth/resend-verification` (U3) |
| `/onboarding` | consent, preferences (base currency, locale, timezone), first account | `POST /v1/consent`, `PUT /v1/preferences`, `POST /v1/accounts` (U3/U4) |
| `/connect` | exchange key, wallet address, WalletConnect session, CSV import, verification status | `POST /v1/accounts`, `POST /v1/accounts/{id}/verify`, `POST /v1/imports`, `GET /v1/imports/{id}` (U4), C14 WalletConnect |
| `/dashboard` | total value, allocation, drift, freshness and data status | `GET /v1/portfolios` (U6) |
| `/portfolios` | portfolio list, create and rename (US3.2) | `GET`/`POST /v1/portfolios`, `PATCH /v1/portfolios/{id}` (U6) |
| `/portfolios/{id}` | per-asset breakdown; move an account in or out (US3.2, AC3.1–AC3.4) | `GET /v1/portfolios/{id}`, `PUT /v1/portfolios/{id}/accounts/{accountId}`, `DELETE` the membership (U6) |
| `/portfolios/{id}/targets` | target editor with the sum and difference from 100; no-targets prompt (AC5.1–AC5.2) | `PUT /v1/portfolios/{id}/targets?preview=true` and without preview (U6) |
| `/portfolios/{id}/intents` | intent preview with projected allocation, drift and shortfall; create; cancel (US5.3) | `POST /v1/portfolios/{id}/order-intents?preview=true`, `POST`, `POST .../{intentId}/cancel` (U6) |
| `/portfolios/{id}/trades` | record an executed trade (US5.4) | `POST /v1/portfolios/{id}/trades` (U6) |
| `/portfolios/{id}/history` | immutable snapshot history with as-of and data status (US4.4) | `GET /v1/portfolios/{id}/snapshots` (U6) |
| `/activity` | transactions, corrections, transfers and P&L with coverage labels (US4.1–4.5) | `GET /v1/users/{userId}/transactions`, `GET /v1/users/{userId}/pnl`, `POST /v1/transactions/{id}/corrections` (U7) — C1 additions recorded in the contract; a blocking dependency until the gateway exposes them |
| `/briefs` | brief list (US6.1) | `GET /v1/users/{userId}/briefs?period=&cursor=` (U8) |
| `/briefs/{briefId}` | one brief: pending, generated or failed with `failureReason`, lines, coverage, as-of and data status | `GET /v1/users/{userId}/briefs/{briefId}` (U8) |
| `/notifications` | feed and preference matrix (US6.3) | `GET /v1/users/{userId}/notifications`, `GET`/`PUT .../preferences` (U9) |
| `/settings` | profile, security, sessions, MFA enrolment and recovery codes (US7.1–7.2) | `GET`/`PATCH /v1/users/me`, `GET /v1/sessions`, `DELETE /v1/sessions/{id}`, `POST /v1/mfa/enrol`, `POST /v1/mfa/recovery` (U3) |
| `/privacy` | consent history, export request and download, deletion request (US7.3) | `GET /v1/consent`, `POST /v1/exports`, `GET /v1/exports/{id}`, `POST /v1/privacy/deletions` (U3) |
| `/help` | help centre articles, support request, user status page (US8.1) | `GET /v1/support/articles`, `POST /v1/support/requests`, `GET /v1/support/incidents?active=true` (U3) — not the U13 console grant APIs |
| `/settings/mfa` | MFA enrolment and recovery codes (added by the C1 amendment) | `GET /v1/users/me` (`mfaRequired`, `mfaEnrolled`), `POST /v1/mfa/enrol`, `POST /v1/mfa/recovery` (U3) |
| `/support` | the separately deployed U13 support-console bundle on its own path, outside this SPA's router and service worker | U13 bundle |

Unimplemented C1 operations are recorded as blocking dependencies owned by the gateway (U1) and
the providing unit; a route whose operation is missing renders the blocking state and is listed in
the traceability as GAP rather than silently failing.

## Forms and interaction flows

| Form | Local validation | Gateway refusal handling |
|---|---|---|
| Sign-in / sign-up gate | email shape; password length; gate token required for sign-up | 401 shows the problem detail; 422 maps field errors |
| Resend verification | email shape | 429 shows the retry-after label |
| Consent / preferences | one legal basis per purpose; currency, locale and timezone required | 422 field errors; the step stays open |
| Exchange credential | label and key fields required; passphrase optional; no client storage | 422 names the field; a verification failure shows the reason and keeps the form |
| Wallet address | network from the supported list; address shape per network | 422 lists the supported networks (AC2.1.4) |
| CSV import | file type and size; duplicate and skipped counts shown after upload; per-line errors listed with line numbers | 422 shows the per-line errors; a partial import shows accepted and skipped counts |
| Target editor | each target 0–100; total must equal 100 to save (preview shows the difference) | 422 names the offending asset; save stays disabled until valid |
| Intent form | side, quantity > 0; preview before submit; shortfall shown when the holding is insufficient | 409 when the intent conflicts; 422 field errors |
| Trade form | side, quantity > 0, price ≥ 0, executed time; price source venue or user | 409 on a duplicate idempotency key with the existing trade; 422 field errors |
| Correction form | transaction selected; corrected values validated as the trade form | 409 when a correction already exists; 422 field errors |
| Backfill-free support request | subject and body required; no credential or full address permitted | 422 field errors |

Each flow: loading state on submit, disabled double-submit, error panel with the RFC 7807
`title`/`detail` and the response `correlationId`, and a retry that reuses the same request body.
**A 401 signs out, clears the query cache and redirects to `/` with a return path; a 403 renders
the refusal; a 5xx retries once, then renders the retry panel.**

## Behaviour

- **Session.** The Supabase client uses `persistSession: true` with a custom storage adapter that
  keeps every key in memory **except** the `${storageKey}-code-verifier`, which alone lives in
  `sessionStorage` until the PKCE callback exchange completes (the one stated exception to BR3.1);
  access and refresh tokens never touch browser storage and are lost on reload, so a reload
  re-authenticates. `autoRefreshToken: true`, `detectSessionInUrl: true`, `flowType: 'pkce'`.
- **Data.** One API-client module owns gateway calls. It generates a fresh `x-correlation-id`
  per request and, on an error, shows the `correlationId` from the problem body. TanStack Query
  owns server state; a 401 is retried once after re-authentication, otherwise it surfaces.
- **Figures.** The SPA renders decimal strings as supplied, tabular mono, em dash for absent.
  It never formats `null` as `0`/`$0.00` and never recomputes allocation, drift or P&L.
- **Status model.** Valuation-level `complete|partial|unavailable`, per-line
  `priced|unpriced|holdings_unavailable`, `holdingsUnavailable`, `pricedCoverage`,
  `excludedCount`, freshness `live|delayed|stale|unpriced`, and P&L `marksUnavailable`,
  `holdingsUnavailable`, `fxUnavailable`, `fxEstimated` each render a label, a shape and an as-of;
  a withheld figure is an em dash. Every route declares its panels so one source's outage degrades
  only its panel (AC2.3.3, AC3.4.2, US4.3).
- **Localisation.** One JSON resource per locale (`en`, `vi`) loaded by a small i18n module with
  the `en` fallback; before sign-in the locale comes from `navigator.language` and a URL
  `?lang=`, after sign-in from the server preference. Problem titles/details map to localised
  messages by problem type, with the server detail shown when no mapping exists.
- **Accessibility.** WCAG 2.2 AA on core flows: keyboard order, visible focus, 2.5.8 target size,
  2.4.11 focus not obscured, 3.3.8 accessible authentication (no cognitive test), 3.3.7 redundant
  entry avoided, 3.3.6 consistent help, table `scope`/`caption`, live regions. Verified by
  automated axe checks and a keyboard walkthrough per core flow.
- **PWA.** `vite-plugin-pwa` `injectManifest` with a root-scoped worker (`/`); its
  `shouldBypass` predicate skips `/support/*`, `/v1/*` and cross-origin Supabase/WalletConnect
  requests, and only same-origin static assets are cached. `/support` is served by the U13
  bundle, not this SPA.
- **MFA.** After sign-in and on each protected navigation, a profile with `mfaRequired &&
  !mfaEnrolled` routes to `/settings/mfa`; declining leaves a limited session with a banner and
  the connect/trade actions disabled (C1 amendment).
- **Errors.** A source outage degrades one panel; a fatal error (lost session, failed bootstrap)
  renders a full-page recovery action.

## Non-goals

- No business math, no direct service calls beyond the gateway (Supabase Auth and WalletConnect are
  the only direct calls), no credentials in the browser beyond the in-memory session token.
