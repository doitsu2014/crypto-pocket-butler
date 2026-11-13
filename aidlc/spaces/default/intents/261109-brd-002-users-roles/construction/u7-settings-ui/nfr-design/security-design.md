# Security Design — u7-settings-ui

**Upstream:** `nfr-requirements/security-requirements.md` (NFR9.1, NFR10.1–10.3, NFR11.1–11.2,
NFR21.1, NFR22.1–22.6, NFR24.1–24.7, NFR25.1–25.6, NFR26.1–26.3),
`nfr-requirements/tech-stack-decisions.md` (T1–T6), `functional-spec.md` (§1, §2.1, §3.5–§3.7,
§7, §8, §10), `contract-summary.md` (C1/C2, C3, C6, C7).
**Unit kind:** ui with the six Identity route additions the spec names. The authorisation
mechanisms themselves (JWT validation, `PrincipalService`, the lifecycle state machine, the audit
writer) pre-exist and are not redesigned here; this document designs what the unit adds and the
bounds that keep it from weakening them.

## 1. Trust model

- **The server decides; the browser presents.** Every status, ownership and role decision is
  made by Identity from the validated token and the stored record. The SPA's route guard, the
  `gateStatus` mapping, and every disabled control are presentational (FR15.9, NFR24.3). A
  forged status, a hand-edited cache entry or a direct `curl` reaches the same server refusal.
- **Trusted:** the Supabase-issued JWT after Identity has validated it, the gateway's route
  table, Identity's own database.
- **Untrusted:** everything the browser sends (including its path, query, body and headers other
  than the bearer token), every rendered user value (display names), and every problem payload
  beyond `title`/`status`/`correlationId`.
- **Two trust boundaries this unit crosses:** the browser → gateway edge, where the token is
  presented and forwarded unchanged; and the gateway → Identity hop, where the route table decides
  whether the path is reachable at all (deny-by-default).

## 2. The non-denying reads (NFR24.1, NFR24.2, NFR24.7) — the unit's central security design

A `pending_deletion` or `disabled` account must be able to learn its own state and recover, but
BR1.7's denial must not be weakened for anything else. The design is a narrow, self-scoped
allowlist, not a change to the predicate.

**The five non-denying entry points on Identity, and what bounds each**

| Entry point | Handler | Why it must bypass the denial | Bound |
|---|---|---|---|
| D0 `GET /v1/me/account-status` (new) | `account_status` | Routing must know the status before any screen can be chosen — including the one that says "you are pending" | Resolves by auth subject through the stored-record lookup; takes **no id**; returns only the caller's own status |
| D1 `GET /v1/me/deletion-request` (new) | `get_own_deletion` | The recovery screen must show the recovery date and whether a request exists | Same lookup, no id, returns only the caller's own open request |
| D2 `DELETE /v1/me/deletion-request` (new) | `cancel_own_deletion` | Restoring is the whole point of the recovery window | Same lookup, no id, cancels only the caller's own open request (`410` after the window) |
| D3 `POST /v1/privacy/deletions` (exists) | `request_deletion` | An `Active` person requests deletion | `AccountLifecycle::request_deletion` accepts only the `Active → PendingDeletion` edge (`lifecycle.rs` `EDGES`, BR5.1); every other status is refused with `409` |
| `GET /v1/privacy/deletions/{deletionId}` (exists) | `get_deletion` | **Not used by this unit's screens** — the delete and restore routes read D1 only (spec §3.5/§3.6, and `performance-design.md` §7 has no polling anywhere). It is listed because it already resolves through the stored-record lookup, so it is part of the non-denying surface whose bound must hold for any later consumer | `deletions.status(&caller, deletion_id)` is ownership-checked; another user's id answers `404` without disclosure |

**How the lookup is built.** D0/D1/D2 resolve their caller with the same stored-record lookup the
existing deletion handlers use — `find_by_auth_subject(claims.subject)`, the shape of
`caller_for_deletion` in `services/identity/http/src/handlers.rs` — rather than the denying
`PrincipalService`. That is the *only* difference: the caller is then passed through exactly the
same use case and ownership path as any owner request.

**What is deliberately not done.**

- No route accepts a user id it did not derive from the token. The three new routes have no id
  segment, so they cannot be pointed at another user's row even by a caller who supplies one.
- No widening of `UserStatus::denies_use`. Every other endpoint still answers `403` for
  `disabled`, `pending_deletion` and `deleted`; the predicate and its `PrincipalService`
  enforcement are untouched (NFR24.2).
- No widening of the payload. D0 answers `AccountStatusView` — `{ userId, status }`, exactly the type
  `frontend-components.md` §2 declares, where `userId` is the query's identity value and is never
  rendered (§5); D1/D2 answer the caller's own deletion record. Neither exposes profile, preferences,
  consent, portfolio or account data — those reads keep the denial.
- No new non-denying route without a bound. A sixth path that resolves through the stored-record
  lookup and has none of the three shapes above (no-id self-scope, a state-machine edge guard, or
  an ownership check) is a security regression; NFR24.7 says so explicitly so a later unit cannot
  add one quietly.

**Residual risk and why it is acceptable.** The bypass is readable from the code as one function
with a comment saying exactly why it exists (the existing `caller_for_deletion` doc comment makes
this argument for the two deletion routes). The blast radius of a mistake in D0/D1/D2 is a caller
learning **their own** status or **their own** open request — information the product is required
to give them to make recovery possible.

## 3. Session and token handling (NFR10.1–NFR10.3)

- The token is read from the shell's in-memory session holder and presented as
  `Authorization: Bearer` by the shared client. The unit adds no storage API call: no token, no
  deletion id, no recovery date and no consent state is written to `localStorage`,
  `sessionStorage`, IndexedDB or a cookie.
- Preferences are persisted **server-side** through X2. The browser holds them only as query-cache
  values, which are memory-only (the query client is never paired with a persister).
- The SPA never decodes or validates the JWT and never derives a role or a status from it
  (NFR10.2). The `ProfileView.status` and the role badge (U9/U10) come from server reads.
- Because authorisation travels in a header and never a cookie, this flow needs no CSRF token and
  adds none (NFR10.3). Recorded so the absence is a decision.

## 4. Transport and origin discipline (NFR9.1)

- Every settings call goes over TLS to the API Gateway. The shell's existing external origins —
  webfonts (`fonts.googleapis.com`, `fonts.gstatic.com`), the WalletConnect origins in the CSP,
  and Supabase Auth — are pre-existing; **this unit adds none**.
- The timezone list is built from `Intl.supportedValuesOf("timeZone")`, so there is no dataset
  fetch and no CDN dependency.
- The edge serves the SPA with the existing CSP (`web/nginx.conf`); the unit adds no `connect-src`
  entry, because every new call is same-origin `/v1/...`.

## 5. Sensitive-data hygiene (NFR22.1–NFR22.6)

- **No identifier in a URL.** Every user-scoped settings route is self-scoped (`/v1/me/...`,
  `/v1/privacy/deletions`) — the edge logs URLs by default, so an id in a path is an id in the
  logs. The one exception is the export poll `GET /v1/exports/{exportId}`, which carries an
  opaque job id owned by the caller; the server checks ownership and the id names no person.
- **Profile field discipline.** P1 returns `userId`, `displayName`, `displayNameOverridden`,
  `providerDisplayName`, `email`, `emailVerified`, `status`. The screen renders the display
  names, the email, the verified flag and the status; `userId` is a cache/key value and is never
  rendered, logged or copied into an error. The provider subject, its hash and its ciphertext are
  never part of the projection.
- **Errors render `title` + `status` + `correlationId` only.** `ProblemDetails.detail` is
  server-facing and is never rendered, so a server message that names a record cannot reach the
  screen. The unit does **not** reuse the shared `ErrorPanel` where that component renders
  `detail`; its error surfaces are the spec's own inline/ full-page components.
- **User-controlled text is text.** Display names render through React's text path; no
  `dangerouslySetInnerHTML` anywhere in this flow. Length and content rules are enforced server
  side; the client's 1–200 check is feedback only.
- **The delete page collects no free text** (no reason field), so there is no unvalidated input
  and no personal data in a reason.
- **The "what is kept" text is fixed.** It renders a fixed i18n sentence and the server's
  `retainedCategories` when present; it invents no category and infers no retention rule.

## 6. Input validation design (presentation layer)

| Field | Client rule (feedback only) | Server truth |
|---|---|---|
| Display name | trimmed, 1–200 characters; inline error tied by `aria-describedby` | P2's `422` with field errors; the client maps `errors[]` to fields and keeps the value |
| Locale | one of `en`/`vi` | X2's own answer; the client renders the server's `source` rather than deriving a rule |
| Base currency | one of the supported ISO codes the service reports | X2 |
| Consent switches | none beyond the failed write | X5/X6 |

Every client rule mirrors a server rule for feedback; none of them substitutes for it.

## 7. Non-disclosure design

- Another user's record is never addressable from this flow: no settings route carries a user id.
  Where a record is addressed by its own id (the deletion poll, the export poll), a non-owner
  answer is `404` without disclosure (FR15.6).
- The blocked screen says the status the server reported and offers sign-out; it does not explain
  *why* an account was disabled (that is support's surface, U3/U9).
- D1's `404` ("no open request") is indistinguishable from a non-existent request by design; it
  carries the pinned no-record problem type (§14.4) so the client can treat it as data while any
  other `404` stays an error.

## 8. Destructive-action design (NFR24.5, NFR24.6)

- **Two-step intent.** `DeleteAccountPage` states the consequences and requires an explicit
  confirmation dialog (`ConfirmDialog`, extended in place per T5) with the confirm action
  labelled for the action; no destructive action fires from a single click.
- **Single-flight.** The confirm control is disabled while the mutation is in flight and the
  dialog cannot be re-submitted, so a double click issues one D3 request. If the answer is lost
  (deadline), the surface says so and re-reads D1 instead of assuming success.
- **Server-side idempotency of the state, not a key.** A second D3 while a request is open
  answers `409`, which the screen renders as the existing pending state; a second D2 after the
  window answers `410`, which is terminal. The product therefore never needs an idempotency key
  for these two calls — the state machine is the deduplication.
- **The recovery date is the server's.** `recoveryEndsAt` is rendered exactly (formatted by
  `Intl` in the profile locale/timezone); the client never computes the deadline or decides
  whether the window is open.
- **Focus discipline.** The dialog moves focus into itself, traps Tab, closes on Escape and
  returns focus to its trigger (NFR25.4), so a keyboard user can cancel without losing their place.

## 9. Freshness and server-side enforcement (NFR24.3, NFR24.4)

- Status is re-read on every authenticated entry (`staleTime: 0` for D0) and re-decided server-side
  on every request. The client's cached status only decides which screen renders; it never
  authorises anything.
- Every write is scoped server-side to the caller's own row: P2 edits the caller's profile, D2
  cancels the caller's own request, D3 creates a request for the caller. No write takes an actor
  id from the browser.
- The blocked and recovery screens are reachable by a denied account only for the three self-reads
  and the two bounded deletion paths of §2 — nothing else on the settings surface is.

## 10. Rate limiting and abuse (NFR11.1, NFR11.2)

**What the gateway actually enforces, stated exactly.** The limiter has two windows
(`services/gateway/http/src/rate_limit.rs`). The default bucket (`check`) keys on the forwarded
client IP plus whether a bearer token is present, so it is per-address, not per-person. A declared
`max_requests_per_minute` is enforced by `check_route`, whose bucket is the **route label plus the
SHA-256 of the bearer token** — the token is never stored as a key, and the code's own note says
two tokens for the same person are separate budgets. Neither window is keyed on the validated `sub`.

**The decision for D3.** Declare `max_requests_per_minute: Some(5)` on
`POST /v1/privacy/deletions`, and **accept the per-token window as meeting NFR11.1's intent**, with
the reasons written down so the acceptance is a decision and not an oversight:

- The address window still runs on every request, so rotating tokens from one client cannot multiply
  the budget: the per-address limit binds regardless of how many tokens the caller mints.
- The action is performed once. A second session or a re-issued token starting a fresh 5/min budget
  grants nothing an honest person needs and gives a script three attempts per minute at most across
  its own tokens, all inside the address window.
- It needs no new machinery: one route declaration using the existing field.

**The alternative if a true per-user window is required.** Key `check_route` on the validated
subject (`sub`) instead of the token hash. That is a change to the gateway's limiter and middleware
order — the bucket would have to be filled after JWT validation, not at the edge — so it is a
gateway unit's change, not this unit's, and it is recorded in §14.5 rather than assumed. NFR11.1's
wording ("5 requests per minute per user") is therefore met in effect, and this paragraph is the
recorded reading; if the team wants the literal keying, §14.5 is the work item.

**Client side.** No mutation is auto-retried, so a `429` cannot be amplified by the client; it
renders as a recoverable inline error with the correlation id and the person retries by hand
(NFR11.2). The settings screens add no polling, so their read volume is what a person's navigation
produces, bounded by the existing default windows.

## 11. Audit (NFR21.1)

The unit writes no audit record and holds no audit content. Every mutation it triggers is audited
server-side by Identity under the existing rules. The unit's obligation is that the mutation
actually reaches the service through the gateway (so it can be recorded) and that no sensitive
identifier leaves the browser (§5).

## 12. Compliance controls

### Accessibility (NFR25.1–25.6)

- Labelled `nav` with `aria-current="page"`; one `h1` per route; keyboard order matches DOM order.
- Switches are `role="switch"` with `aria-checked` and a text state beside them.
- Field errors are tied by `aria-describedby`; the save confirmation and the recovery banner are
  `role="status"`/`aria-live="polite"`.
- The delete dialog traps focus, closes on Escape and returns focus (NFR25.4).
- `StatusPill` always pairs colour with a word and a dot shape; the pending-deletion date is text
  beside it.
- Every rendered state passes jest-axe in the unit's suite — the a11y check is a test gate, not a
  review item.

### Localisation (NFR26.1–26.3)

- Every string exists in English and Vietnamese in the existing `web/src/i18n` catalogues.
- Resolution is the shell's existing order (`?lang=` → stored preference → browser → English);
  the panel edits the stored preference through X2 and shows the server's per-field `source`.
- Dates and numbers go through `Intl`; the recovery date is an exact formatted date, never a
  relative string alone.

## 13. Threat → mechanism map

| Threat | Mechanism |
|---|---|
| A denied account is stranded with no recovery | D0/D1/D2 self-scoped, bounded by §2; the recovery screen is reachable for exactly those reads |
| A widened bypass reaches other data | Bound types are enumerated (NFR24.7); the payloads carry status/own request only; the predicate is untouched |
| Another user's record reached through a settings route | No user-scoped route carries an id; id-addressed routes (`deletionId`, `exportId`) are ownership-checked and answer `404` |
| Identifier leakage into edge logs | Self-scoped paths; the export id names a job, not a person |
| A server message leaking record detail to the screen | Only `title`/`status`/`correlationId` rendered; `detail` never rendered; no reuse of a detail-rendering panel |
| Stored credentials or personal values in the browser | Memory-only cache, no storage API, preferences persisted server-side |
| A forged client status opening a screen | Server re-decides every request; the guard is presentational |
| Double deletion, double restore, or an assumed success | Single-flight controls + the state machine (`409`/`410`) + a re-read after a lost answer |
| A scripted burst of deletion triggers | One-route rate limit (5/min/user) at the gateway; no client auto-retry |
| XSS through a display name | Text rendering only; no `dangerouslySetInnerHTML`; server-side length/content rules |

## 14. Recorded security dependencies (must be pinned before Code Generation)

1. **Gateway entries for D0, P1, P2, D1, D2 and D3.** The gateway's deny-by-default table declares
   none of `/v1/me/account-status`, `/v1/me/profile`, `/v1/me/deletion-request` or
   `/v1/privacy/deletions` today, so without them no settings call can reach Identity.
2. **Gateway entries for `POST /v1/exports` and `GET /v1/exports/{exportId}`.** Identity already
   serves both; the gateway declares neither, so the privacy panel's export control has no path.
3. **A per-route budget of 5/min on D3** (NFR11.1), declared through the existing
   `max_requests_per_minute` field — keyed per bearer token, with the address window still running
   (§10 states what that does and does not bound).
4. **D1's no-record `404` must carry the pinned problem type**
   `https://crypto-pocket-butler.example/problems/no-record`. Identity's `not_found` renders
   `https://errors.cpb/identity/not-found` for every `404`; without the pinned type the empty state
   renders as an error (`performance-design.md` §3, T6).
5. **If NFR11.1 is read as a literal per-user window**, the work is a gateway change: key
   `check_route`'s bucket on the validated subject rather than the bearer-token hash, which means
   filling the bucket after JWT validation instead of at the edge (`rate_limit.rs`, `middleware.rs`).
   Not expected for this unit — §10 records why the per-token window with the address window behind
   it meets the intent — but named so the choice is visible rather than assumed.
6. **The contract summary's `/v1` route rows.** D0/P1/P2/D1/D2/D3 and the two export routes are
   SPA-facing and belong to **C1/C2**, which govern the browser-to-gateway edge; C6 and C7 are the
   internal `/internal/...` service-to-service edges, and no internal edge changes in this unit.
   The functional spec's open item 1 should be read the same way (its own §12 open item 1 says "C3
   upstream list, C6 and C7" and needs the same correction).