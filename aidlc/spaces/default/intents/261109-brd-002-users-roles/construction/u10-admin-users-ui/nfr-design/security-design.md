# Security Design — u10-admin-users-ui

**Upstream:** `nfr-requirements/security-requirements.md` (NFR9.1–9.2, NFR10.1–10.3, NFR11.1–11.3,
NFR22.1–22.6, NFR21.1, NFR23.1, NFR24.1–24.4, NFR25.1–25.5, NFR26.1–26.3),
`nfr-requirements/tech-stack-decisions.md` (T1–T7, §3 dependencies), `functional-spec.md`
(§2.1 gating, §3, §4 W1–W5, §7 posture, §8 routes, §10 errors), `frontend-components.md` (§2 types,
§3 client and hooks, §5 accessibility), `contract-summary.md` (C1/C2, C3, C5, C7, C9, RFC 7807).
**Unit kind:** ui — enforcement of every admin decision is server-side (gateway `requires_admin`,
Identity's JWT re-check). This design makes the console **unable to undermine** those controls and
**unable to leak** what they protect.

## 1. Trust model and origins (NFR9.1, NFR9.2)

```
Browser (untrusted)                      Edge / backend (trusted)
┌───────────────────────────┐   TLS    ┌─────────┐         ┌──────────────────────────┐
│ support console           │ ───────▶ │ Gateway │ ──────▶ │ Identity                 │
│  admin-users section      │  JWT in  │ JWT val.│  route  │ admin lookup (role read) │
│  - renders server answers │  memory  │ admin   │  table  │ role / disable / restore │
│  - no decision, no loop   │  only    │ check,  │         │ grant list (U3's store)  │
└───────────────────────────┘  rate    │ limits  │         └──────────────────────────┘
     Google Fonts (pre-existing load-time origin, no data)
```

- **The console's browser-external origins today** are: the gateway (`/v1`, same origin via the
  edge), Supabase Auth (sign-in), and **Google Fonts** — `support-console/index.html` preconnects
  `fonts.gstatic.com` and loads `fonts.googleapis.com`, and the edge CSP allowlists both
  (`deployment/localhost/nginx/default.conf`, console locations). This unit **adds no origin**:
  no CDN data, no analytics, no telemetry, no other font. Google Fonts is a pre-existing
  console-wide choice; a reader must not be told otherwise. Replacing it with self-hosted fonts is
  a console-wide change (and a CSP change at the edge), not something this unit claims or makes.
- **Gateways only for data.** Every U10 call is a `/v1` gateway path through `src/api/client.ts`;
  no component builds a URL or calls a service.
- **URL discipline.** No reason, no address and no record field is put in a URL (§4 resolves the
  last place the functional design still did that). Reasons travel in the JSON body; the looked-up
  address travels in a body after §4.
- **The internal user id stays in the mutation paths** (`/v1/admin/users/{userId}/role|disable|restore`).
  That is deliberate and recorded: C7's paths are established, the id is an opaque identifier the
  audit trail already records with the change, and moving it into the body would not protect
  anything the address and reasons do not already need protected. The residual is stated rather
  than hidden: the edge log of those three paths contains a user UUID and no other record field.

## 2. Session, token and cache hygiene (NFR10.1, NFR22.5)

- The section reuses the console's in-memory session; it adds no token handling and writes nothing
  to `localStorage`, `sessionStorage`, IndexedDB or cookies — not the token, not the looked-up
  address, not the record, not a reason draft. The existing suite's no-storage assertion covers
  the module; the new suites add no storage call to mock.
- The console **never decodes or validates the JWT for a decision** (NFR10.2). It reads the
  session's `app_metadata.role` only to decide whether to offer the section (presentational), and
  every route it calls is authorised again from the forwarded token.
- The record lives in the query cache only. The section's unmount effect removes the `["admin"]`
  prefix (performance design §3), and the console's existing `clearSupportCache` runs on 401/403,
  sign-out, grant expiry and revocation (`SupportShell.tsx`, `useClearSupportCacheOnErrors.ts`).
  A support session that loses its grant therefore cannot keep reading the last record from cache.
- **The 401/403 wiring is per query, exactly as the console does it today.** `clearSupportCache` is
  invoked through `useClearSupportCacheOnErrors(error)` inside the owning component
  (`AccountSearch`, `AccountMetadataPanel`, `TicketQueue` all do this). `useAdminLookup` and
  `usePendingGrants` call it with their query error, and each change form calls it with its
  mutation's error, so a `401`/`403` on any U10 call clears `["admin"]` at once instead of waiting
  for the section to unmount. The reused `GrantApprovalQueue` owns no query in this layout — its
  list comes from `usePendingGrants` — so it needs no new wiring and its behaviour is unchanged.

## 3. Route enforcement (NFR10.3, NFR11.1–11.3, NFR24.3)

| Path | Gateway declaration | Identity |
|---|---|---|
| `POST /v1/admin/users/lookup` (§4 — converted from A1′'s GET) | `requires_admin: true`, `max_requests_per_minute: 60` | admin re-check from the forwarded JWT; the lookup stays audited (`identity.admin.user_lookup`, allowed and refused) |
| `PUT /v1/admin/users/{userId}/role` | `requires_admin: true`, `max_requests_per_minute: 20` | existing `change_role_for_gateway` (U2) |
| `POST /v1/admin/users/{userId}/disable` | `requires_admin: true`, `max_requests_per_minute: 20` | existing `disable_account` handler registered on the `/v1` router (U5's use case) |
| `POST /v1/admin/users/{userId}/restore` | `requires_admin: true`, `max_requests_per_minute: 20` | existing `restore_account` handler, same registration |
| `GET /v1/support-grants?status=pending_approval` | `requires_admin: true`, `max_requests_per_minute: 60` | new list handler over `SupportGrantRepository::list_by_status` (U3's store; no grant rule added) |

- **Deny by default.** Every path is declared in the gateway's `Route` table; an undeclared path is
  `404` before a service is reached. No U10 path is service-token-only, and the browser never
  calls Identity directly.
- **Per token, per replica.** The limiter keys on the token (`check_route(route, token, limit)`),
  so one admin's runaway client cannot spend another's budget, and the per-IP budget still applies
  underneath. The limiter is **in-process per replica**, so the effective ceiling is per replica —
  stated as the honest bound, not as a global one (NFR11.3).
- **20/min on the three mutations, 60/min on the two reads** — the figures come from the gateway's
  own existing declarations (20/min creation trigger, 60/min metadata read), not from invention.
  The 5/min deletion budget stays reserved for the irreversible erasure path.
- **A `429` is its own state** with `Retry-After`, never retried in code (performance design §6).
- **Presentational gating** (NFR24.3): a tampered client that opens `#admin-users` for a `support`
  session gets `403` from its first call, rendered as the not-an-administrator line with the
  correlation id — never an empty list and never a silent failure.

## 4. Design decision D-S1 — the lookup carries the address in its body

**Context.** A1′ is `GET /v1/admin/users?email=<address>`. Two edges are involved, and neither is
configured to keep the address out of a log. The console's **static** edge is the `web` service:
`web/nginx.conf` serves the console at `/support/` and `/support/assets/`, `infra/swarm-stack.yml`'s
`web` service publishes port 80, those locations set no `access_log off`, and nginx's combined
format logs `$request`. But that file proxies no `/v1` path — its console CSP permits same-origin
`connect` — so the lookup request does not pass through it. The edge that **does** handle `/v1` in
production is not defined in this repository: the local stack's
`deployment/localhost/nginx/default.conf` both serves `/support/` and proxies `/v1/`, and it sets no
`log_format`/`access_log` override, so it logs the request URL by default — and nothing here says
what plays that role in production. A GET lookup therefore puts a customer's address into an access
log on an edge whose production configuration this unit cannot see, let alone change. This is the
same class of exposure the console's read reason was moved out of the URL to avoid
(`X-Support-Reason`, `client.ts`).

**Decision.** A1′ becomes **`POST /v1/admin/users/lookup` with `{"email": "<address>"}`**, admin-only
at the edge and again from the forwarded JWT, 60/min, audit unchanged. The client's
`lookupAdminUser(token, email)` sends the body; no component ever builds the URL.

**Why this option and not the others.**
- It depends on nothing this unit does not control: no edge log configuration, no CDN behaviour,
  no undefined production stack. It is true wherever the console is deployed.
- It is a read in semantics only: no state changes, and the audit event the lookup already writes
  is unchanged. **No existing gateway route is a body-carrying read**, so this is the table's first;
  that is a deliberate trade, not an oversight — the method field is data, the route still declares
  `requires_admin: true` and its own 60/min budget, and the alternative is a customer address in a
  log this unit cannot reach. It is recorded here so the reviewer and the next reader see the
  departure and why it was taken.
- CSRF is not a consideration: the console holds its token in memory and sends it as
  `Authorization`, never a cookie, so a cross-site form cannot forge the call.

**Alternatives rejected.** (a) Keep the GET and add a log-redaction rule — rejected: the only file
here that both serves the console and proxies `/v1` is the local stack's
`deployment/localhost/nginx/default.conf`, a developer convenience that is not production; the
console's production static edge (`web/nginx.conf`) never sees the lookup, and the production `/v1`
edge is undefined in this repository. A rule written into either file would not govern the edge that
matters, so the option has no production owner and no place to land today; it can be added later, at
whichever edge is chosen, without touching this design. (b) Accept the exposure — rejected
as a silent default; the team may still choose it deliberately, and if it does, the decision is
recorded as a deviation from NFR22 rather than assumed. (c) Put the address in a header on the GET
— rejected: headers are not logged by nginx's default format, but a GET whose meaning lives in a
header is a worse contract than a POST with a body, and the address would then be invisible to the
route's own documentation and tests.

**Ripple, which must land in the same change as Code Generation.**
- `functional-spec.md` §3.1 and §8 (A1′) restate the call as `POST /v1/admin/users/lookup` with a
  body; §10's error table keeps its statuses unchanged.
- `frontend-components.md` §3.1's `lookupAdminUser` comment changes to the POST shape.
- `contract-summary.md` C1/C2 and C3 gain the entry; the C4 model and `docs/architecture/README.md`
  move with them (project rule: they change together).
- The gateway `Route` entry is `POST`, not `GET`.
- Until the entry lands, Code Generation must **not** ship the GET shape as a fallback.

## 5. Sensitive-data hygiene (NFR22.1, NFR22.2, NFR22.4, NFR22.6)

- **Reasons are body-only.** They are never pre-filled, never echoed back into rendered copy,
  never put in a URL, header, log or telemetry event. The forms state the transition and that it
  is audited — not the reason text. (The console's existing `X-Support-Reason` header belongs to
  the metadata read in the `search` section; U10 does not reuse or extend that pattern.)
- **Rendered fields are the record's display fields and nothing else**: display name, email and
  its verified flag, the internal id, role, status, timestamps. No provider subject, hash, token,
  wallet address, credential, holding, balance, subscription or entitlement (spec §3.2).
- **Errors show status and correlation id only.** `ProblemDetails.detail` is never rendered; no
  audit id is shown even though the lookup is audited. `422` field errors map by `errors[].field`
  to the local message text, never to server wording.
- **Escaping is React's default.** No `dangerouslySetInnerHTML` anywhere in the section; a crafted
  display name or address renders as text (NFR22.6).
- **Nothing is logged client-side.** The section adds no `console.*` of record contents and no
  telemetry call.

## 6. Abuse and duplicate-change posture (NFR11.1–11.3, NFR2.7)

- The three mutations are one-shot: the console issues exactly one attempt and relies on the
  server's audited intent/outcome pair (C4/C9) rather than retrying. This mirrors Identity's own
  provider writer (one attempt), and it is the correct trade: a duplicated role change or disable
  is worse than a failed one the admin must repeat.
- No polling, no focus refetch (performance design §3), no loop on any failure path.
- After an ambiguous answer (`409`, `503`) the section re-reads and reports; it does not re-submit.
- The `429` surface offers a manual retry only; the seconds come from `Retry-After`.

## 7. Freshness as a security control (NFR24.1, NFR24.2, NFR24.4)

- **The role shown is the provider's current `app_metadata.role`, read at lookup time.** Design:
  a `RoleReader::read_role(&self, subject)` on the authorisation port beside `RoleWriter`'s
  `set_role`, implemented by the same adapter and the same Supabase Auth client (connect 1 s, total
  3 s, one attempt). The lookup use case resolves the internal id to the provider subject with
  `reveal_auth_subject` (as `role_change.rs` already does) and calls `read_role`. Mapping follows
  `Role::from_claim` (`owner`/legacy `user`/absent → owner; `admin`, `support` as written).
- **Ordering against the audit trail.** `AdminLookup::by_email` writes its `Success` event after the
  record lookup and before returning. `read_role` therefore runs **before** that success event: a
  provider failure records `AuditOutcome::Failed` with a `provider-unavailable` context and returns
  `503`, so the trail never shows a successful lookup that answered `503`. The success event's
  fields are otherwise unchanged.
- **A provider failure is `503`, never a stored value.** The section renders "this user cannot be
  looked up right now" with a retry; the stored `role` column is not consulted, because it
  refreshes only at the target's next principal resolution and would be stale on the one screen
  used to judge access.
- **The server's answer wins everywhere.** No optimistic cache write, no client-side assumption of
  a new status; every success re-reads and every ambiguous failure re-reads before rendering.
- **The mutation target is the internal user id from the server's record**, never the console
  session's Supabase user id — the two id spaces differ (NFR24.4). No id translation in the browser.
- **Caching does not undermine freshness**: the record is removed on unmount and on 401/403/expiry
  (§2), and the re-read after a change is what makes the badge show what the provider now holds.

## 8. The self-account warning reaches both change forms

The threat table's lockout row promises a warning when the looked-up account is the signed-in one.
The functional components give `signedInEmail` only to `RoleChangeForm`; `StatusChangePanel`'s
props are `record`, `token`, `onChanged`. As designed there, a self-disable — the lockout case the
row names — would render no warning. **This design therefore passes `signedInEmail` to
`StatusChangePanel` as well**, and the panel renders the same quiet warning above the disable
action when `record.email` equals it:

- Role change (existing copy): "This is the account you are signed in with. Demoting it removes
  your own administrator access at your next request."
- Disable (new copy, same shape): "This is the account you are signed in with. Disabling it removes
  your own administrator access at your next request."
- Restore renders no warning: restoring your own account cannot lock you out.

It is a warning, not a block: no self-change rule exists on the server (spec §12 item 3), and the
console does not invent one. **This refines `frontend-components.md` §2.2's prop list**; it is
recorded as a required edit for the functional owner and in the stage diary, not made silently in
code.

## 9. Compliance — accessibility (NFR25.1–25.5)

- **Structure.** The section is a labelled region (`aria-labelledby` the visible `h2`), the section
  link carries `aria-current="page"` through the shell's existing nav, and every field has a
  visible label with its error tied by `aria-describedby` and `aria-invalid`.
- **The confirmation block** is a focus-managed group: focus moves to **Confirm** when it opens,
  Tab cycles between Confirm and Cancel while it is open, Escape cancels, and focus returns to the
  action that opened it on cancel or completion.
- **Announcements.** A completed change is announced in a `role="status" aria-live="polite"`
  region without moving focus; a failure is a `role="alert"` carrying its text and correlation id.
- **Colour never alone.** `RoleBadge` and `StatusBadge` render a label plus a shape (ring, dot,
  square, diamond) from the console's `StatusPresentation` vocabulary for all three roles and five
  statuses (`frontend-components.md` §5.1); Parchment Ledger tokens only, no raw values.
- **Mechanical gate.** Every rendered state in every U10 suite passes `jest-axe` (NFR25.5); a new
  state without an axe assertion is a review-time defect.

## 10. Compliance — localisation (NFR26.1–26.3)

- `createdAt` and `updatedAt` render through `Intl.DateTimeFormat` with the browser's locale (the
  console has no locale module to consult); statuses and roles render as words from the fixed
  mapping, never as raw enum values.
- **NFR26.2 remains deferred and is recorded as deferred, not met.** The console has no i18n
  module; every existing section renders English literals (`support-console/src/`), and this unit
  adds its copy the same way rather than forking a second localisation mechanism. The owner is a
  console-wide i18n module; when it lands, this section's strings move with it (tech-stack T7).
- `Intl` degrades to its default rather than rendering an empty or malformed date; no date is ever
  shown as a raw timestamp (NFR26.3).

## 11. Threat → mechanism map

| Threat (security requirements) | Design mechanism |
|---|---|
| Account enumeration by a compromised admin session | Exact-email lookup only, no list and no prefix search; every lookup audited (allowed and refused); 60/min per token; `404`/`403` distinguishability is no more than an admin could obtain anyway |
| The looked-up address reaching a log | Body-carrying POST lookup (D-S1); no address in any URL, header or referrer |
| A stale role misleading the admin | Provider read at lookup time; failed read is `503`, never a stored or cached value (§7) |
| Self-demotion or self-disable locking the last administrator out | Warning on both forms when the account is the signed-in one (§8); no server rule invented; change takes effect next request |
| Approvals showing only the session's own requests | The panel reads the pending-grant list; a failed read is an error with a retry, never an empty list |
| Audit content leaking through the UI | Reasons never rendered back, no audit id, `detail` never displayed (§5) |
| XSS through display name, address or reason | React escaping; no HTML injection point; reasons never rendered (§5) |
| Credential theft from browser storage | Memory-only session; no storage writes; cache cleared on 401/403/sign-out/expiry (§2) |
| Replayed or duplicated privileged change | One attempt per action; re-read on `409`/`503`; no automatic retry anywhere (§6) |
| A tampered client reaching an admin route | Deny-by-default route table with `requires_admin` on all five paths, re-checked in Identity (§3) |
| Limit evasion across replicas | Stated honestly: the limiter is per-token, per replica; the per-IP limit stays underneath (§3) |

## 12. Recorded dependencies (must be pinned before Code Generation)

1. **D-S1's contract ripple** — `POST /v1/admin/users/lookup`, plus the functional-spec,
   frontend-components, contract-summary, C4 and `docs/architecture/README.md` edits listed in §4.
2. **`RoleReader::read_role`** on the authorisation port and its adapter (Identity's
   `authorisation`/`infrastructure`, U2's territory); must land with the admin lookup's widening
   and map a provider failure to `503`.
3. **W2/W3/W5 contract entries** — the disable/restore `/v1` registrations and the pending-grant
   list, with their C1/C2, C5 and C7 entries and the C4 model moving together.
4. **A status index on the grant table for the list read** — **required**, owned by U3 (the schema
   owner); this unit adds the query method but not the migration.
5. **The provider's 3 s ceiling** stays Identity's; a change there changes the lookup budget.
6. **The self-change policy** (spec §12 item 3) — an authorisation-unit decision; the console warns
   and invents nothing.
7. **The production `/v1` edge** — the console's static edge is the `web` service (`web/nginx.conf`
   serves `/support/`; `infra/swarm-stack.yml` publishes port 80), but nothing in this repository
   defines what terminates `/v1` in production. D-S1 makes the design independent of it, and that is
   the reason no log-redaction option is chosen.