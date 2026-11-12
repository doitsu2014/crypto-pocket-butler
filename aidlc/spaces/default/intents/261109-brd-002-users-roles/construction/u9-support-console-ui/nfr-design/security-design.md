# Security Design — u9-support-console-ui

**Upstream:** `nfr-requirements/security-requirements.md` (NFR9.1–9.2, NFR10.1–10.4, NFR11.1–11.2,
NFR21.1, NFR22.1–22.6, NFR23.1, NFR24.1–24.4, NFR25.1–25.5, NFR26.1–26.3),
`nfr-requirements/tech-stack-decisions.md` (T1–T8, §3 dependencies), `functional-spec.md`
(§2.1–§2.4 journeys, §3.1–§3.4 screen rules, §4 W1–W7, §7 posture, §8 S1–S6, §10 errors, §12 open
items), `frontend-components.md` (§2 types and props, §3 client and hooks, §5 accessibility),
`contract-summary.md` (C1/C2, C5, C9; RFC 7807).
**Unit kind:** ui — every authorisation decision is server-side (the gateway's route table,
Identity's JWT re-check and grant rules). This design makes the console **unable to undermine**
those controls and **unable to leak** what they protect.

## 1. Trust model and origins (NFR9.1, NFR9.2)

```
Browser (untrusted)                        Edge / backend (trusted)
┌────────────────────────────┐   TLS     ┌─────────┐         ┌─────────────────────────────┐
│ support console            │ ────────▶ │ Gateway │ ──────▶ │ Identity                    │
│  request form / banner /   │  JWT in   │ JWT val.│  route  │ S1 grant create (support)   │
│  metadata section          │  memory   │ route   │  table  │ S2 own-grant read (U3)      │
│  - renders server answers  │  only     │ budgets │         │ S3 metadata read (support)  │
│  - no decision, no loop    │           │         │         │ → Account / Portfolio fan-out│
└────────────────────────────┘           └─────────┘         └─────────────────────────────┘
   Supabase Auth (sign-in) · Google Fonts (pre-existing load-time origin, no data)
```

- **Origins.** The console's browser-external origins are the gateway (same-origin `/v1` through
  the edge), Supabase Auth, and the Google Fonts origins the page already loads
  (`support-console/index.html:12-15`; the edge CSP allowlists them). **U9 adds none** — no CDN
  data, no analytics, no telemetry, no other font — and every data call it makes is a `/v1` gateway
  path through `src/api/client.ts`.
- **No URL or header carries case data.** The reason is body-only; the target user id is a path
  segment only on the metadata read, where C5 defines it; the projection never travels in a query
  string. The console stops sending `X-Support-Reason` (§4).
- **No service is reachable from the browser** except through the gateway; the console builds no
  URL outside `client.ts`.

## 2. Session, token and cache hygiene (NFR10.1, NFR10.2, NFR22.5)

- **Memory-only session.** The section reuses `src/auth/session.ts`; it writes nothing to
  `localStorage`, `sessionStorage`, IndexedDB or a cookie — not the token, not the grant id, not
  the reason draft, not the filter text, not the projection. The existing no-storage assertion
  covers the module; U9 adds no storage call.
- **No token inspection.** The console never decodes or validates the JWT for a decision. It reads
  the session's `app_metadata.role` only to decide **what to show** — the request form for
  `support`, the metadata section only for `support` — and every route re-authorises from the
  forwarded token (NFR10.2, NFR24.3).
- **Cache clearing has two scopes, and the design follows the shipped code.** Today
  `clearSupportCache` calls `client.clear()` (`support-console/src/api/queryClient.ts:24-27`) and
  `useClearSupportCacheOnErrors` invokes it on any `401`/`403`. U9 keeps the **full** clear where it
  belongs — `401`, sign-out, grant expiry and revoke (NFR22.5's guarantee) — and adds a **scoped**
  clear for one case: a route-level `403` from S2 or S3 means *this grant* is not live, not *this
  session* is invalid, so it must not wipe an admin's `["admin", …]` cache in a session that is
  also signed in as an admin (NFR10.4).

```ts
// design sketch (≤15 lines) — implementation belongs to Code Generation
export function clearSupportCache(client: QueryClient): void {
  client.clear();                                  // 401 / sign-out / expiry: unchanged
}

export function clearSupportQueries(client: QueryClient): void {
  client.removeQueries({
    predicate: (q) => q.queryKey[0] === "support",  // a dead grant, not a dead session
  });
}
```

- `queryClient.ts` is a **changed** file: it gains `clearSupportQueries` beside the existing
  `clearSupportCache`, which is untouched.
- **Which caller uses which hook** — the separation is by caller, never by status, because a hook
  cannot tell a dead grant from a dead session by reading a `403`. `useClearSupportCacheOnErrors`
  is **unchanged**: every existing caller (U10's `useAdminUsers` at
  `support-console/src/hooks/useAdminUsers.ts:43,59` and the change forms) keeps the full
  `client.clear()` on `401` **and** on `403`, so an admin route's `403` still discards admin data
  exactly as it does today. The same file gains a second exported hook,
  `useClearSupportQueriesOnRouteErrors(error)`, wired only by the two U9 queries: `401` →
  `clearSupportCache` (the session is invalid, so everything goes), `403` → `clearSupportQueries`
  (this grant is not live, so only `["support", …]` goes). U9's queries do not use the old hook;
  no admin query uses the new one.
- **Nothing survives a reload.** The `mine` re-read (W4) is what recovers the session's grant; no
  grant id or projection is persisted (NFR22.5).

## 3. Route enforcement (NFR10.3, NFR11.1, NFR11.2, NFR24.3)

| Path | Gateway declaration | Identity |
|---|---|---|
| `POST /v1/support-grants` (S1) | declared as `limited_identity_route(Method::Post, "/v1/support-grants", false, 20)` — the gateway routes and budgets (`20`/min per token) and requires authentication; it has no support-role capability | re-checks the support role from the forwarded JWT (refusing `403`); refuses a second live grant with `409`; audits the request; refuses an inaccessible target (`403`) or an unknown one (`404`) |
| `GET /v1/support-grants/mine` (S2) | declared with **no per-route budget** — it rides the gateway's shared per-caller-address budget; authentication required, no role capability (built and owned by **U3**, tech-stack T1) | support role re-check; bounded to 20 rows; the role refusal is Identity's, not the gateway's |
| `GET /v1/support-grants/{grantId}/metadata` (S3) | declared, `max_requests_per_minute: 60` per token; authentication required, no role capability | support role re-check, grant held by the caller, grant active, target accessible; serves the complete projection or `503` |

- **The gateway has no role capability to claim.** `Route` carries only `requires_admin: bool`
  (`services/gateway/domain/src/route.rs:53-65`), and S1 ships with `false`, so any authenticated
  caller reaches Identity; the support-role decision is always Identity's re-check. Nothing in this
  unit asks the gateway for a capability it does not have.

- **Deny by default.** Every path is in the gateway's `Route` table; an undeclared path is `404`
  before a service is reached. No U9 path is service-token-only, and the browser never calls
  Identity directly.
- **S2's budget is not invented.** The route table has no per-route limit for it, so the honest
  statement is the shipped shared per-caller-address budget. Adding a per-route limit is optional
  and lands with the route (U3), not as a claim here.
- **The limiter is in-process per replica**, so the effective ceiling is per replica — stated as
  the honest bound, not as a global one (NFR11.2).
- **Presentational gating.** A tampered client that opens the support surface with a non-support
  role gets `403` from its first call, rendered as the not-allowed or access-ended state with the
  correlation id — never an empty projection and never a silent failure (NFR24.3).
- **Two `403` causes on S1, one neutral line.** The session is not a support agent
  (`services/identity/application/src/support.rs:107-112`), or the target is inaccessible because
  the account is pending deletion (`support.rs:128-136`). Both render "This request is not
  allowed." with the correlation id, so the screen does not disclose which check refused it
  (NFR22.4).

## 4. Design decision D-U9-1 — retire `X-Support-Reason`, and correct the two files that describe it

**Context.** `support-console/README.md:33` documents an `X-Support-Reason` header and
`src/api/client.ts` sends it, but the gateway's upstream client forwards it
(`services/gateway/infrastructure/src/upstream.rs:88-90`) and **no Identity handler reads it**. The
reason that matters is the one in the `POST /v1/support-grants` body, which is audited. A second,
unread reason channel is a header that looks like a control and is not one, and it invites a future
reader to trust it.

**Decision.** The console stops sending the header; every file that documents it as live is
corrected in the same change — `support-console/README.md:33`, `web/nginx.conf:7-9` (whose comment
still says "the free-text read reason travels in X-Support-Reason"), the comment at
`services/gateway/infrastructure/src/upstream.rs:88-90`, and `src/api/client.ts` itself by
construction, because the sending code is removed. No file then asserts a header the console no
longer sends. The reason travels in the JSON body only (NFR9.2, NFR22.3).

**Alternatives rejected.** (a) Keep the header "for future use" — rejected: an unread header is a
false affordance and a second place to leak a reason into a log. (b) Make Identity start reading it
— rejected: it would create a second path for the same data with no requirement behind it, outside
this unit's boundary.

**Ripple, which lands with Code Generation.** `support-console/README.md` lines 29 and 33 (the
cache sentence and the header), `web/nginx.conf:7-9` (the log-format comment), and the gateway's
comment at `services/gateway/infrastructure/src/upstream.rs:88-90` are corrected in one change; the
gateway comment is a comment-only edit owned by U9 (tech-stack §3). No gateway behaviour or test
changes.

## 5. Sensitive-data hygiene (NFR22.1, NFR22.2, NFR22.3, NFR22.4, NFR22.6)

- **The projection is whitelisted at the boundary.** `pickSupportMetadata` keeps exactly
  `userId`, `displayName`, `emailVerified`, `accounts[]` and `portfolios[]` and their declared
  fields; an over-sent field is dropped, and a malformed projection throws rather than rendering a
  partial one (NFR22.2). No component receives a raw response object.
- **Excluded classes have no field and no route**: provider subject, subject hash, provider or
  Supabase token, wallet address, exchange credentials, holdings, balances, valuations,
  transactions, orders (NFR22.1).
- **The reason is body-only**, never pre-filled, never echoed back into rendered copy beyond the
  grant's own reason heading, never in a URL, header, log, telemetry event or browser storage; it
  lives in component state and is cleared on success (NFR22.3).
- **Errors show status and correlation id only.** `ProblemDetails.detail` is never rendered and no
  audit-record id is shown. The one deliberate ambiguity is the shared `403` line in §3.
- **Escaping is React's default.** No `dangerouslySetInnerHTML` anywhere in this unit; a crafted
  display name, account label or portfolio label renders as text (NFR22.6).
- **The audit statement is copy, not content.** "This read is recorded in the audit log." is
  rendered; no audit record, id or retained personal data is ever displayed (NFR21.1).

## 6. Abuse and duplicate-request posture (NFR11.1, NFR2.7)

- **The create is one-shot.** One attempt per submit; Submit is disabled while in flight. A `409`
  offers **Check your current grant**, which reads S2 — it does not re-send S1. A `429` is not
  retried in code; the seconds come from `Retry-After`.
- **No polling, no focus refetch** (performance design §3), and no loop on any failure path.
- **No automatic re-read after a manual action except the one the person asked for** (**Check
  status**, **Refresh**), which are user-initiated and therefore also the retry controls.
- **Nothing is optimistic.** The banner and the panel render server answers; the local clock only
  shows *Expired* from an `expiresAt` the server already set.

## 7. Freshness and authorisation as controls (NFR24.1–NFR24.4)

- **Four statuses, no dead branch.** The console models `pending_approval`, `active`, `expired` and
  `revoked` — the statuses U3's domain produces. C5's `denied` has no producer; modelling it would
  add an unreachable state (spec §12 item 3), so it is not modelled until U3 adds it.
- **The server is authoritative on every read.** The local status is a clock-derived view of the
  last answer; an `active` grant whose `expiresAt` is null or unparseable is treated as expired
  (fail closed). A read after expiry or revoke is refused `403` by Identity, and the screen marks
  access ended rather than claiming access the server would refuse.
- **The 60-minute window is the server's.** It is computed at approval and displayed; the console
  never extends it and never computes a different one.
- **Role and status changes take effect at the next request.** No surface caches a role it would
  act on; the session's role decides only what is offered (NFR10.2), and the S2 gate follows it.
- **Every client-side gate is presentational** (FR15.9, NFR24.3): hiding the form or disabling a
  read changes no server decision, and a tampered client receives `403`.

## 8. Compliance — accessibility (NFR25.1–NFR25.5)

- **Structure.** Each U9 section is a labelled region with a visible heading; the accounts table
  carries a `<caption>` and `scope="col"` headers; the portfolio list is a `<ul>`; the filter is a
  labelled input; the banner's remaining time is expressed in text.
- **Linking errors.** Every input has a visible label and its error is tied with
  `aria-describedby` and `aria-invalid`; the reason counter is not a live region.
- **Announcements.** The request result, the status change and the metadata read completion are
  announced in a `role="status" aria-live="polite"` region without moving focus; an access-ended or
  failed state is a `role="alert"` carrying its text and correlation id.
- **Colour never alone.** `StatusBadge` renders label + tone + shape for the four statuses from
  `grantStatusPresentation` (`src/domain/grants.ts`): pending = warn/diamond, active =
  success/dot, expired = danger/square, revoked = danger/ring. The urgency of a nearly expired
  grant is text ("less than a minute left"), not colour.
- **Keyboard.** Every control is reachable and operable; the metadata filter and the request form
  keep their reading order so the metadata-only reminder precedes the projection; focus is visible
  on every control.
- **Mechanical gate.** Every rendered state in every U9 suite passes `jest-axe` (NFR25.5); a state
  without an axe assertion is a review-time defect.

## 9. Compliance — localisation (NFR26.1–NFR26.3)

- `expiresAt` and `lastSyncedAt` render through `Intl.DateTimeFormat` in the console's locale; the
  remaining-minutes text is a whole number and "less than a minute left" replaces `0`; no date is
  rendered as a raw timestamp or epoch value (NFR26.1).
- **NFR26.2 remains deferred and is recorded as deferred, not met.** The console has no i18n
  module; every existing section renders English literals and the SPA's i18n does not reach this
  package. U9 adds its copy the console's way and forks no second mechanism. The deferral is
  tracked as decision `ab46b27b` in this intent's decision log, naming a console-wide localisation
  unit as the owner (tech-stack T8); until it lands, NFR26 is not satisfied for the console.
- `Intl` degrades to its default rather than rendering an empty or malformed date, and every status
  is rendered as a word from the presentation mapping, never as a raw enum value (NFR26.3).

## 10. Threat → mechanism map

| Threat | Design mechanism |
|---|---|
| A support agent reading beyond the grant's target or window | S3 checks role, caller-held grant, active status and target accessibility server-side; the console's disabled state is presentation only (§3, §7) |
| An admin session tripping S2 and losing its own cache | `useMyGrants` is enabled only for `role === "support"`; a route-level `403` clears the support keys only, not the whole cache (§2, §3) |
| A stale or forged client claim opening the support surface | Every route re-checks the role from the forwarded JWT; the client gate is presentational (§7) |
| An over-sent projection field leaking holdings or identifiers | The console whitelist drops unknown fields and throws on a malformed projection (§5) |
| The reason leaking through a URL, header or log | Reason is body-only; `X-Support-Reason` is retired and its stale documentation corrected in the same change (§4) |
| Credential or token theft from browser storage | Memory-only session; no storage writes; full cache clear on 401/sign-out/expiry (§2) |
| Cross-site scripting through a display name or account label | React's escaped rendering; no HTML injection point (§5) |
| A duplicate request creating two grants | The server refuses a second live grant with `409`; the console sends one attempt and offers **Check your current grant** (§6) |
| Limit evasion across replicas | Stated honestly: the limiter is in-process per replica; S2 rides the shared per-caller-address budget (§3) |

## 11. Recorded dependencies (must be pinned before Code Generation)

1. **S2's backend is U3's, and U9 is blocked on it** (tech-stack T1): `list_by_support_agent(&UserId, u32)`
   filtering `support_agent_id` with `ORDER BY requested_at DESC, id`, `MY_GRANTS_LIMIT = 20`, a bare
   `Vec<PendingGrantResponse>` answer, the `/v1` registration, the gateway `Route` entry, and the
   `/v1/support-grants/mine` arm in Identity's `route_label`
   (`services/identity/http/src/middleware.rs:140` — without it S2 is counted as a revoke).
   **None of it exists yet**: `contract-summary.md` declares no `mine` path, the gateway `Route`
   table and `services/identity/` have no such entry or method, and U3's work is already landed. So
   until that delivery ships, the console's only non-admin read — and with it W4's reload recovery
   and the **Check status** control — has no server half and no frozen response shape. Code
   Generation for `useMyGrants` does not start before it; building against a route that returns
   `404` would produce a green suite over a dead surface.
2. **Contract entries, in the same delivery as item 1**: `contract-summary.md` C1/C2 and C5 gain the
   `mine` path and its bare-array `Grant` response (including `requestedBy`, which the console
   renders but never compares against a session identity — performance design §3). The Functional
   Design statements this record supersedes are listed in tech-stack §3 (S2 ownership and budget,
   the cache-clear file list, the non-admin approve state, `onGrantChanged`).
3. **Documentation edits in the same change**: `support-console/README.md` lines 29 and 33,
   `web/nginx.conf:7-9`, and the comment at `services/gateway/infrastructure/src/upstream.rs:88-90`
   (U9 owns the gateway comment edit; no behaviour or test changes).
4. **No support-facing user lookup exists** (spec §12 item 2): the form takes a pasted internal
   user id; resolving a ticket reference is a new route with its own authorisation and audit, not
   something this surface invents.
5. **C5's `denied` has no producer** (spec §12 item 3): if it is intended, U3 adds it and the
   console adds the branch then.
6. **NFR26's owner** is tracked as decision `ab46b27b`; no console-wide i18n unit exists in
   `unit-of-work.md` yet.
7. **NFR2.3 has no duration series today** (performance design §8): a route-labelled series is U3's
   optional deliverable; until then the figure is a design budget, not an instrumented one.