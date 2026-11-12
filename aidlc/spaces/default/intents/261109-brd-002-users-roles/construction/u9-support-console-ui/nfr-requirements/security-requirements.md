# Security Requirements — u9-support-console-ui

**Upstream:** `functional-spec.md` (§2.1–§2.4 journeys, §3.1–§3.4 screen rules, §4 W1–W7,
§6 accessibility, §7 security posture, §8 S1–S6, §10 error handling, §12 open items),
`frontend-components.md` (§2 types and props, §3 client and hooks, §5 accessibility and status
presentation), `requirements.md` (FR15.4–FR15.9, FR16.1–FR16.6, NFR21–NFR26), `contract-summary.md`
(C1/C2, C5, C9; the RFC 7807 envelope), codekb `technology-stack.md`.
**Prefix note:** `NFR9` (transport and secret protection), `NFR10` (token and session security)
and `NFR11` (abuse protection) inherit the BRD-001 intent's prefixes and refine them for this UI
unit; `NFR22`, `NFR24`, `NFR25` and `NFR26` refine this intent's inception NFRs. Sub-numbers are
unit-local.

## Transport and session security

| ID | Requirement | Source |
|---|---|---|
| NFR9.1 | Every U9 call goes over TLS to the API Gateway only. The console's other browser-external origins are the shipped ones — Supabase Auth, and the Google Fonts origins (`fonts.googleapis.com`, `fonts.gstatic.com`) loaded by `support-console/index.html:12-15` — and **this unit adds none**: no CDN data, no analytics, no font fetch of its own, no socket. Every route it calls is a `/v1` gateway path | BRD-012 NFR-SEC-01; project mandate (console → gateway only); `support-console/index.html` |
| NFR9.2 | The reason, the target user id, the grant id and every projection field travel in the JSON body or the path the contract defines — never in a query string for S1/S2/S3, never in a header other than `Authorization` and `Content-Type`, and never in a referrer or client telemetry. The console **stops sending `X-Support-Reason`**: the gateway's upstream client currently forwards that header (`services/gateway/infrastructure/src/upstream.rs:88-90`) and `support-console/README.md:33` still documents it, but no Identity handler reads it. The README and the upstream comment are corrected in the same change that removes the console's use, so no stale claim remains | BRD-012 NFR-SEC-03; spec §4 W3, §7, §12 item 8 |
| NFR10.1 | The session token stays in memory only: U9 writes nothing to `localStorage`, `sessionStorage`, IndexedDB or a cookie — not the token, not the grant id, not the reason draft, not the filter text, not the projection. The console's existing sign-in already holds the token in `src/auth/session.ts`, and its suite asserts no storage writes | BRD-012 NFR-SEC-10; spec §7 |
| NFR10.2 | The console never validates, decodes or introspects the JWT for a decision. It reads the session's `app_metadata.role` only to decide **what to show** — the request form for `support`, the metadata section only for `support` — and every route is authorised again server-side from the forwarded token. Hiding a section is not a control (FR15.9; spec §3.1, §3.4) | FR15.9; spec §3.1 |
| NFR10.3 | Every U9 route is declared in the gateway's deny-by-default table and re-checked in Identity from the same forwarded JWT. S1 and S3 already work this way; S2 (`GET /v1/support-grants/mine`) is declared with no admin requirement and requires the **support role** in Identity, which refuses any other role with `403`. No U9 path reaches a service directly, and no path is service-token-only from the browser | project mandate; C1/C2; spec §4 W4 |
| NFR10.4 | **S2 is called only by a support-role session.** The shell gates `useMyGrants` on `role === "support"`; an admin session never mounts the hook, so the guaranteed admin `403` the design review found cannot occur. If an S2 `403` arrives anyway (a tampered role claim or a role change mid-session), the shell shows one line with the correlation id and **must not wipe another surface's cache**. The scoping follows the shipped code instead of reversing it: a `401`, sign-out, grant expiry or revoke keeps the **full** `client.clear()` in `support-console/src/api/queryClient.ts` — that is NFR22.5's guarantee and `support-console/README.md` states it — while a **route-level `403` from S2 or S3** clears only the support query keys, so an admin's `["admin", …]` queries survive it. `queryClient.ts` and the `useClearSupportCacheOnErrors` hook are therefore **changed** by this unit, not comment-only | spec §4 W4, §10; `support-console/src/api/queryClient.ts`; `support-console/README.md`; its line 29 sentence is corrected in the same edit as line 33 (tech-stack §3) |
| NFR11.1 | The gateway budgets are the shipped ones: **S1** `POST /v1/support-grants` at 20/min per token and **S3** `GET /v1/support-grants/{grantId}/metadata` at 60/min per token. **S2 carries no per-route budget**: a route with no `max_requests_per_minute` rides the gateway's shared per-caller-address budget, not a per-token one. The NFR record states that model rather than inventing a per-token limit the route table does not declare. Adding a per-route limit for S2 is optional and, if chosen, lands with the route | BRD-012 NFR-SEC-12; `services/gateway/http/src/middleware.rs:157-181`, `services/gateway/domain/src/route.rs:61-63` |
| NFR11.2 | The limiter is in-process per replica, so the effective ceiling is per replica — stated as the honest bound rather than claimed global. A `429` is rendered as its own state and never retried automatically (NFR2.8) | `services/gateway/http/src/rate_limit.rs` |

## Data protection and sensitive identifiers

| ID | Requirement | Source |
|---|---|---|
| NFR22.1 | No U9 response or rendered surface carries the provider subject, a subject hash, a provider or Supabase token, a wallet address, an exchange credential, or the target's email address. The projection is exactly `userId`, `displayName` (nullable), `emailVerified` (a boolean, never the address) and the `accounts[]` / `portfolios[]` summaries; every excluded class — holdings, balances, valuations, transactions, orders, credentials, keys, seed phrases — has no route and no field | NFR22; FR16.4; spec §3.3, §7 |
| NFR22.2 | The projection is whitelisted in the console (`SUPPORT_METADATA_FIELDS`): a field the server over-sends is dropped by `pickSupportMetadata`, which throws on a malformed projection rather than rendering a partial or widened one. No component receives a raw response object | BR1.4; AC3.3.1/AC3.3.2; spec §3.3, §7 |
| NFR22.3 | The request reason reaches the server in the JSON body only, is never pre-filled, never echoed back into rendered copy beyond the grant's own reason heading, and never enters a URL, a header, a log, a telemetry event or browser storage. It lives in component state and is cleared on success | NFR22; BRD-012 audit-logging rules; spec §3.1, §7 |
| NFR22.4 | Error surfaces carry the HTTP status and the correlation id only. `ProblemDetails.detail` is never rendered, and no audit-record id is shown. **S1 has two 403 causes** — the session is not a support agent (`services/identity/application/src/support.rs:107-112`), or the target is inaccessible because the account is pending deletion (`support.rs:128-136`) — and both render one neutral line, "This request is not allowed.", so the screen does not disclose which check refused it. The typed values are kept for both | NFR22; spec §3.1, §10 |
| NFR22.5 | The projection, the grant, the reason draft and the filter live in memory only. When access ends the shell clears the support query cache (NFR2.10); sign-out clears the cache and the session as it does today. No grant id or projection survives a reload — the `mine` re-read is what recovers the session's grant (W4) | NFR10.1, NFR22; `support-console/src/api/queryClient.ts`; spec §3.3 |
| NFR22.6 | Every server string (display name, account label, portfolio label) is rendered as text by React's default escaping; no U9 surface renders server text as HTML (`dangerouslySetInnerHTML` nowhere in this unit), so a crafted label cannot inject markup | NFR22 |
| NFR21.1 | **N/A — no audit record is written or read here.** Every metadata read is audited by U3 into C9 with the six-month retention the audit service owns. This unit's obligation is the visible statement "This read is recorded in the audit log." and never rendering audit content, an audit id, or retained personal data itself | NFR21; spec §3.3; C9 |
| NFR23.1 | **N/A — no data layer.** This unit holds no schema and no Postgres role; its one new route reads through Identity's grant store under Identity's own role. Least privilege at the data layer is U1/U3's record | NFR23; spec §4 W4 |

## Authorisation and freshness (NFR24)

| ID | Requirement | Source |
|---|---|---|
| NFR24.1 | The grant status the console shows is the server's answer from the contract's four statuses — `pending_approval`, `active`, `expired`, `revoked`. C5's `denied` has no producer in U3's domain; modelling it would add a dead branch (spec §12 item 3). The console renders the other four only | NFR24; spec §2.4, §12 |
| NFR24.2 | The server is authoritative on every read. The local status is a clock-derived view of the last answer; an `active` grant whose `expiresAt` is null or unparseable is treated as **expired** (fail closed). A read after expiry or revoke is refused `403` by the server and the screen marks access ended — the console never claims access the server would refuse | NFR24; spec §2.3 step 5, §2.4, §3.3 |
| NFR24.3 | Every client-side gate (hidden request form, disabled reads, "Check status") is presentational (FR15.9). A tampered client that calls S1 as a non-support role or S3 under a dead grant receives `403` from the server, rendered as the access-ended or not-allowed state with the correlation id — never as an empty projection or a silent failure | FR15.9; spec §3.1, §10 |
| NFR24.4 | The role and status the console shows come from the signed token and the server's answers at request time (NFR24): a role or status change takes effect at the next request, and no surface caches a role it would act on. The grant's own 60-minute window is computed by the server at approval; the console displays it and never extends it | NFR24; BR4.1; spec §2.2, §4 W4 |

## Threat considerations (unit-specific)

| Threat | Mitigation in this design |
|---|---|
| A support agent reading data beyond the grant's target or window | S3 checks the support role, that the grant is held by the caller, active, and the target accessible, all server-side; the console's disabled state is only presentation (NFR24.2/NFR24.3) |
| An admin session tripping a support-only route and losing its own cache | `useMyGrants` is gated to support sessions; an S2 `403` does not clear the admin cache (NFR10.4) |
| A stale or forged client claim opening the support surface | Every route re-checks the role from the forwarded JWT; the client gate is presentational (NFR10.2/NFR24.3) |
| An over-sent projection field leaking holdings or identifiers | The console whitelist drops unknown fields and throws on a malformed projection (NFR22.2); the server's projection is already closed |
| The reason leaking through a URL, header or log | The reason is body-only; `X-Support-Reason` is retired and its stale documentation corrected (NFR9.2, NFR22.3) |
| Credential or token theft from browser storage | Memory-only session and no storage writes (NFR10.1, NFR22.5) |
| Cross-site scripting through a display name or account label | React's escaped rendering; no HTML injection point (NFR22.6) |
| A duplicate request creating two grants | The server refuses a second live grant with `409`; the console sends one attempt and offers **Check your current grant** rather than retrying (NFR2.7; spec §3.1) |

## Compliance — accessibility (NFR25)

| ID | Requirement |
|---|---|
| NFR25.1 | Every U9 section meets **WCAG 2.2 AA**: a labelled region with a visible heading, full keyboard operation, visible focus on every control, and no motion-dependent interaction. The accounts table has a `<caption>` and `scope="col"` headers |
| NFR25.2 | Every input has a visible label and its error is tied with `aria-describedby`; the request result, the status change and the metadata read completion are announced in a `role="status"` live region; an access-ended or failed state uses `role="alert"` |
| NFR25.3 | Colour never carries meaning alone: `StatusBadge` renders a **label plus a tone and a shape** (pending = warn/diamond, active = success/dot, expired = danger/square, revoked = danger/ring), and the urgency of a nearly expired grant is carried in text ("less than a minute left"), not colour |
| NFR25.4 | The metadata filter and every control are operable by keyboard; the banner and the metadata section keep their reading order, so the metadata-only reminder is heard before the projection |
| NFR25.5 | Every rendered state in every U9 suite passes `jest-axe` — accessibility is a test gate in this unit (`frontend-components.md` §6), not a review checklist item |

## Compliance — localisation (NFR26)

| ID | Requirement |
|---|---|
| NFR26.1 | Every date and time the banner and the projection show (`expiresAt`, `lastSyncedAt`) is formatted with `Intl.DateTimeFormat` in the console's locale; the remaining-minutes text is a whole number and "less than a minute left" replaces `0`. No date is rendered as a raw timestamp or a bare epoch value |
| NFR26.2 | **The English/Vietnamese half of NFR26 is deferred for this surface, and it is recorded as deferred rather than as met.** The support console has no i18n module: every existing section renders English string literals, the SPA's i18n does not reach this package, and `functional-spec.md` §6/§12 item 6 records the same fact. U9 adds its copy the way the console already does and does not fork a second localisation mechanism. The deferral is **tracked** as decision `ab46b27b` in this intent's decision log, which names the owner — a console-wide localisation unit, not yet in `unit-of-work.md` — so it cannot be lost without a trace; it is the same deferral U7 and U10 recorded. Until that unit lands, NFR26 is **not** satisfied for the console |
| NFR26.3 | Locale-dependent formatting degrades safely: where a locale is unavailable, `Intl` falls back to its default rather than rendering an empty or malformed date, and every status is rendered as a word from the presentation mapping, never as a raw enum value |

## Out of scope for this unit

Audit writes and retention (NFR21, C9 — U3 owns them), the Postgres role model and the grant store
(NFR23 — U1/U3), JWKS validation and the grant decisions themselves (C4/C5 — U2/U3), the gateway's
rate-limit configuration beyond the routes this unit calls, and the Account/Portfolio fan-out. This
unit's obligation is never to undermine them and to fail loudly when a server-side answer says it
cannot proceed.