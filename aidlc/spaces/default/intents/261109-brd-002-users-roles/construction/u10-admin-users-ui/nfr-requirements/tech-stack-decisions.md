# Tech Stack Decisions — u10-admin-users-ui

**Upstream:** `functional-spec.md` (§4 W1–W5, §8 routes, §12 open items),
`frontend-components.md` (§3 client, hooks and files), `requirements.md` (constraints), codekb
`technology-stack.md`, project memory mandates ("Stack (fixed — do not substitute)").

## 1. Mandated stack (confirmed, no substitution)

Every layer this unit touches is already fixed by the project's mandates; the table records the
confirmation and the version each choice pins to today (codekb `technology-stack.md`):

| Layer | Choice | Version in repo | Mandate source |
|---|---|---|---|
| Admin surface | React + TypeScript, in the separate `support-console/` package | 18.3.1 / ^5.7.2 | project memory (React 18 + TS 5) |
| Build | Vite | ^6.0.7 | project memory |
| Server state | TanStack Query | ^5.62.11 | project memory; `createSupportQueryClient()` already returns `retry: false` for queries and mutations |
| Auth | @supabase/supabase-js, session in memory | ^2.47.10 | project memory; BRD-012 NFR-SEC-10 |
| Styling | Tailwind CSS v4 mapped to the Parchment Ledger tokens | ^4.0.0 | project memory (no raw values) |
| Tests | Vitest 3 + Testing Library + MSW 2 + jest-axe | ^3.0.0 / ^16.1.0 / ^2.7.0 / ^10.0.0 | team testing posture; all already in the console |
| Lint / format / typecheck | ESLint 9 flat, Prettier 3, `tsc -b` | ^9.17.0 / ^3.4.2 / ^5.7.2 | project memory; `yarn lint`, `yarn format:check`, `yarn typecheck` |
| Package manager | Yarn 4.18.1 via corepack | pinned | project memory (npm/npx/pnpm forbidden) |
| Backend for the three new routes | Rust — Axum + tower + sqlx, in Identity and in the gateway's `Route` table | as shipped | project memory (fixed stack, no substitution) |

**No dependency is added by this unit**, in either package (§2 T1/T6). The console has no
size-limit budget (`support-console/package.json` defines `build`, `test`, `lint`, `typecheck`,
`format:*`, and no `size` script); this unit adds none and states the chunk intent instead
(performance NFR2, "Resource constraints").

## 2. Unit-level decisions

### T1 — `RoleBadge` is a console-local component; no shared design-system package

- **Context.** The record must show a role with a label, a tone **and a shape** (NFR25.3), and the
  console already has `StatusBadge` and the `ui.ts` class helpers. Nothing in the console shares a
  component library with `web/`; they are two Yarn packages with their own lockfiles.
- **Decision.** Author `support-console/src/components/RoleBadge.tsx` in this unit, reusing
  `StatusBadge`'s `StatusPresentation` vocabulary; remove the unused `getSupportGrant` call from
  `src/api/client.ts` as the spec says.
- **Consequences.** The badge lives where its only consumer is; the console gains one small
  component and one new type (`ConsoleRole`); `web/` is untouched.
- **Alternatives rejected.** (a) A shared component package — rejected: no monorepo workspace
  exists, and creating one to share a twelve-line badge is a larger decision than this unit's
  story; (b) duplicating badge markup inside `AdminUserRecord` — rejected: the role is rendered in
  two places in the section and the shape/label rule must not drift; (c) putting `RoleBadge` in
  `web/` — rejected: U7's review already established these components belong to the unit that
  renders them, in this package (`functional-spec.md` §11, §12 item 7).

### T2 — The provider's role is read through the existing role port and client, not a second client

- **Context.** W1 answers the admin record with the provider's **current** `app_metadata.role`
  (NFR24.1). Identity already has the only Supabase Auth admin client in the repository
  (`SupabaseRoleWriter`, connect 1 s / total 3 s, one attempt), whose private `read_app_metadata`
  performs exactly this read before a write; and `RoleChangeService` already resolves the internal
  id to the provider subject through `UserRepository::reveal_auth_subject` before calling
  `set_role`.
- **Decision.** Add a read to the authorisation ports — `RoleReader::read_role(&self, subject)`
  beside the existing `RoleWriter::set_role` — implemented by the same adapter against the same
  client and the same metadata read (promote `read_app_metadata` to a shared helper or add
  `read_role` on it). The admin lookup use case resolves the target's subject with
  `reveal_auth_subject` (as `role_change.rs` already does) and calls `read_role`; a provider
  failure maps to the `503` the section renders, never to a stored value.
- **Consequences.** One provider client, one timeout policy, one mapping (`Role::from_claim`); the
  read costs one provider round trip per lookup, bounded at 3 s and already budgeted in
  performance NFR2.1. The port addition is inside Identity (`application` traits and the
  `infrastructure` adapter), which is U2's territory — carried as a dependency in §3.
- **Alternatives rejected.** (a) Read the stored `role` column — rejected: it refreshes only at the
  target's next principal resolution, so it is stale on exactly the screen used to judge access
  (this was the functional design review's critical finding); (b) have the console call the
  Supabase Admin API — rejected: it would put a service-role credential in the browser and break
  the console → gateway rule; (c) cache the provider's answer — rejected: NFR24.1's whole point is
  the value at lookup time.

### T3 — D1/D2 reuse the existing handlers; the work is a gateway route plus a `/v1` registration

- **Context.** `disable_account` and `restore_account` already exist in
  `services/identity/http/src/handlers.rs`. They already resolve the acting admin from the
  forwarded JWT (`caller_of`) and delegate to `AccountLifecycle::disable`/`restore`. What does
  **not** exist is a browser-reachable path: they are registered only on the internal router
  (`/internal/users/{userId}/disable|restore`) behind `require_internal_caller`.
- **Decision.** Register the same two handlers on the JWT-authenticated `/v1` router at
  `/v1/admin/users/{userId}/disable|restore`, and declare those two paths in the gateway's
  `Route` table with `requires_admin: true` and `max_requests_per_minute: 20` (NFR11.1). No new
  handler, no new use case, no new table.
- **Consequences.** The browser path and the internal path share one implementation and one
  authorisation decision; the two registrations differ only in their middleware. The gateway
  entries are data plus their own tests (rate limit, `403`, `404`).
- **Alternatives rejected.** (a) New handlers for the `/v1` shape — rejected: identical body and
  answer, two copies of the admin check to keep in step; (b) routing the gateway to the internal
  path with a service token — rejected: the gateway would then have to mint a service token and
  the acting user's re-check would move out of the handler, which is the property the forwarded-JWT
  rule exists to keep; (c) a separate admin service in front of Identity — rejected: a new
  service for two routes.

### T4 — The pending-grant list is one bounded read on the support-grant port

- **Context.** W5 needs every pending grant, not the session's own. `SupportGrantRepository`
  (`identity/application/src/ports.rs`) has `insert`, `find_by_id`, `update`, `find_live_by_agent_and_user`,
  `insert_with_event`, `approve_with_event`, `revoke_with_event` — and no list. The route is
  bounded to 100 rows, newest first (spec §4 W5).
- **Decision.** Add `list_by_status(&self, status: GrantStatus, limit: u32)` to
  `SupportGrantRepository` with a single `SELECT … WHERE status = $1 ORDER BY created_at DESC
  LIMIT $2`; the gateway declares `GET /v1/support-grants` (`requires_admin: true`, 60/min) and
  Identity registers the handler on the `/v1` router. No new table, no new state, no grant rule.
- **Consequences.** One query per panel mount and per approve/revoke refetch. If the grant table
  has no index for the status filter, U3 adds one in its own migration — this unit does not
  migrate a schema it does not own.
- **Alternatives rejected.** (a) Reuse `find_by_id` in a loop over ids the client already has —
  rejected: it only ever answers the session's own requests, which is the defect W5 exists to fix;
  (b) read the grant table from another service — rejected: cross-schema access is forbidden;
  (c) paginate — rejected for now: the panel's job is "what is pending", and 100 rows with the
  server bound is the honest shape (spec §4 W5).

### T5 — A client deadline on the U10 calls, added to the shared `request()`

- **Context.** `support-console/src/api/client.ts`'s `request()` passes the request to `fetch`
  with no timeout and no abort signal: a hung gateway leaves a U10 loading state indefinitely.
  The contract's browser-facing budget is connect 5 s + read 10 s.
- **Decision.** Add an optional `deadlineMs` (an `AbortController` whose timer is cleared when the
  response arrives) to `RequestOptions`; the five U10 client functions pass 15 s. On fire, the
  request rejects with the console's retryable error carrying a timeout message, which the section
  renders as the inline retryable error (performance NFR2.2). Default behaviour without the option
  is unchanged, so no other console section moves.
- **Consequences.** Every U10 call is bounded and its failure surface is a test case in the MSW
  suite (a delayed handler must produce the named-timeout state). Sections that pass no deadline
  keep their current behaviour — the change is additive, and widening it console-wide is a separate
  decision for the console's owner.
- **Alternatives rejected.** (a) No deadline, rely on the server — rejected: a hung gateway or a
  dropped connection strands the screen, which NFR2.2 forbids; (b) a global default for every
  console call — rejected: it changes every existing section's failure behaviour inside a unit that
  owns none of them; (c) a per-component `setTimeout` racing the fetch — rejected: it cannot
  cancel the request and leaks a late answer into the cache.

### T6 — Tests use the console's existing tooling; no new test dependency

- **Context.** The console already has Vitest 3, Testing Library, `user-event`, `jest-axe`, `jsdom`
  and MSW 2 as dev dependencies, and the existing suites mock the gateway with MSW.
- **Decision.** New suites follow the console's pattern (`frontend-components.md` §6): MSW handlers
  for A1′/A2/A3/A4/A5′/D1/D2, `jest-axe` on every rendered state, `user-event` for the confirmation
  block's keyboard behaviour.
- **Consequences.** No dependency change to review; the tests exercise the real `client.ts` path,
  including the deadline added in T5.
- **Alternatives rejected.** Hand-rolled fetch stubs — rejected: they would bypass the client
  module whose contract the suite exists to protect.

### T7 — Localisation: the console's existing strings, no i18n library

- **Context.** NFR26 asks for English and Vietnamese; the support console has no i18n module and
  every existing section renders English literals (`functional-spec.md` §12 item 5).
- **Decision.** Add the section's strings the way the console already does, and format dates with
  `Intl.DateTimeFormat`. Add no i18n library and no second locale mechanism. **NFR26's EN/VI half
  is therefore recorded as deferred for this surface (security NFR26.2), not as met.**
- **Consequences.** The console stays internally consistent; the deferral has one owner (a
  console-wide i18n module) and one record (NFR26.2 here, spec §12 item 5 there). When that module
  lands, this section's strings move with it — no rework of the data flow.
- **Alternatives rejected.** (a) A local `t()` helper for this section only — rejected: a second
  localisation mechanism for one section, with no locale source; (b) an i18n library added here —
  rejected: it would need to cover the console's existing strings to be coherent, which is a
  console-wide change this unit does not own.

## 3. Open dependencies on other units (not stack choices)

- **A1′'s address in a logged URL (NFR22.3).** The lookup is a `GET` with the email in the query
  string and the edge's default log format records the URL. Before Code Generation, one of: a
  body-carrying lookup (`POST /v1/admin/users/lookup`), an edge log-redaction policy for that
  route, or an explicit recorded acceptance of the exposure. The functional spec's A1′ needs the
  same edit in whichever form is chosen.
- **W1's role read on the port (T2).** The `RoleReader` trait and its adapter implementation are
  inside Identity's `authorisation` and `infrastructure` layers (U2's territory). It must land with
  the admin lookup's widening, and its failure must map to `503` rather than to a stored value.
- **W2/W3/W5 contract entries.** `POST /v1/admin/users/{userId}/disable`, `POST …/restore` and
  `GET /v1/support-grants` need their C1/C2, C5 and C7 entries, and `contract-summary.md`, the C4
  model and `docs/architecture/README.md` move together in the change that lands them.
- **The grant-table index (T4).** If the status filter has no supporting index, U3 adds it in its
  own migration; this unit does not touch a schema it does not own.
- **Provider availability is Identity's budget (T2).** The 3 s single-attempt ceiling is the
  existing adapter's; if the infrastructure design changes it, performance NFR2.1's composition
  changes with it.
- **The self-change policy (spec §12 item 3).** Whether an admin may demote or disable their own
  account is a decision for the authorisation unit; the console warns and does not invent a rule.