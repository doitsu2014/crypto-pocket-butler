# Security Requirements — u10-admin-users-ui

**Upstream:** `functional-spec.md` (§2.1 gating, §3.1–§3.5, §4 W1–W5, §6 accessibility, §7 security
posture, §8 routes, §10 error handling, §12 open items), `frontend-components.md` (§2 types, §3
client and hooks, §5 accessibility), `requirements.md` (FR15.4, FR15.9, FR17.2, FR20.1–FR20.4,
NFR21–NFR26), `contract-summary.md` (C1/C2, C3, C4, C5, C7, C9; the RFC 7807 envelope),
codekb `technology-stack.md`.
**Prefix note:** `NFR9` (transport and secret protection), `NFR10` (token and session security)
and `NFR11` (abuse protection) inherit the BRD-001 intent's prefixes and refine them for this
UI unit; `NFR22`, `NFR24`, `NFR25` and `NFR26` refine this intent's inception NFRs. Sub-numbers
are unit-local.

## Transport and session security

| ID | Requirement | Source |
|---|---|---|
| NFR9.1 | Every U10 call goes over TLS to the API Gateway only. The console's sole other browser-external origin is Supabase Auth; this unit adds no third-party origin (no CDN data, no analytics, no font fetch), and every route it calls is a `/v1` gateway path | BRD-012 NFR-SEC-01; project mandate (console → gateway only) |
| NFR9.2 | The free-text reasons, the looked-up address and the record's fields never appear in a URL query or path, in a request header other than the JSON body's content type, in a referrer, or in client telemetry. Reasons travel in the JSON body (spec §7). The edge's default access-log format records the URL and never a `$http_*` header (`web/nginx.conf`), so a field that moves into the URL is a field in the log — NFR22.3 carries the one place the current contract still does that | spec §7; `web/nginx.conf` |
| NFR10.1 | The session token stays in memory only: U10 writes nothing to `localStorage`, `sessionStorage`, IndexedDB or a cookie — not the token, not the looked-up address, not the record, not a reason draft. The console's existing sign-in already holds the token in `src/auth/session.ts` and its suite asserts no storage writes | BRD-012 NFR-SEC-10; spec §7 |
| NFR10.2 | The console never validates, decodes or introspects the JWT for a decision. It reads the session's `app_metadata.role` only to decide **what to show** (the section is offered to `admin`), and every route this unit calls is authorised again server-side from the forwarded token. Hiding the section is not a control | FR15.9; spec §2.1 |
| NFR10.3 | Every U10 route is `requires_admin: true` in the gateway's deny-by-default table **and** re-checked in Identity from the same forwarded JWT (A1′, A5′ and the existing A2 already work this way; D1/D2 join them — spec §4 W2/W3). No U10 path reaches a service directly, and no path is service-token-only from the browser | project mandate; C1/C2; spec §4 |
| NFR11.1 | The three U10 mutations carry their own per-token budgets in the gateway route table — `max_requests_per_minute: 20` on `PUT /v1/admin/users/{userId}/role`, `POST …/disable` and `POST …/restore` — set in the same `Route` declarations this unit adds or widens. 20/min matches the gateway's existing strictest *creation* trigger (`POST /v1/support-grants`, 20/min); the 5/min deletion trigger stays reserved for the irreversible erasure path | BRD-012 NFR-SEC-12; `services/gateway/domain/src/route.rs` |
| NFR11.2 | The two U10 reads carry `max_requests_per_minute: 60` — the admin lookup `GET /v1/admin/users` and the pending-grant list `GET /v1/support-grants` — matching the gateway's existing 60/min metadata read. The lookup is the console's one path that can enumerate accounts one address at a time, so it is bounded on its own rather than left on the shared budget; the list is bounded because it answers up to 100 grant rows per call | BRD-012 NFR-SEC-12; spec §4 W5 |
| NFR11.3 | The limit is per token, not per IP (`check_route(route, token, limit)`): one administrator's runaway client cannot spend another's budget, and the per-IP budget still applies underneath. A `429` is rendered as its own state and never retried automatically (NFR2.8). **The gateway's limiter is in-process per replica**, so the effective ceiling is per replica — stated here as the honest bound rather than claimed as a global one | `services/gateway/http/src/rate_limit.rs` |

## Data protection and sensitive identifiers

| ID | Requirement | Source |
|---|---|---|
| NFR22.1 | No U10 response or rendered surface carries the provider subject, a subject hash, a provider access token, a wallet address, an exchange credential, or any other provider identifier. The record's identifiers are the internal user id (shown so it can be quoted in a ticket) and the email with its verified flag; spec §3.2's metadata boundary names every excluded class (holdings, balances, valuations, transactions, orders, credentials, tokens, subscriptions, entitlements) | NFR22; spec §3.2, §7 |
| NFR22.2 | Reasons reach the server in the JSON body only, are never pre-filled, never echoed back into the rendered copy (the forms state the transition and that it is audited, not the reason text), and never enter a log, a telemetry event, a URL or a header. The console's existing rule for the support read reason (`X-Support-Reason` documented in `web/nginx.conf` as never-to-be-logged) is the same rule applied to the body | NFR22; spec §7; BRD-012 audit-logging rules |
| NFR22.3 | **The admin lookup must not carry the address in a logged URL.** A1′ is `GET /v1/admin/users?email=<address>`, and the edge's default access-log format records the request URL, so a looked-up address lands in the edge log today — the same class of exposure the console's read reason was moved out of the URL to avoid. The required change is a body-carrying lookup (`POST /v1/admin/users/lookup` with `{email}`) **or** an edge log-redaction policy scoped to that route; it must land before Code Generation (tech-stack §3). If the team instead accepts the exposure, that acceptance is recorded explicitly as a deviation from NFR22 with its reason — it is not assumed away, and the console does not silently rely on it | NFR22; spec §8 A1′; `web/nginx.conf` |
| NFR22.4 | Error surfaces carry the HTTP status and the correlation id only. `ProblemDetails.detail` is never rendered (it can hold server wording and identifiers), and no audit-record id is shown; spec §3.1 states the console shows no audit id even though the lookup is audited | spec §3.1, §7 |
| NFR22.5 | The looked-up record, the address and any reason draft live in memory only, and the console discards them deliberately: the section removes its `["admin"]` queries on unmount (`frontend-components.md` §3.2), and the console's existing cache clear on 401/403, sign-out, grant expiry and revocation drops them too — a support session that loses its grant cannot keep reading the last admin record from cache | NFR10.1, NFR22; `support-console/src/api/queryClient.ts` |
| NFR22.6 | The email the record shows for identity confirmation is rendered as text by React's default escaping; no U10 surface renders server text as HTML (`dangerouslySetInnerHTML` nowhere in this unit), so a crafted display name or address cannot inject markup | NFR22; spec §3.2 |
| NFR21.1 | **N/A — no audit record is written or read here.** The role change, the status transitions and the lookup's audit are U2's, U5's and U1's, written server-side into C9 with the six-month retention the audit service owns. This unit's obligation is the two visible statements: it states that a change is audited, and it renders no audit content, no audit id and no retained personal data itself | NFR21; spec §3.1, §3.3; C9 |
| NFR23.1 | **N/A — no data layer.** This unit holds no schema and no Postgres role; its three new routes read and write through Identity's existing use cases and repositories under Identity's own role. Least privilege at the data layer is U1/U3/U5's record | NFR23; spec §4 |

## Authorisation and freshness (NFR24)

| ID | Requirement | Source |
|---|---|---|
| NFR24.1 | The role the record shows is the **provider's current `app_metadata.role`, read at lookup time** (spec §4 W1), never a stored copy: the stored `role` column is refreshed only when the target's own principal is next resolved, so it is stale on exactly the screen used to judge access. A provider read failure answers `503`, and the section then renders "this user cannot be looked up right now" with a retry — **it never falls back to a stored or cached role** | NFR24; spec §3.2, §4 W1, §10 |
| NFR24.2 | Status is the server's answer, never the client's assumption. No mutation seeds the cache optimistically or writes the record itself; every success re-reads, and every ambiguous answer (`409` for a moved edge, `503` for an unrecorded audit or an uncertain provider write) re-reads before rendering and reports what the server says, with the correlation id — the console never claims that nothing changed | NFR24; spec §3.3, §3.4, §10 |
| NFR24.3 | The section's visibility and every client-side check are presentational (FR15.9). A tampered client that opens `#admin-users` for a `support` session receives `403` from the first call it makes, rendered as the not-an-administrator line with the correlation id — never as an empty list or a silent failure | FR15.9; spec §2.1, §10 |
| NFR24.4 | The target id a mutation names is the **internal user id from the record the server answered**, never the console session's Supabase user id (the two id spaces differ, spec §3.3). This unit adds no id translation in the browser: mis-targeting a privileged change is not a client risk it is willing to own | spec §3.3; C3 |

## Threat considerations (unit-specific)

| Threat | Mitigation in this design |
|---|---|
| Account enumeration by a compromised admin session | Exact-email lookup only, no list and no prefix search (spec §12 item 2); every lookup audited (`identity.admin.user_lookup`, allowed and refused); a 60/min per-token budget (NFR11.2); `404` and `403` are distinguishable but neither discloses more than the admin could obtain anyway |
| A stale role misleads the admin judging access | The provider is read at lookup time and a failed read is `503`, never a stale value (NFR24.1) |
| Self-demotion or self-disable locking the last administrator out | No server rule exists (spec §12 item 3), so the console warns when the looked-up address equals the signed-in one and states that the change takes effect at the next request. Recorded as a policy question for the authorisation unit, not silently "fixed" in the browser |
| Approvals panel showing only the session's own requests | The panel reads the pending-grant list (W5) and treats a failed read as an error with a retry, never as "nothing pending" (NFR2.5, spec §10) |
| Audit content leaking through the UI | Reasons are never rendered back, no audit id is shown, `ProblemDetails.detail` is never displayed (NFR22.2, NFR22.4) |
| Cross-site scripting through a display name, address or reason | React's escaped rendering; no HTML injection point in the unit (NFR22.6) |
| Credential theft from browser storage | Memory-only session and no storage writes (NFR10.1, NFR22.5) |
| Replayed or duplicated privileged change | The requests are not idempotent by nature; the console sends exactly one attempt and relies on the server's audited intent/outcome pair (C4/C9) rather than retrying — a duplicate privileged change is the worse outcome (the same reasoning as the provider writer's one attempt) |

## Compliance — accessibility (NFR25)

| ID | Requirement |
|---|---|
| NFR25.1 | The admin section, the record view, both change forms and the approvals panel meet **WCAG 2.2 AA**: full keyboard operation, visible focus on every control, a labelled region with a visible `h2`, `aria-current="page"` on the section link, each field error tied to its control with `aria-describedby`, and no motion-dependent interaction |
| NFR25.2 | The disable/restore confirmation block is operable without a pointer: focus moves to **Confirm** when it opens, Tab stays inside it while open, Escape cancels, and focus returns to the action that opened it on cancel or completion (spec §3.4, §6) — the focus behaviour the role change and the queue already get from ordinary controls |
| NFR25.3 | Colour never carries meaning alone: `RoleBadge` and `StatusBadge` each render a **label plus a shape** (ring, dot, square, diamond) from the console's `StatusPresentation` vocabulary, and the mapping for all five statuses and three roles is fixed in `frontend-components.md` §5.1 |
| NFR25.4 | A completed change is announced in a `role="status" aria-live="polite"` region without moving focus; a failure is a `role="alert"`; both carry their text, so a screen-reader user hears the outcome and the correlation id on a failure |
| NFR25.5 | Every rendered state in every U10 suite passes `jest-axe` — accessibility is a test gate in this unit (`frontend-components.md` §6), not a review checklist item |

## Compliance — localisation (NFR26)

| ID | Requirement |
|---|---|
| NFR26.1 | Every date and time the record shows (`createdAt`, `updatedAt`) is formatted with `Intl.DateTimeFormat` in the console's locale, and every status and role is rendered as a word from the §5.1 mapping — never as a raw enum value and never as an untranslated code |
| NFR26.2 | **The English/Vietnamese half of NFR26 is deferred for this surface, and it is recorded as deferred rather than as met.** The support console has no i18n module: every existing section renders English string literals (`support-console/src/`), and `functional-spec.md` §12 item 5 records the same fact. This unit adds its copy the way the console already does and does not fork a second localisation mechanism. When a console-wide i18n module lands, this section's strings move with it; until then NFR26 is **not** satisfied for the console |
| NFR26.3 | Locale-dependent formatting degrades safely: where a locale is unavailable, `Intl` falls back to its default rather than rendering an empty or malformed date, and no date is ever rendered as a raw timestamp or a bare epoch value |

## Out of scope for this unit

Audit writes and retention (NFR21, C9 — U1/U2/U5 own them), the Postgres role model (NFR23 —
U1/U3/U5), JWKS validation and the admin decision itself (C4 — U2 and Identity), the gateway's
rate-limit configuration beyond the per-route fields this unit declares, and the provider's own
availability. This unit's obligation is never to undermine them and to fail loudly when a
server-side answer says it cannot proceed.