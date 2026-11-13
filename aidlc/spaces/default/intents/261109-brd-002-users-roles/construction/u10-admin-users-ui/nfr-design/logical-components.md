# Logical Components — u10-admin-users-ui

**Upstream:** `functional-spec.md` (§2 shell and gating, §3 screens, §4 W1–W5, §8 routes),
`frontend-components.md` (§1 hierarchy, §3 client and hooks, §5 presentation),
`nfr-design/performance-design.md`, `nfr-design/security-design.md`, `contract-summary.md`
(C1/C2, C3, C5, C7).
**Unit kind:** ui — the console inventory lives inside `support-console/`; the unit's server work is
four route entries and one read method inside existing services. It adds no deployable, no schema
and no container.

## 1. Component inventory

### 1.1 Console (`support-console/src/`)

| Logical component | Kind | Responsibility | Depends on | Failure domain |
|---|---|---|---|---|
| `AdminUsersSection` (`src/screens/admin/AdminUsersSection.tsx`) | section (lazy, `React.lazy` like `IncidentConsole`) | Owns the panel layout (lookup, record, approvals), the selected record state, the unmount cache removal | hooks, `GrantApprovalQueue` | The section only — a chunk-load failure falls to the shell's existing `Suspense`/error behaviour, not another route |
| Shell wiring in `SupportShell.tsx` | routing | Adds `{ id: "admin-users", label: "Admin users", adminOnly: true }` to `SECTIONS`, renders the lazy section, and **widens the hard-coded `visibleSection` guard to read `adminOnly`** so a support session's `#admin-users` hash falls back to `grants` | section | Nav and section switch; other sections unchanged |
| `AdminUserLookup` | form component | Exact-email field, client-side presence/format checks, submit sets the query's email; renders the in-flight, `404`, `403` and error states | `useAdminLookup` | The lookup panel |
| `AdminUserRecord` | component | Read-only record: identity, `RoleBadge`, `StatusBadge`, timestamps; hosts the two change forms | `RoleBadge`, `StatusBadge`, forms | The record panel |
| `RoleBadge` (`src/components/RoleBadge.tsx`) | component (console-local, tech-stack T1) | Label + tone + shape for `owner`/`admin`/`support`, reusing `StatusPresentation` | `ui.ts` helpers | Presentation only |
| `RoleChangeForm` | form component | Role select + reason; self-account warning; submit → `useChangeRole`; renders transition + "audited" from the answer | `useChangeRole`, `signedInEmail` | The role panel |
| `StatusChangePanel` | form component | Action by current status, reason, confirm block with focus management, self-account warning on disable; submit → `useSetUserStatus` | `useSetUserStatus`, `signedInEmail` (security design §8) | The status panel |
| `GrantApprovalQueue` | reused component | The Approvals panel, unchanged; its list prop now comes from `usePendingGrants` and `onGrantChanged` reaches the shell | `usePendingGrants`, shell callbacks | Its existing failure surface |
| `useAdminLookup` | hook | One-shot query on submit; the explicit options in performance design §3 (`refetchOnWindowFocus: false`, `refetchInterval: false`, `staleTime: 0`, `retry: false`) | `client.ts`, query client | Lookup read |
| `usePendingGrants` | hook | Bounded pending-grant read; refetched after approve/revoke; same explicit options | `client.ts`, query client | Approvals read |
| `useChangeRole`, `useSetUserStatus` | hooks (mutations) | One attempt, 15 s deadline, render-then-re-read, `409`/`503` re-read before reporting | `client.ts` | One change; failure stays inline |
| `client.ts` additions | transport (`src/api/`) | `lookupAdminUser` (body-carrying POST, security design D-S1), `changeUserRole`, `disableUser`, `restoreUser`, `listPendingGrants`; `deadlineMs` + `DeadlineError`; `retryAfterSeconds` parsing | gateway | Shared module — additive; other callers unchanged |
| `types.ts` additions | types | `ConsoleRole`, `AdminUserStatus`, `AdminUserRecord`, the mutation bodies/results | — | Compile-time only |
| MSW handlers + suites | test infra | A1′/A2/A5′/D1/D2 handlers incl. delayed, 429 and 403 variants; jest-axe on every state | `msw` (dev-only) | Tests only; cannot reach the production bundle |

`getSupportGrant` stays. The functional spec (§3.1 note) called the by-id grant read unused, but it
is imported and called by `src/hooks/useSupportGrant.ts` and exercised by `src/api/client.test.ts`
and `src/hooks/useSupportGrant.test.tsx`; deleting the function alone breaks `tsc -b` (`yarn
build`) and both suites. The hook itself has no component consumer today, so the chain could be
deleted whole — function, hook and both tests — but U10 otherwise touches only additive client
changes, and removing a tested hook is cleanup outside this unit's blast radius. It is therefore
left in place unchanged; the functional spec's removal note should be corrected when that stage is
next touched (recorded in the stage diary as a deviation).

### 1.2 Server-side touches (existing services, no new component)

| Touch | Where | What changes |
|---|---|---|
| Gateway `Route` entries | `services/gateway/domain/src/route.rs` | `POST /v1/admin/users/lookup` (60/min, admin), `POST /v1/admin/users/{userId}/disable` and `…/restore` (20/min, admin), `GET /v1/support-grants` (60/min, admin); the existing `GET /v1/admin/users` entry is replaced by the POST |
| Admin lookup widening | Identity `application`/`http` | `AdminUserView` (application) and `AdminUserResponse` (http) are **new** admin-only types carrying `role` (functional spec §3.2); the shared `UserRecordView` and `UserResponse` keep their shape; the use case resolves the subject and calls `RoleReader::read_role`; provider failure → `503` |
| Role read | Identity `application` ports + `infrastructure` | `RoleReader::read_role` beside `RoleWriter`, implemented by the same adapter/client |
| `/v1` registrations | Identity `http/src/routes.rs` | The existing `disable_account`/`restore_account` handlers registered on the JWT-authenticated router (they already re-check admin from the forwarded JWT) |
| Grant list | Identity `application` + `http` | `SupportGrantRepository::list_by_status(status, limit)` and its handler for `GET /v1/support-grants` |

## 2. Failure domains and blast radius

- **Lookup fails (network, timeout, 5xx, 403)** → the inline error with its retry; the record view
  stays absent and the approvals panel keeps working (separate query key). Blast radius: the lookup
  panel.
- **Provider role read fails** → `503`, rendered as "cannot be looked up right now"; no record is
  shown with a guessed role. Blast radius: the lookup.
- **A change fails ambiguously (`409`, `503`)** → re-read, report the server's state plus the
  correlation id; nothing is re-sent. Blast radius: the one form.
- **The section chunk fails to load** → the shell's existing `Suspense` fallback covers the first
  load; the console has no route-level boundary to add one to (hash sections, not routes), so the
  blast radius is the section body inside the shell — the header, nav and banner remain.
- **A tampered client shows the section** → every call still answers `403` from the gateway and
  Identity; the worst case is a presentational bug (NFR24.3).
- **The approvals read fails** → its own inline error and retry, never an empty list — the panel
  cannot be mistaken for "nothing pending".

## 3. Isolation strategy

- The feature lives in one folder (`src/screens/admin/` for the section and forms, one new
  `src/components/RoleBadge.tsx`, hooks beside the existing `src/hooks/`); the only files touched
  outside it are `src/api/client.ts` (additive), `src/api/types.ts` (additive), `src/screens/
  SupportShell.tsx` (section entry + guard widening), and the new test files.
- The console's shared query client is **not modified**: every U10 behaviour is a per-query option
  or a per-section cache operation, so no existing section's caching, retry or refetch behaviour
  changes.
- The shared `request()` gains one optional field; calls without it are byte-for-byte the same
  request as today.
- No new context providers, no shared state with other sections, no storage API, no telemetry call
  (the absence is part of the design — security design §2, §5).

## 4. Shared resources

| Resource | Shared with | Isolation |
|---|---|---|
| `src/api/client.ts` + `request()` | Every console section | Additive functions and one optional `deadlineMs`; existing call sites untouched |
| Query client and cache | Every console section | U10 keys namespaced under `["admin"]`; options set per hook; the prefix is removed on unmount |
| `SupportShell` nav and section switch | Every section | One `SECTIONS` entry and a guard generalisation from `incidents`-specific to `adminOnly`; existing sections render identically |
| `GrantApprovalQueue` | The `grants` section | Reused as-is with a different list source and the shell's `onGrantChanged`; no component change |
| Session (`src/auth/`) | Shell | Read-only reuse (token, role, email); the section adds no token handling |
| Gateway `Route` table | Every route | Four new entries and one method change; deny-by-default means an undeclared path is `404` |
| `SupportGrantRepository` | U3's grants feature | One read method; no write, no rule, no table |
| Supabase Auth client (`SupabaseRoleWriter`) | U2's role change | One read added on the same adapter; same timeouts, same one-attempt policy |

## 5. Data-layer note (NFR23.1 refinement)

NFR23.1 in the requirements says this unit's routes read and write "through Identity's existing use
cases and repositories". That is true for four of the five paths, but **this unit does add one
repository read**: `SupportGrantRepository::list_by_status` (tech-stack T4), because no list method
exists today. It runs under Identity's own Postgres role and DSN, returns only grant rows U3 owns,
and adds no schema. The least-privilege position is unchanged; the requirements wording should read
"existing repositories plus one bounded list read" — recorded here and in the stage diary rather
than silently overriding the requirement.

## 6. Bridge to Infrastructure Design

Nothing in this unit changes infrastructure. The section ships inside the existing console build;
the five routes ride the existing gateway and Identity services; no replica count, secret, volume
or Swarm service changes. Two facts the next stage should carry:

- **The console's static edge is the `web` service, but the production `/v1` edge is not defined**
  (`web/nginx.conf` serves the console at `/support/` and `/support/assets/`; the `web` service in
  `infra/swarm-stack.yml` publishes port 80; that file proxies no `/v1` path).
  D-S1's body-carrying lookup is what keeps this unit independent of whichever edge terminates
  `/v1`; if that edge is later specified, its log format is that stage's record to make, not this
  one's.
- **The grant-table status index** is a required dependency on U3's schema (security design §12
  item 4); this unit's list read is bounded to 100 rows regardless, so a missing index is a
  performance risk in U3's release, not a correctness one here.