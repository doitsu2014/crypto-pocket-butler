# Security Design — u2-web-app

Sources: `nfr-requirements/security-requirements.md`, `functional-design/rules.md` BR3/BR7.

## Supabase client and session

One module (`src/auth/client.ts`) creates the client with `persistSession: true`,
`autoRefreshToken: true`, `detectSessionInUrl: true`, `flowType: 'pkce'` and a custom storage
adapter. The adapter is the rule: it keeps every key in memory **except**
`${storageKey}-code-verifier`, which alone is written to `sessionStorage` and removed after the
callback exchange. Access and refresh tokens therefore never touch browser storage, and the PKCE
callback survives the full-page navigation. auth-js's `persistSession: false` is *not* used,
because it replaces a custom adapter with its in-memory one and would lose the verifier.

`src/auth/session.ts` exposes the supabase session (read-only) and `signOut`; the API client reads
the access token from it per request. The standalone token setter in `web/src/auth/session.ts` is
removed.

**401 rule (one rule everywhere).** A 401 on any gateway call signs the user out
(`supabase.auth.signOut()`), clears local query caches, and redirects to `/` with a return path.
A 403 renders the refusal panel with the correlation id. A 5xx retries once, then renders the
retry panel. The functional spec, frontend-components and performance design all state this rule.

## API client

One module; base URL from `VITE_GATEWAY_URL` (same-origin `/v1` when unset); a fresh
`x-correlation-id` per request; the problem body's `correlationId` shown on error. It refuses any
origin other than the gateway, Supabase Auth and WalletConnect.

## Secret detection

`src/support/redact.ts` matches 12/24-word mnemonics, `0x`+64 hex, `xprv`/`xpub`, and the key
prefixes `sk-`, `pk_live_`, `AKIA`; the support form blocks submit and names the field. A normal
sentence containing "key" passes.

## Service worker

`vite-plugin-pwa` with `strategies: 'injectManifest'`, `srcDir: 'src'`,
`filename: 'pwa/sw.ts'`, registered at **root scope** (`/`), because the SPA is served at the site
root. The fetch handler delegates to a pure predicate `shouldBypass(url)` that returns true for
`/support/*`, `/v1/*` and every cross-origin Supabase/WalletConnect request; only same-origin
static assets are cached. Tests import `shouldBypass` directly and assert the registered scope.

## CSP

`web/nginx.conf` ships
`default-src 'self'; connect-src 'self' https://*.supabase.co wss://relay.walletconnect.com wss://relay.walletconnect.org https://verify.walletconnect.com https://verify.walletconnect.org; frame-src https://verify.walletconnect.com https://verify.walletconnect.org; script-src 'self'; style-src 'self' 'unsafe-inline'`
— the WalletConnect relay and verify origins are the only external additions, because C14 is a
contracted direct browser call. A test parses the header and asserts those origins plus nothing
else.

## MFA

C1 must expose the MFA state the SPA can read: the amendment below adds `mfaRequired` and
`mfaEnrolled` to the signed-in profile and the `POST /v1/mfa/enrol` and `POST /v1/mfa/recovery`
routes. After sign-in and on each protected navigation, a caller with `mfaRequired && !mfaEnrolled`
(a user with exchange credentials at `aal1`) is routed to `/settings/mfa` before any other route.
Declining leaves the session in a limited state: a persistent banner and the connect/trade actions
disabled.

## Telemetry

`src/telemetry.ts` posts one JSON event (route, error code, correlation id; no token, payload or
PII) to the public, rate-limited C1 route `POST /v1/telemetry/client-errors` (amendment below),
using `sendBeacon` with a fetch fallback. It never blocks rendering and silently drops when the
beacon fails.

## Rendering and supply chain

React escapes by default; no `dangerouslySetInnerHTML`; problem details render as text.
Dependencies are pinned in `yarn.lock`; CI runs `yarn audit` and the image scan. Accessibility:
axe checks in Chromium, Firefox and WebKit plus a keyboard walkthrough per core flow.
