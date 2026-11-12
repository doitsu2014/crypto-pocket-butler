# Security Requirements — u7-settings-ui

**Upstream:** `functional-spec.md` (this unit, as revised after review — §1 "what this unit does
not own", §2.1 entry routing, §3.5/§3.6 deletion and recovery, §7 security posture, §8 API
integration points, §11/§12), `requirements.md` (FR14.4, FR15.5, FR15.7, FR15.9, FR17.1–FR17.5,
FR19.1, FR20.3/FR20.4, NFR21–NFR26), `contract-summary.md` (C1/C2 gateway surface, C3, C6, C7;
RFC 7807 envelope; deny-by-default routing), codekb `technology-stack.md`.

**Prefix note:** `NFR9` (transport and secret protection), `NFR10` (token and session security)
and `NFR11` (abuse protection) inherit the BRD-001 intent's prefixes and refine them for this UI
unit; `NFR21`/`NFR22`/`NFR24` refine this intent's inception NFRs. Sub-numbers are unit-local.

## Transport, session, and client authority

| ID | Requirement | Source |
|---|---|---|
| NFR9.1 | Every call from the settings flow goes over TLS to the API Gateway only; the other browser-external origins are the ones the shell already contacts and this unit adds none of its own. The shell as built loads webfonts from `fonts.googleapis.com` / `fonts.gstatic.com` (`web/index.html`), permits the WalletConnect origins in the CSP (`web/nginx.conf`), and reaches Supabase Auth; this unit contacts no other origin, adds no CDN, no analytics, and no dataset fetch (the timezone list is `Intl`-built) | BRD-012 NFR-SEC-01; project mandate (SPA → gateway only); spec §7; `web/index.html`, `web/nginx.conf` |
| NFR10.1 | Browser tokens stay in memory only. Nothing in this flow writes a token, a deletion request id, a recovery date, or a consent state to `localStorage`, `sessionStorage`, IndexedDB, or a cookie; every value is re-read from the server. The settings screen legitimately persists the person's **preference values**, but only through the server (`PATCH`/`PUT` to `/v1/me/...`), never in browser storage | BRD-012 NFR-SEC-10; spec §7 |
| NFR10.2 | The SPA never validates, decodes, or inspects the JWT to make a decision. Status, role and ownership answers come only from server reads; the token is presented unchanged and the server is authoritative | FR15.9; spec §7 |
| NFR10.3 | Because authorisation travels as a `Authorization: Bearer` header and never as a cookie, this flow needs no CSRF token, and none is added. This is recorded so the absence is a decision, not an omission | C1/C2 (bearer JWT); spec §7 |

## Self-scoping and the non-denying reads (the unit's central security decision)

| ID | Requirement | Source |
|---|---|---|
| NFR22.1 | No U7 call carries a **user identifier** — an internal user id, an email, or a provider subject — in its path or query. Every user-scoped route is self-scoped (`/v1/me/...`, `/v1/privacy/deletions`) and identifies the caller from the validated token only. The edge logs URLs by default (`web/nginx.conf`), so an identifier in a path is an identifier in the logs, which NFR22 forbids. **Recorded exception:** the privacy panel polls `GET /v1/exports/{exportId}`, which carries an opaque export job id, not a user identifier; the server checks ownership of that job, and the id is worthless outside it | NFR22; spec §8; `web/nginx.conf`; `web/src/api/client.ts` (`getExport`) |
| NFR22.2 | The profile read (P1) answers exactly `userId`, `displayName`, `displayNameOverridden`, `providerDisplayName`, `email`, `emailVerified`, `status` — and never the provider subject, its hash, or its ciphertext. Of those, the fields the screen may render are the display names, the email, the verified flag and the status; `userId` is a key for cache identity and is never rendered, copied into an error, or logged. No settings surface renders, copies, or logs a subject, a subject hash, or an internal user id | NFR22; FR14.5; spec §8 P1; `frontend-components.md` §2 `ProfileView` |
| NFR22.3 | Error surfaces render the RFC 7807 `title`, the HTTP status and the `correlationId` only. `ProblemDetails.detail` is never rendered — it is server-facing text that can name a record or an internal reason — and no error payload is forwarded to analytics | NFR22; C1/C2 envelope; spec §10 |
| NFR22.4 | A display name is user-controlled data rendered as **text**, never as markup (`dangerouslySetInnerHTML` is not used anywhere in this flow). Length and content rules are enforced server-side; the client check mirrors them for feedback only | spec §5; construction-phase input-validation rule |
| NFR24.1 | The three self-reads this unit adds — D0 `GET /v1/me/account-status`, D1 `GET /v1/me/deletion-request`, D2 `DELETE /v1/me/deletion-request` — resolve their caller from the stored record by auth subject, using the same lookup the existing deletion handlers use (`caller_for_deletion`, `services/identity/http/src/handlers.rs`), so a `pending_deletion` or `disabled` person can learn their own status and recover without presenting any identifier. The lookup is **self-scoped by construction**: it accepts no id parameter, so it can never be pointed at another user's row | spec §1, §2.1, §3.6, §8 |
| NFR24.2 | The non-denying entry points on Identity are **exactly five**, and each one is bounded in the way NFR24.7 records: the three this unit adds (D0, D1, D2), and the two that already exist and which this unit's delete and poll surfaces use — D3 `POST /v1/privacy/deletions` (`request_deletion`, `handlers.rs`) and `GET /v1/privacy/deletions/{deletionId}` (`get_deletion`, `handlers.rs`), both of which already resolve through `caller_for_deletion`. None of the five leaks more than the caller's own status or own open request. `UserStatus::denies_use` (BR1.7) and its enforcement in `PrincipalService` are unchanged: every other endpoint still answers `403` for `disabled`, `pending_deletion` and `deleted`. This unit adds no allowance and no relaxation, and a change that widens the predicate — or that routes a sixth path around it — is a security regression, not an implementation detail | FR15.7; BR1.7; spec §1, §12 open item 4; `handlers.rs` |
| NFR24.3 | Status and ownership are decided server-side on every request. The SPA's route guard and every client-side check are presentational only (FR15.9): a client that forges a status, edits the cache, or calls a route directly still faces the server's denial. No security decision in this unit derives from client state | FR15.9; NFR24; spec §2.1 |
| NFR24.4 | Every write in this unit is scoped to the caller's own row by the server, not by a parameter the browser supplies: P2 edits only the caller's profile, D2 cancels only the caller's open request, D3 creates a request for the caller. A request that would act on another user's record is impossible by construction, and the read of another user's record remains `404` without disclosure (FR15.6) | FR15.5, FR15.6; spec §8 |
| NFR24.7 | Each of the five non-denying entry points carries its own bound, and each bound is a requirement, not an accident: **D0/D1/D2** answer only the caller's own status and own open request and take no id (NFR24.1); **D3** is bounded by the lifecycle state machine — `AccountLifecycle::request_deletion` accepts only the `Active → PendingDeletion` edge declared in `services/identity/domain/src/lifecycle.rs` (`EDGES`, BR5.1) and answers `409` for any other status, so a denied account gains nothing by calling it; **`GET /v1/privacy/deletions/{deletionId}`** is bounded by the ownership check inside `deletions.status(&caller, deletion_id)`, which answers `404` for a deletion that is not the caller's. Removing either bound, or adding a route that resolves through `caller_for_deletion` without one of these three shapes, is a security regression | BR1.7, BR5.1; `handlers.rs`; `lifecycle.rs`; FR15.6 |

## Data protection on the deletion and recovery surfaces

| ID | Requirement | Source |
|---|---|---|
| NFR22.5 | The delete page collects **no** reason and no free text: the request body is empty or carries only what the service defines, so there is no unvalidated input to sanitise and no personal data to leak from a reason field (Q5-A) | spec §3.5, §5 |
| NFR22.6 | The "what is kept" text is a fixed sentence and a fixed i18n key. While `retainedCategories` is empty the page renders the placeholder sentence and invents no category; when the service fills it, only the server's categories render. The page never infers a retention rule | spec §3.5, §12 open item 2 |
| NFR24.5 | The irreversible action is gated by an explicit confirmation dialog with a focus trap, and the request control is single-flight while in flight. `D3 409` ("already open") is treated as an existing state and re-read through D1 — never as a new success. `D2 410` is terminal and offers no further action; recovery after the window is a product decision the screen does not invent | spec §3.5, §3.6, §10 |
| NFR24.6 | The recovery date shown is the server's `recoveryEndsAt`, rendered exactly (Q6-A). The client never computes, rounds, or abbreviates the deadline, and never decides whether the window is still open | spec §3.6 |

## Abuse protection

| ID | Requirement | Source |
|---|---|---|
| NFR11.1 | The manual, irreversible trigger (D3) is rate-limited at the gateway more strictly than the authenticated default — **5 requests per minute per user**, against a gateway default of 120/min (`services/gateway/src/config.rs`). The number is chosen for the action, not copied from another route: deleting an account is done once, never in a burst, so a budget two orders of magnitude below the default costs a legitimate person nothing while making a scripted trigger useless. The gateway's own limited routes (20/min for support-grant creation, 60/min for its metadata read, `services/gateway/domain/src/route.rs`) are higher because those actions are cheaper and more repeatable. The SPA never auto-retries a mutation, so a client cannot amplify the trigger | BRD-012 NFR-SEC-12; `config.rs`; `route.rs`; spec §8 |
| NFR11.2 | A `429` renders as a recoverable inline error with the correlation id and is never looped on. The person waits and retries by hand | spec §10 |

## Audit (inherited, not owned)

| ID | Requirement | Source |
|---|---|---|
| NFR21.1 | This unit writes no audit record. Every mutation it triggers is audited server-side by the owning service (Identity) under the existing rules, and the settings flow never sends audit content from the browser and never renders an audit record. The unit's obligation is only never to undermine that: no sensitive identifier leaves the browser (NFR22.1–NFR22.3), and every mutation reaches the service through the gateway so the server can record it | NFR21; FR20.3; spec §8 |

## Threat considerations (unit-specific)

| Threat | Mitigation in this design |
|---|---|
| A denied account learning nothing and being stranded | D0/D1/D2 answer only the caller's own status and own request, self-scoped by auth subject; the recovery surface is reachable without weakening the denial on any other path (NFR24.1/NFR24.2) |
| Using the recovery path to reach other data | The non-denying reads return only status and the open deletion request. They grant no access to profile, preferences, consent or portfolio data, because those endpoints still apply the predicate |
| Existence disclosure through the settings flow | Every route is self-scoped and takes no identifier (NFR22.1); another user's record answers `404` without disclosure; error text is correlation-id only |
| Identifier leakage into logs | No user id, subject, hash, or email in a path or query (NFR22.1); `detail` never rendered (NFR22.3) |
| Forged client state unlocking a surface | Every decision is server-side (NFR24.3); the client guard is presentation |
| Double deletion or double restore | Single-flight controls plus the server's state machine: `409` on D3 and `410` on D2 are terminal answers the screen renders rather than retries (NFR24.5) |
| Stored credentials in the browser | Tokens in memory only; preferences persisted server-side (NFR10.1) |

## Compliance — accessibility (NFR25)

| ID | Requirement |
|---|---|
| NFR25.1 | Every settings screen meets WCAG 2.2 AA: the navigation is a labelled `nav` whose active link carries `aria-current="page"`, every control has a visible label, and the whole flow is operable by keyboard in DOM order |
| NFR25.2 | Every switch is `role="switch"` with `aria-checked`, and its state is also written in text beside it — never colour or position alone |
| NFR25.3 | Every error is tied to its field with `aria-describedby`; the save confirmation and the recovery banner are `role="status"` with `aria-live="polite"` so they are announced on arrival |
| NFR25.4 | The delete confirmation is a modal dialog that moves focus into it, traps Tab, closes on Escape, and returns focus to its trigger |
| NFR25.5 | Status is never colour alone: `StatusPill` pairs its colour with a word and a dot shape, and the pending-deletion date is text beside the pill |
| NFR25.6 | Every rendered state of the shell, the four panels, the delete page and both account-status screens passes jest-axe in the unit's test suite — accessibility is a build gate, not a review checklist item |

## Compliance — localisation (NFR26)

| ID | Requirement |
|---|---|
| NFR26.1 | Every user-facing string — labels, hints, errors, the delete consequences, the retained-categories placeholder, the recovery banner and the blocked notices — exists in English and Vietnamese through the existing `web/src/i18n` module |
| NFR26.2 | Locale resolution is the shell's existing order, unchanged: `?lang=` → the signed-in person's stored server preference → browser locale → English. The settings panel edits that stored preference; saving it changes the locale on the next render, and the panel shows the server's reported `source` for each field so the person can see where a value came from |
| NFR26.3 | Every date and number goes through `Intl` in the profile locale and timezone. The recovery date is an exact formatted date, never a relative "in N days" string alone, so the deadline cannot be misread across locales |

## Out of scope for this unit

Server-side authorisation decisions (U2), the deletion state machine and erasure (U5), rate-limit
configuration and route-table declarations (gateway), JWKS validation (Identity), and the audit
record itself (Audit service). This unit's obligation is never to undermine them: self-scoped
routes (NFR22.1), server-authoritative decisions (NFR24.3), and no client-side amplification
(NFR11.1).