# Tech Stack Decisions — u9-support-console-ui

**Upstream:** `functional-spec.md` (§4 W1–W7, §8 S1–S6, §12 open items),
`frontend-components.md` (§2 types and props, §3 client, hooks and shell wiring, §6 testing
surface, §7 files), `requirements.md` (constraints), codekb `technology-stack.md`, project memory
mandates ("Stack (fixed — do not substitute)").

## 1. Mandated stack (confirmed, no substitution)

Every layer this unit touches is already fixed by the project's mandates; the table records the
confirmation and the version each choice pins to today (codekb `technology-stack.md`):

| Layer | Choice | Version in repo | Mandate source |
|---|---|---|---|
| Support surface | React + TypeScript, in the separate `support-console/` package | 18.3.1 / ^5.7.2 | project memory (React 18 + TS 5) |
| Build | Vite | ^6.0.7 | project memory |
| Server state | TanStack Query | ^5.62.11 | project memory; `createSupportQueryClient()` returns `retry: false` for queries and mutations |
| Auth | @supabase/supabase-js, session in memory | ^2.47.10 | project memory; BRD-012 NFR-SEC-10 |
| Styling | Tailwind CSS v4 mapped to the Parchment Ledger tokens | ^4.0.0 | project memory (no raw values) |
| Tests | Vitest 3 + Testing Library + MSW 2 + jest-axe | ^3.0.0 / ^16.1.0 / ^2.7.0 / ^10.0.0 | team testing posture; all already in the console |
| Lint / format / typecheck | ESLint 9 flat, Prettier 3, `tsc -b` | ^9.17.0 / ^3.4.2 / ^5.7.2 | project memory; `yarn lint`, `yarn format:check`, `yarn typecheck` |
| Package manager | Yarn 4.18.1 via corepack | pinned | project memory (npm/npx/pnpm forbidden) |
| Backend for S2 | Rust — Axum + tower + sqlx, in Identity and in the gateway's `Route` table | as shipped | project memory (fixed stack, no substitution) |

**No dependency is added by this unit**, in either package (T8). The console has no size-limit
budget (`support-console/package.json` defines `build`, `test`, `lint`, `typecheck`, `format:*`,
and no `size` script); this unit adds none and states the chunk intent instead (performance
NFR2, "Resource constraints"). The client deadline the performance NFRs rely on already exists:
`RequestOptions.deadlineMs` is shipped in `support-console/src/api/client.ts:165-190`, and U9
passes `15_000` on its calls.

## 2. Unit-level decisions

### T1 — The unit is presentation-only; the S2 route is U3's work, not U9's

- **Context.** W4 needs a read that returns the caller's own grants, and no such route exists.
  `unit-of-work.md` allocates U9 as `ui` ("Embedded in the support console build", "read-only
  presentation") and assigns route additions to service units. The design review found that
  putting Identity application/HTTP files and gateway tests inside a UI unit leaves the backend
  half unowned.
- **Decision.** The **owner of S2 is U3** (Support Access), which owns Identity's grant store, the
  `/v1/support-grants` routes and their tests. U3 builds:
  - the port method `SupportGrantRepository::list_by_support_agent(&self, agent: &UserId, limit: u32)`
    — one `SELECT … WHERE support_agent_id = $1 ORDER BY requested_at DESC, id LIMIT $2`, matching
    the existing `find_live_by_agent_and_user` filter and ordering on the shipped
    `support_grants` table (`services/identity/migrations/0001_init.up.sql:110`; the requester
    column is `support_agent_id`, there is no `requested_by` column). No new table, no new grant
    rule;
  - the limit constant `MY_GRANTS_LIMIT: u32 = 20` (the W4 bound);
  - the response: **a bare array**, `Vec<PendingGrantResponse>`, reusing the shipped DTO
    (`services/identity/http/src/dto.rs:757`) that already carries the full C5 grant shape —
    `id`, `userId`, `requestedBy`, `reason`, `status`, `requestedAt`, `approvedBy`, `approvedAt`,
    `expiresAt`. No envelope and no new row struct: the name `GrantRow` is already the sqlx row
    type in the Identity adapter and would collide;
  - the `/v1` registration and the gateway `Route` entry (`GET /v1/support-grants/mine`, support
    role, no per-route budget — NFR11.1), with the service-side role check and its tests;
  - the `/v1/support-grants/mine` arm in Identity's `route_label`
    (`services/identity/http/src/middleware.rs:140`): today every `/v1/support-grants/` path that
    is not `/approve` or `/metadata` is labelled `support.grants.revoke`, so S2 would be counted
    as a revoke. The new arm goes before the catch-all and is asserted in the route's tests.
- **Consequences.** U9 consumes a route U3 owns, and the boundary matches `unit-of-work.md`. The
  dependency is named in §3 and must land before U9's Code Generation. If the team prefers U9 to
  own the route instead, `unit-of-work.md` is amended explicitly in that change — the ownership is
  never left implicit.
- **Alternatives rejected.** (a) Keep the route inside U9 — rejected: it puts Identity and gateway
  production code in a `ui` unit with no owner for the port, the bound or the service tests;
  (b) have the console read each grant by id — rejected: no by-id read exists and the session does
  not know its grant ids after a reload; (c) reuse the admin pending list — rejected: it is
  admin-only and lists other agents' requests, not the caller's own.

### T2 — `useMyGrants` runs for support sessions only; the shell keeps no pending list

- **Context.** `Role::is_support()` is exactly `Support` (`services/identity/domain/src/claims.rs:72-74`),
  so a support-only S2 answers `403` to every admin session. The shell's existing
  `initialPendingGrants`/`pendingGrants` are dead state: the pending list lives in U10's
  `AdminApprovals` through `usePendingGrants` (`support-console/src/hooks/useAdminUsers.ts:53`).
  Approve/revoke answers are keyed `grantId` and do **not** carry `requestedBy`
  (`services/identity/http/src/dto.rs:714-734`), so an adopt rule cannot read it from them.
- **Decision.** `useMyGrants` is enabled only when `role === "support"`; an admin session never
  calls S2. The shell drops `initialPendingGrants`/`pendingGrants` entirely and adopts its grant
  from the **S2 re-read** (newest row whose `requestedBy` matches the signed-in user) or from the
  S1 create answer. `onGrantChanged` from the admin section updates nothing in the shell; the
  banner changes only when the session's own grant changes on an S2 re-read. `onCheckStatus` is
  supplied by the shell to `GrantRequestForm` (`onCheckStatus?: () => void`), which is the only
  consumer of the `409` **Check your current grant** control. The cache rule follows the shipped code rather than contradicting it: a `401`,
  sign-out, grant expiry or revoke keeps today's **full** `client.clear()` in
  `support-console/src/api/queryClient.ts` (that is NFR22.5's guarantee), while a **route-level
  `403` from S2 or S3** — the access-ended answer — clears only the support query keys through a
  small scoped helper. `queryClient.ts` and the `useClearSupportCacheOnErrors` hook therefore
  appear in this unit's file list as **changed**, not comment-only.
- **Consequences.** No guaranteed `403` for admins; a route-level `403` no longer wipes U10's
  `["admin", …]` cache, while sign-out and expiry still clear everything; one source of truth for
  pending grants (U10's AdminApprovals); adoption uses data the answer actually carries.
- **Alternatives rejected.** (a) Open S2 to any authenticated caller with an empty list for admins
  — rejected: it widens a route for a call the admin surface never needs; (b) keep the shell's
  pending list — rejected: it duplicates U10's list and has no consumer; (c) detect the session's
  grant from approve/revoke answers — rejected: those answers do not carry `requestedBy`.

### T3 — `pickSupportGrant` accepts `grantId` or `id`; one mapper for every grant answer

- **Context.** Create/approve/revoke answers are keyed `grantId`; the pending list is keyed `id`
  with `requestedBy`. The shipped `pickSupportGrant` requires `id`
  (`support-console/src/domain/metadata.ts:149`), so a create or approve answer throws today — a
  real defect (W2).
- **Decision.** `pickSupportGrant` reads `grantId ?? id` as the identifier and validates the four
  statuses it models. One mapper serves S1, S2 and the U10 answers.
- **Consequences.** The console reads the contract's own keys; no per-call mapper forks.
- **Alternatives rejected.** (a) Change the server's key to `id` — rejected: the contract and U3
  are shipped and the console is the consumer; (b) a second mapper for `grantId` — rejected: two
  shapes of the same entity drift.

### T4 — Retire the routes and header that do not exist; correct their stale documentation

- **Context.** The console calls `/support-grants/{id}` (no such route), `/support/accounts*` (no
  such route) and sends `X-Support-Reason` (forwarded by the gateway's upstream client,
  `services/gateway/infrastructure/src/upstream.rs:88-90`, but read by no service). W3 retires all
  three; W7 removes `getSupportGrant`/`useSupportGrant`.
- **Decision.** Remove `getSupportGrant`, `useSupportGrant`, `searchAccounts`,
  `fetchAccountMetadata`, `AccountSearch`, `AccountMetadataPanel`, and the header. The gateway's
  upstream comment and `support-console/README.md:33` are corrected in the same change, so no file
  claims a header the console no longer sends or a route Identity no longer serves. The metadata
  read is `GET /v1/support-grants/{grantId}/metadata` only.
- **Consequences.** The console has no call to a 404; the documentation matches the code.
- **Alternatives rejected.** (a) Leave the dead functions behind a flag — rejected: dead surface
  that still documents a nonexistent contract; (b) keep the header for "future use" — rejected: it
  is not read, and it invites a second reason channel.

### T5 — `GrantApprovalQueue` is mounted by the admin section only; the non-admin state is dropped

- **Context.** The queue is mounted only under `AdminUsersSection`, which the shell renders only
  for `role === "admin"` (`support-console/src/screens/SupportShell.tsx:225`), so the "Approve
  disabled when role is not admin" state and its test are unreachable.
- **Decision.** Drop the non-admin disabled state and its test; the component keeps its existing
  behaviour and lives in U10's `AdminApprovals` only. The support surface renders no approval
  control anywhere (W6).
- **Consequences.** No unreachable branch and no test of a state no user can reach.
- **Alternatives rejected.** (a) Keep the state "for safety" — rejected: an untestable branch;
  (b) mount the queue in the support section — rejected: AC3.2.2 and W6.

### T6 — `SupportMetadataPanel` is mounted only with an active grant; the shell owns the no-grant callout

- **Context.** The props made `grant` required and non-nullable, while the screen rules say the
  metadata section, with no grant, renders one callout and performs no read.
- **Decision.** The shell renders the no-grant callout itself; `SupportMetadataPanel` receives a
  non-null `grant` in local `active` state and is mounted only then. Its query is enabled only
  when the local clock says the window is open, so a mounted panel cannot read under a dead grant.
- **Consequences.** One component has one state; the no-grant path does no work.
- **Alternatives rejected.** (a) Make `grant` nullable and disable the query inside — rejected:
  it puts a second copy of the no-grant copy in the panel; (b) render the panel with a null grant
  and a guard — rejected: a null is a state the component never needs.

### T7 — Tests use the console's existing tooling; no new test dependency

- **Context.** The console already has Vitest 3, Testing Library, `user-event`, `jest-axe`, `jsdom`
  and MSW 2 as dev dependencies, and the existing suites mock the gateway with MSW.
- **Decision.** New suites follow the console's pattern (`frontend-components.md` §6): MSW handlers
  for S1/S2/S3, `jest-axe` on every rendered state, `user-event` for the request form's keyboard
  behaviour and the filter.
- **Consequences.** No dependency change to review; the tests exercise the real `client.ts` path,
  including the `deadlineMs` option.
- **Alternatives rejected.** Hand-rolled fetch stubs — rejected: they bypass the client module
  whose contract the suite exists to protect.

### T8 — Localisation: the console's existing strings, no i18n library

- **Context.** NFR26 asks for English and Vietnamese; the support console has no i18n module and
  every existing section renders English literals (`functional-spec.md` §6, §12 item 6).
- **Decision.** Add the section's strings the way the console already does, and format dates with
  `Intl.DateTimeFormat`. Add no i18n library and no second locale mechanism. **NFR26's EN/VI half
  is therefore recorded as deferred for this surface (security NFR26.2), not as met**, with a
  console-wide localisation unit as the named owner.
- **Consequences.** The console stays internally consistent; the deferral has one owner and one
  record. When that module lands, this section's strings move with it.
- **Alternatives rejected.** (a) A local `t()` helper for this section only — rejected: a second
  localisation mechanism with no locale source; (b) an i18n library added here — rejected: it
  would need to cover the console's existing strings to be coherent, which is a console-wide
  change this unit does not own. The deferral is tracked as decision `ab46b27b` in the decision
  log, so its owner is recorded rather than hypothetical.

## 3. Open dependencies on other units (not stack choices)

- **S2's backend (T1).** The `list_by_support_agent` port method, `MY_GRANTS_LIMIT = 20`, the
  `Vec<PendingGrantResponse>` answer, the `route_label` arm, the `/v1` registration and the gateway
  `Route` entry are U3's work. They must land before U9's Code Generation; U9 consumes the route
  and adds no Identity or gateway code.
- **Upstream artifacts this change amends (recorded so NFR Design and Code Generation read one
  story, not two).** The frozen Functional Design still says S2 is this unit's addition
  (`functional-spec.md` §8, §12 items 1 and 5) and still gives it a "shared per-token budget"
  (§8), and `frontend-components.md` §3.3 reads the session's grants for a support **or admin**
  session while §7 lists the Identity application/HTTP files, the gateway route and its tests as
  files this unit creates. This record supersedes those four statements: **S2 is U3's**, the
  session reads its own grants only as a **support-role** session (NFR10.4), its budget is the
  shared per-caller-address budget (NFR11.1), and U9 changes no Identity or gateway file except the
  gateway's stale `X-Support-Reason` comment (T4). The same amendment covers the statements the
  other decisions reverse: `frontend-components.md` §7's `src/api/queryClient.ts — comment only`
  becomes **a changed file**, with `src/api/queryClient.ts` (scoped route-level `403` clear) and
  the `useClearSupportCacheOnErrors` hook both in the file list (NFR10.4, T2); §2.2's "Approve is
  disabled when `role !== "admin"`", §6's test row for it and §7's "non-admin disabled state" row
  are **dropped** (T5); and §3.3's `onGrantChanged` rule that the admin section sets the banner
  is **superseded** — the banner changes only on the session's own S2 re-read (T2). `support-console/README.md`
  line 29's "any 401/403 clear the TanStack Query cache" is corrected in the same README edit as
  line 33, to state the full clear on `401`/sign-out/expiry and the scoped clear on a route-level
  `403`. The Functional Design is not rewritten; this list is the amendment of record.
- **Contract entries for S2.** `contract-summary.md` C1/C2 and C5 need the `mine` path and its
  bare-array `Grant` response; the C4 model and `docs/architecture/README.md` move together only
  if a container or relationship changes (this is a route addition).
- **Retired-header documentation (T4).** `support-console/README.md:33` is this unit's file and
  U9 corrects it. The comment at `services/gateway/infrastructure/src/upstream.rs:88-90` is **also
  edited by U9**, as a comment-only change in the same commit, because it documents the header this
  unit stops sending and leaving it would leave two files claiming a live header; it changes no
  gateway behaviour and no gateway test.
- **Support-facing user lookup (spec §12 item 2).** There is no route that resolves a support
  ticket's requester reference to a user id. The form takes a pasted internal user id; identifier
  resolution is a new route with its own authorisation and audit, not something this surface can
  invent.
- **C5's `denied` status (spec §12 item 3).** U3's domain produces four statuses. If `denied` is
  intended, U3 adds it and the console adds the branch then; U9 models no dead status.
- **NFR26's owner (T8).** A console-wide i18n module is not in `unit-of-work.md`; the deferral is
  recorded as decision `ab46b27b` in the decision log (not left ownerless), and the unit that adds
  the module adopts this section's strings.