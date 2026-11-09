# Security Design — u13-support-console

Inputs: `nfr-requirements/security-requirements.md`, `functional-design/rules.md`.

- **Transport**: HTTPS only; the console is served under `/support` behind the CDN/WAF; CSP with
  `connect-src` limited to the same-origin gateway and Supabase Auth. The gateway must be served
  from the same origin as `/support` (through the CDN host); if deploy configures a different
  gateway origin, that origin is added to `connect-src` or the C2 calls are blocked.
- **Session**: Supabase Auth JWT kept in memory; never localStorage or sessionStorage; sign-out
  clears the TanStack Query cache; a tab close ends the session.
- **Authorization**: the console reads `app_metadata.role` to show the shell, but the gateway
  enforces `support`/`admin` on every C2 call; the UI is never the control.
- **Grant binding**: server-side in the support API (active, unexpired, account owner equals grant
  target); the UI mirrors the same check and shows the reason for a refusal.
- **Data minimisation**: DTOs contain only kind, label, venue, status, lastSyncedAt, and ticket
  metadata (including a pseudonymous `requesterRef`); no credential, holding, valuation, portfolio
  content, requester label, or ticket conversation content.
- **Audit**: reads and incident actions are audited by the support API with a reason hash and grant
  id; the console holds no audit write path. The free-text read reason travels only in the
  `X-Support-Reason` request header, never in a URL, so access logs, browser history, and
  referrers never carry it. The gateway and nginx access logs must not log that header.
- **Grant-scoped search and tickets**: account search, account metadata, and the ticket list all
  require the grant id and return only records owned by that grant's target user; a support user
  cannot enumerate other users' accounts or tickets.
- **Cache clearing**: the metadata cache is cleared on sign-out, on grant expiry or revocation, and
  on any 403.
- **Service worker separation**: a root-scoped customer service worker would control `/support`
  and the same-origin `/v1` calls. U2's worker must bypass `/support` and `/v1`; the console also
  unregisters any registration whose scope covers `/support` at startup (`src/pwa/isolate.ts`) as
  defence in depth.

## Accessibility

- **Semantics (WCAG 2.2 AA)**: landmark regions, labelled controls, ARIA live regions for grant and
  incident status, and every screen reachable by route without a mouse.
- **Keyboard and focus**: logical tab order, a visible focus ring on every control, focus trapped in
  modals and returned to the trigger on close.
- **Contrast**: text and control contrast verified against the Parchment Ledger tokens at AA.
- **Status cues**: status is never conveyed by colour alone (label plus shape), so a colour-blind
  operator cannot misread a grant or incident state.
- **Verification**: an automated axe check in the test suite plus a manual keyboard-and-screen-reader
  pass before release.

NFR references: NFR9.1–NFR9.7, NFR12.1, NFR15.1–15.4.
