# Security Design — u8-signup-gates-ui

**Upstream:** `nfr-requirements/security-requirements.md` (NFR9.1, NFR10.1–10.2,
NFR11.1–11.2, NFR22.1–22.3, NFR24.1–24.2, NFR25.1–25.4, NFR26.1–26.3),
`functional-spec.md` (revision 2 — §5 security posture, §6 open items), `contract-summary.md`
(C1/C2, C3, C6, C8), codekb `technology-stack.md`.
**Unit kind:** ui — the design covers the SPA feature only. Enforcement of every gate decision
lives server-side (U4 consent gate, U6 jurisdiction gate, gateway route table); this document's
job is to make the client **unable to undermine** those controls and **unable to leak** what
they protect.

## 1. Trust model

```
Browser (untrusted)                Edge / backend (trusted)
┌────────────────────────┐   TLS   ┌─────────┐        ┌──────────────────────┐
│ signup-gates feature   │ ──────▶ │ Gateway │ ─────▶ │ Identity (U1/U4/U6)  │
│  - presentation only   │  JWT in │ JWT val.,│  C8 →  │ jurisdiction decide, │
│  - no decision, no     │  memory │ deny-by- │  C3 →  │ provision, record    │
│    stored state        │  only   │ default, │  C6    │ consent              │
└────────────────────────┘         │ IP deriv.│        └──────────────────────┘
      Supabase Auth (sign-in only) └─────────┘
```

- The SPA is a **presentation surface**: every security-relevant answer (role, status, consent
  sufficiency, jurisdiction) is read from G1/G2 responses; nothing is inferred from the JWT
  (NFR10.2 — no decode/validate code exists in `web/`; the feature imports no JWT library).
- Client-side validation (§6) is UX only; a visitor who bypasses the route guard is still
  refused by U4's server-side consent gate and the gateway table (NFR24.2, FR15.9).

## 2. Session and token handling (NFR10.1)

- The feature reuses the shell's existing in-memory session holder (`web/src/auth/`); it adds
  **no** token handling of its own and writes nothing to localStorage, sessionStorage,
  IndexedDB, or cookies. The TanStack Query cache is memory-only and never persisted (§8 of
  `performance-design.md`); the idempotency-key holder (§4) is likewise memory-only.
- **Everything the feature holds is scoped to the signed-in session, by construction — not by
  a clearing hook.** The repository's manual sign-out path (`AppShell` → `session.signOut()`)
  clears only the token; `queryClient.clear()` runs only on the 401 path in `main.tsx`. The
  design therefore relies on neither: gate query keys are namespaced with the session's user id
  (`["signup-gates", <sessionUserId>, …]`, memory-only, never a URL) and the key holder (§4)
  stamps the session id it minted for. A user switch in the same tab — whatever cleared or
  didn't — finds a different namespace and an unbound holder, so the previous user's gate state
  can never render and the previous key can never be reused (NFR24.1, NFR11.1's "cleared on
  sign-out" is implemented as "nonexistent for the next session"). No change to `session.ts`
  or `main.tsx` is needed, and none is made.

## 3. Transport and origin discipline (NFR9.1)

- Every runtime call from the feature goes through `web/src/api/client.ts` to the gateway over
  TLS; the only other browser-external origin remains Supabase Auth (existing shell, unchanged).
- **No third-party runtime origin by construction:** the country dataset is a bundled generated
  module (T2), locale catalogues are bundled (`web/src/i18n`), terms/risk documents are links
  the visitor follows (new tab, `rel="noopener noreferrer"`) — never fetches. MSW is a
  devDependency imported only from test setup, so it cannot reach the production bundle; the
  existing CSP in `web/nginx.conf` is unchanged and remains the mechanical backstop against a
  new origin sneaking in.

## 4. Idempotency key: holder design and lifetime (NFR11.1, NFR11.2)

- **Holder.** A module-scope feature component (a "submission ticket"), stamped with the
  session id it was minted for (§2 — a different session never sees it): created on the first
  submit intent of a screen visit, held **above** the screen component so in-app remounts and
  route bounces during a pending submission reuse the same key, and discarded on submit
  success. `crypto.randomUUID()` generation (T4); never persisted (NFR10.1).
- **Key ↔ payload invariant.** The key is minted for one exact submit payload: an identical
  resubmission (manual retry after 503/5xx/deadline, or the post-422 completion of the same
  values) reuses it, and that is the replay the server's idempotency handling deduplicates. If
  the visitor **edits any field** between attempts — a different country after a failure, for
  example — the next submit mints a **new key**, because a materially different submission must
  be re-evaluated by C8 and must not replay under the old key's outcome. Duplicate-avoidance
  for consent kinds an earlier partial attempt already recorded never rests on key replay: it
  is the server-side skip-already-recorded backstop (NFR11.2, dependency 2 below), with the G2
  re-fetch rendering only what is still missing.
- **Reload boundary.** A full page reload loses the key by design. The end-to-end guarantee
  therefore rests on the **server-side backstop**: G3/C6 treat already-recorded consent kinds as
  a no-op (NFR11.2, contract dependency to be confirmed before Code Generation). The client
  never needs to know whether a lost-key resubmit is a duplicate: after any failure the screen
  re-fetches G2 and renders only what is still missing (spec §2.6), so a post-reload
  resubmission completes the remainder rather than repeating it.
- **Abuse posture.** No auto-retry of G3 (mutation `retry: 0`), Continue disabled during
  `submitting`, and the gateway's stricter auth-trigger rate limits bound the rest (NFR11.1);
  a client-side loop cannot amplify abuse because the client issues no loop.

## 5. Sensitive-data hygiene (NFR22.1–22.3)

- **URL discipline (NFR22.3).** The feature's client methods are designed against
  **self-scoped** routes only (`/v1/me/...` shape — caller identified from the validated token).
  No path or query string the feature sends contains a provider subject, subject hash, internal
  user id, or email — which matters because the edge logs URLs by default (`web/nginx.conf`).
  If the required contract change (dependency 1 below) does not land before Code Generation,
  the exposure must be accepted as an explicit recorded deviation from NFR22; the feature code
  itself never falls back to a subject-in-path call.
- **Read-only discipline on entry (dependency 1).** Both entry reads — the principal lookup
  *and* the gate-state read — must be **read-only, non-provisioning, and self-scoped**, with a
  defined "no record" answer. The existing `GET /v1/me/consent/state` route is **not** usable
  for entry as it stands: its handler path (`caller_of` → `PrincipalService::load_caller` →
  `resolve`) provisions a user record when none exists — contradicting spec §2.1/FR19.4
  ("never by a read") — and otherwise answers 422 for a first-time visitor because nothing in
  production sets the `x-country-code` header its gate-clearance check needs. The entry
  resolution must therefore target a confirmed read-only surface: either that route gains a
  read-only, non-provisioning mode, or the gate state folds into the read-only principal answer
  (spec §6 open item 2). **No-records vs failure:** the client method maps the defined
  "no record" answer to a typed sentinel (`gateState: null`) inside `client.ts`, so the query
  *resolves* with Gate-Screen semantics (spec §2.1) and is never confused with a rejected
  query, which is the only path to `GateStatus.error` (§2 of `performance-design.md`).
- **Render discipline (NFR22.1).** ProblemDetails handling is an **allowlist mapping**: the
  response `status`/`type` selects a pre-written, localised message variant; `detail` is
  discarded unrendered; the `correlationId` is the only server string ever displayed. No screen
  state, log, or telemetry event contains a subject, hash, user id, or email.
- **No raw problems in shared surfaces.** The repository's shared `ErrorPanel`
  (`web/src/components/index.tsx`) renders `resolved.detail`, and `RequireSession` already uses
  it for its retry surface — so the gate's error surfaces, **the full-page entry retry
  included**, are rendered by dedicated feature components (`GateErrorNotice` and its
  full-page variant) that take the already-mapped localised variant + correlation id and never
  receive the raw problem object. `ErrorPanel` itself is left unchanged; passing a gate problem
  into it is a review-time defect, not a configuration.
- **Minimisation (NFR22.2).** The submit body is exactly `{ countryCode, terms, risk, age }`
  plus the `Idempotency-Key` header; the IP is added by the gateway (never a client field or
  header). The screen collects nothing else. **Telemetry decision:** the feature calls no
  telemetry API at all — `web/src/telemetry.ts` has no property-less counted-event surface
  (its `reportClientError` beacon sends route, errorCode, correlationId, sessionId), so the
  design does not pretend to one. The only browser-emitted report that can mention the gate
  route is the shell's existing render-crash beacon, fired by the root ErrorBoundary for any
  route: its fixed shape carries no country, no acceptance, no user identifier, and that is
  this unit's NFR22.2 answer — zero feature-emitted events, one pre-existing shell beacon
  whose payload is already within the rule.

## 6. Input validation design (presentation layer)

- Four presence checks on submit (country selected; three checkboxes checked) — naming, not
  deciding: the validation summary lists missing items by name (spec §2.3).
- `countryCode` is additionally checked for **membership in the bundled ISO module** before
  submit; this is a data-integrity check (the value must be one of the codes the module
  shipped), not a policy check — the restricted list never exists client-side, and a restricted
  choice is refused by the server (spec §1).
- No business or compliance computation in the browser (project mandate): jurisdiction
  decisions, consent sufficiency, and provisioning are all server answers the feature renders.
- React's default escaping plus the static generated module mean no user- or server-supplied
  string is ever inserted as markup; free-text server strings (`detail`) are never rendered
  at all (§5).

## 7. Non-disclosure design (existence and denial reasons)

- One `BlockedJurisdictionNotice` for every denial: `restrictedBy` (`self_declared | ip |
  both`) is never surfaced, and the 403 mapping is indistinguishable across denial reasons
  (spec §2.5); the continued-use variant differs only by the account-acknowledgement line the
  server state (`jurisdictionBlocked`) already implies to the signed-in visitor.
- The no-record entry answer routes to the Gate Screen exactly like a pending record with
  missing acceptances — the feature reveals nothing about prior existence beyond what the
  visitor's own session already knows.

## 8. Freshness as a security control (NFR24.1)

- Gate state is re-resolved on every authenticated entry and after every failure/422/success
  (`staleTime: 0`, invalidation on success — `performance-design.md` §2/§8); no persisted or
  cached "passed" state can unlock a route. The route guard renders from the fresh resolution
  only, and remains presentational (NFR24.2): removing it from the DOM changes nothing about
  what the server enforces.

## 9. Compliance controls

### Accessibility (NFR25.1–25.4)

- Focus management is designed in, not bolted on: validation summary appears above the fields
  in DOM order with `role="alert"` and receives focus; the blocked notice is announced as a
  page-level status; the combobox follows WAI-ARIA APG (keyboard: arrows, Home/End, type-ahead
  over the static list); tab order equals DOM order (summary links → country → terms → risk →
  age → continue).
- Every state pairs colour with icon + text (Parchment Ledger tokens only — no raw values);
  no motion-dependent interaction.
- **Mechanical gate:** every rendered state (ready, summary, submitting, blocked, both error
  variants) passes jest-axe in the unit's suite (NFR25.4).

### Localisation (NFR26.1–26.3)

- All strings — labels, summary, notices, error variants, country names — ship in `en.json` /
  `vi.json` under a `gates.*` namespace and the generated country module (both locales);
  missing-key fallback follows the existing i18n module behaviour.
- Locale resolution reuses the shell's `LocaleProvider` unchanged; the gate route adds no
  resolution logic of its own. **Recorded deviation from NFR26.2's wording:** `resolveLocale`
  supports the four-step order (`?lang=` → stored server preference → browser → English), but
  the shell's `LocaleProvider` today supplies only `urlLang` and `browser` — the
  stored-preference leg is dormant shell-wide, so the effective order on this screen is
  `?lang=` → browser → English. That matches functional-spec §4 (a first-time visitor has no
  stored preference anyway), and feeding the preference into `LocaleProvider` is a shell change
  outside this unit's blast radius: it is recorded here rather than made here, and NFR26.2's
  full order activates the day the shell wires it — with no gate-route change.
- No date/time rendering on this screen (NFR26.3 is vacuous here and stays that way).

## 10. Threat → mechanism map

| Threat (security requirements) | Design mechanism |
|---|---|
| IP spoofing to defeat the jurisdiction gate | Feature sends no IP field/header; gateway derives it from the trusted proxy chain (spec §1, open item 4); submit body is the fixed 4-field shape (§6) |
| Country shopping against a restriction | Blocked notice is terminal for the session — Sign out only, no re-selection affordance exists in the component set (spec §2.5) |
| Existence disclosure | Single notice for all denials; allowlist error mapping; correlation id only (§5, §7) |
| Partial-application confusion | Failure never clears form state; retry re-fetches G2 and renders the server's `missing` list — the client never reconstructs what committed (§4, spec §2.6) |
| Pre-gate data collection | No field beyond country + three booleans exists on the screen; no telemetry payload (§5) |
| Token/credential leakage | Memory-only session, no storage APIs in the feature, no JWT decode, nothing persisted (§2) |
| Subject leakage via logs | Self-scoped routes only; subject-in-path is a recorded contract dependency, not a client fallback (§5) |
| Supply-chain / new-origin risk | Zero new runtime dependencies; MSW dev-only; bundled datasets; CSP unchanged (§3) |
| Duplicate consent records after reload | Server-side skip-already-recorded backstop (NFR11.2 contract dependency) + G2-driven re-render (§4) |

## 11. Recorded security dependencies (must be pinned before Code Generation)

1. Read-only, **non-provisioning, self-scoped** entry reads — both the principal lookup and
   the gate state (gateway/U1/U4 — NFR22.3, §5 above): spec G1's subject-in-path shape is
   replaced, and the existing `GET /v1/me/consent/state` provisioning path is not reused for
   entry; each read needs a defined "no record" answer. Or an explicit deviation is recorded.
2. G3/C6 **skip-already-recorded** idempotency semantics (U4/U1 — NFR11.2, §4 above).
3. `jurisdictionBlocked` field and its source (U6 — spec §6 open item 3).
4. Gateway **IP-derivation** policy from the trusted proxy chain (gateway — spec §6 open item 4).
5. G3 is a **new authenticated gateway route** (`POST /v1/signup/gates` per functional-spec §6)
   carrying the four-field body, the C8 → C3 → C6 orchestration, and `Idempotency-Key`
   handling, added to the deny-by-default table (gateway/U1 — spec §6 open item 2). The
   existing unauthenticated pre-signup route `POST /v1/signup-gate` (`{jurisdiction,
   declaredAge, disclosuresAcknowledged}` → `gateToken`) serves a different, pre-auth purpose
   and stays as it is; it cannot carry G3 and is not a candidate for it.
