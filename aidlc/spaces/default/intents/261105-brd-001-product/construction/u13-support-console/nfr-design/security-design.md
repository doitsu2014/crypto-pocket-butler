# Security Design — u13-support-console

Inputs: `nfr-requirements/security-requirements.md`, `functional-design/rules.md`.

- **Transport**: HTTPS only; the console is served under `/support` behind the CDN/WAF; CSP with
  `connect-src` limited to the gateway and Supabase Auth.
- **Session**: Supabase Auth JWT kept in memory; never localStorage or sessionStorage; sign-out
  clears the TanStack Query cache; a tab close ends the session.
- **Authorization**: the console reads `app_metadata.role` to show the shell, but the gateway
  enforces `support`/`admin` on every C2 call; the UI is never the control.
- **Grant binding**: server-side in the support API (active, unexpired, account owner equals grant
  target); the UI mirrors the same check and shows the reason for a refusal.
- **Data minimisation**: DTOs contain only kind, label, venue, status, lastSyncedAt, and ticket
  metadata; no credential, holding, valuation, portfolio content, or ticket conversation content.
- **Audit**: reads and incident actions are audited by the support API with a reason hash and grant
  id; the console holds no audit write path.
- **Grant-scoped search**: account search requires the grant id and returns only accounts whose
  owner is that grant's target user; a support user cannot enumerate other users' accounts.
- **Cache clearing**: the metadata cache is cleared on sign-out, on grant expiry or revocation, and
  on any 403.

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
