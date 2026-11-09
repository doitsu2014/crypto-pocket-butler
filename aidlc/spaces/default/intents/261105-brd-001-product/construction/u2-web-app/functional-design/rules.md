# Business Rules — u2-web-app

Sources: `requirements.md` (NFR15–NFR16, NFR10), `unit-of-work.md` (U2), `design-system/DESIGN.md`.

## BR1 — Render, never compute

- BR1.1: the SPA renders figures exactly as the gateway supplied them; it never computes
  valuation, allocation, drift, P&L or FX.
- BR1.2: every gateway call goes through the one API-client module; the only direct calls are
  Supabase Auth and WalletConnect.

## BR2 — Missing and degraded values

- BR2.1: a missing financial value renders as an em dash, never `0` or `$0.00`.
- BR2.2: a `partial` or `unavailable` data status is shown with a label and its as-of instant.
- BR2.3: a stale price keeps its value and shows its timestamp and freshness label.

## BR3 — Session and privacy

- BR3.1: the Supabase session is held in memory only; nothing is written to `localStorage` or
  `sessionStorage`, with one stated exception: the PKCE `${storageKey}-code-verifier` may live in
  `sessionStorage` until the callback exchange completes (C1 amendment).
- BR3.2: the SPA never displays a credential, token, seed phrase or full address.
- BR3.3: the support console at `/support` is isolated from the PWA service worker.

## BR4 — Status is never colour-only

- BR4.1: every status pairs a label and a shape with its colour.
- BR4.2: every async surface has loading, empty, error and degraded states.

## BR5 — Accessibility

- BR5.1: core flows are keyboard reachable with visible focus, labelled controls and live-region
  announcements, meeting WCAG 2.2 AA.

## BR6 — Localisation

- BR6.1: English and Vietnamese strings come from one resource file per locale; dates and numbers
  use the user's locale and timezone.

## BR7 — Errors

- BR7.1: a failed call renders the RFC 7807 title and detail with the correlation id; a source
  outage degrades one panel rather than the page.
