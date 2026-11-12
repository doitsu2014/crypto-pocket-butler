# Code Summary — u10-admin-users-ui

**Unit:** `u10-admin-users-ui` · **Stage:** Code Generation · **Testing Contract:**
`sha256:3f8863a9358b0b5e19a34ad2fc282a634f0d5323fb50ae1f7429a92a216766c3` (TDD, Standard, brownfield)
**Plan:** `code-generation-plan.md` (18 steps, all complete) · **Records:** this directory

## What this unit builds

The administrator's user-management surface in the support console, plus the Identity and gateway
surface it reads and writes:

- **Gateway** — a deny-by-default route table gains `POST /v1/admin/users/lookup` (A1′),
  `POST /v1/admin/users/{userId}/disable` (D1), `POST /v1/admin/users/{userId}/restore` (D2) and
  `GET /v1/support-grants` (W5), all `requires_admin`, with the documented budgets (60/min for the
  lookup and grant list, 20/min for role, disable and restore). The U1 `GET /v1/admin/users?email=`
  search is gone (D-S1).
- **Identity** — an admin lookup that takes the email in the body and answers the record plus the
  provider's current role (`RoleReader` port beside `RoleWriter`, `AdminUserView`), the pending-grant
  list read (`status=pending_approval` only), and the route labels the metrics rely on.
- **Console** — `AdminUsersSection` (lazy, admin-only) with `AdminUserLookup`, `AdminUserRecord`,
  `RoleChangeForm`, `StatusChangePanel`, `AdminApprovals` and the shared `RoleBadge`; the five client
  calls with a 15 s deadline, `Retry-After` parsing and `DeadlineError`; four hooks with
  `staleTime: 0`, `retry: false`, no window-focus or interval refetch; the admin query cache removed
  on unmount; `SupportShell`'s section guard widened to read `adminOnly`.

## Design decisions

- **D-S1 — the lookup is a body-carrying POST.** An email address must never reach a URL or an
  access log. `POST /v1/admin/users/lookup` with `{"email": "…"}` replaces the U1 `GET` search; the
  gateway answers the old path with its deny-by-default `404`. The ripple is landed in
  `functional-spec.md` §3.1/§8, `frontend-components.md` §2.2/§3.1/§7, `contract-summary.md`
  C1/C2, C3, C5 and C7, and `docs/architecture/{README.md,workspace.dsl}`.
- **Role read folded into the lookup.** `AdminUserView` carries `role`, read through the new
  `RoleReader` port at lookup time from `app_metadata.role` (`Role::provider_value()` →
  `owner | admin | support`). A provider failure is a `503` and no record — never a stale role.
- **The approvals panel reads the route, not the shell.** `GET /v1/support-grants?status=pending_approval`
  returns the C5 grant shape plus `requestedAt`/`approvedAt` the console's mapper needs; a fresh
  admin session sees requests it did not create. A failure is an inline error with a retry, never an
  empty queue.
- **Every change re-reads.** Mutations never seed the cache optimistically; both forms re-read the
  record after success, and the section removes `["admin"]` on unmount so a looked-up address is not
  left in the console cache.
- **Confirmation instead of a modal.** Disable/restore replace the action with an inline confirmation
  block: focus moves to Confirm, Escape cancels, Tab stays in the block, and focus returns to the
  action on cancel or completion. Disable uses `dangerButtonClass`; restore and role change never do.

## Red evidence

Command: `yarn --cwd support-console test src/screens/admin/ src/components/RoleBadge.test.tsx src/api/client.admin.test.tsx`

Before the components existed, all seven UI suites failed to collect:

```
FAIL  src/screens/admin/AdminUserLookup.test.tsx  — 7 failed (7), Tests  no tests
Error: Failed to resolve import "./AdminUserLookup" ... Does the file exist?
```

`client.admin.test.tsx` was already green from Steps 12–13 (the client and deadline work), so the
Red run isolated the missing UI. The Rust Red runs for Steps 2, 4, 6 and 10 were reported by the
build as compile-level failures (missing routes, ports, DTO fields and route labels) and were not
persisted to disk: the approved plan's steps carry no checkboxes and the stage diary does not hold
them, so the build report is the only ordering evidence and is recorded as such. Step 8 has no
independent Red at all — see Deviation 2.

## Green evidence

- `yarn --cwd support-console test` (whole package): **37 files, 241 tests, all passing**.
  - Step 14 target suites: 7 files, 48 tests (`RoleBadge` 5, `AdminUserLookup` 9,
    `AdminUserRecord` 9, `RoleChangeForm` 7, `StatusChangePanel` 7, `AdminApprovals` 6,
    `AdminUsersSection` 5) plus 2 new `SupportShell` cases (11 total).
  - Regression suites named by the unit instructions: `client.test.ts` 12,
    `GrantApprovalQueue.test.tsx` 8, `useClearSupportCacheOnErrors.test.tsx` 4 — all green.
- `cargo test --workspace`: **1632 passed, 140 ignored, exit 0** (225 suites).
- Unit-scoped Rust suites: `cargo test -p gateway-domain -p gateway-http -p identity-application
  -p identity-http -p identity-infrastructure` green; the new identity HTTP cases cover the POST
  lookup (role returned, 404 miss, 503 provider failure, body validation, admin-only), the retired
  U1 path, the pending-grant list filter and the route labels, and the disable/restore admin paths
  with their 403/404/409/422 answers.

## Quality gates

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | pass (after `cargo fmt --all`) |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass — "No issues found" |
| `bash scripts/check-boundaries.sh` | **FAIL (pre-existing)** — `reporting-domain must not import framework crates: ['serde', 'serde_json']`; `services/reporting/` is untouched by this branch's working tree |
| `cargo test --workspace` | pass — 1632 passed, 140 ignored |
| `yarn --cwd support-console test` | pass — 241 tests |
| `yarn --cwd support-console lint` | pass |
| `yarn --cwd support-console build` | pass — lazy `AdminUsersSection-*.js` chunk emitted (15.11 kB) |
| `yarn --cwd support-console format:check` | **FAIL (pre-existing)** — 27 files, none of them left unformatted by this unit (every remaining diff is in code this unit did not write) |

Coverage: the 80 % per-crate Rust target is judged by Build and Test with `cargo llvm-cov`; this
unit run did not measure it and makes no claim. The ignored adapter/migration suites are the
Build and Test gate's disposable-Postgres run; `TEST_DATABASE_URL` was not set here.

## Sensors

- `traceability` fired for `u10-admin-users-ui` at `code-generation` — **passed**
  (`fire_id 47677aeb`, after the conductor's row corrections described in Deviation 9), 40 upstream
  ids with existing workspace-relative targets: 39 `OK` and `NFR26.2` `Deferred`.

## Tests written but not executed

- The `#[ignore]`d adapter and migration suites (`cargo test -p identity-infrastructure
  -- --ignored`, plus the other `#[ignore]` database cases in the workspace) were not run:
  `TEST_DATABASE_URL` is unset in this environment and the plan's disposable-Postgres provisioning
  is a Build-and-Test gate, not a unit-run step. This unit adds no migration.
- No structurizr validation was run: the CLI is not installed here, and the plan did not name it.

## Deviations from the plan

1. **Plan-checkbox edit via a shell write.** Earlier in the run, two plan checkboxes were flipped
   with a `python3 -` write instead of the file-edit tool. Corrected from that point on: every later
   checkbox was ticked with the edit tool, and nothing else in the plan was touched.
2. **Steps 8 and 9 ordering.** The pending-grant list read was implemented alongside its Red test in
   one pass, so Step 8 has no recorded independent failing output.
3. **Role-reader adapter tests are not `#[ignore]`d.** `FakeRoleReader`/`SupabaseRoleWriter` are
   stub-HTTP based, not database-backed, so their tests run by default; the plan's blanket
   `#[ignore]` expectation did not fit this adapter.
4. **`getSupportGrant` kept.** The functional design says it is removed; the plan's correction kept
   it, and it remains unused by U10. The docs were updated to say so rather than claim a removal.
5. **Test-environment `AbortController` shim.** Node's fetch rejects jsdom's `AbortSignal`
   ("Expected signal to be an AbortSignal"). `src/test/setup.ts` maps the global to
   `node:util`'s `transferableAbortController`; production code keeps the real controller and
   signal, and the deadline behaviour is asserted in `client.admin.test.tsx`.
6. **`AdminUserLookup` prop names.** Implemented as `{ onSubmit, loading, error?, notFound?, onRetry? }`
   rather than the design sketch's `onFound`/`onNotFound`; the section keeps ownership of the query,
   and `frontend-components.md` §2.2 was updated to match.
7. **`cargo fmt --all` reformatted files this unit did not author.** The branch's working tree
   carried unformatted prior-unit files (`identity/tests/application.rs`,
   `support_application.rs`, `authorisation_*.rs`); satisfying the fmt gate rewrote those
   formatting-only differences. No behaviour changed.
8. **No commit, no push, no completion gate** — per the dispatch instructions.
9. **Traceability targets corrected by the conductor.** The build's first `traceability.json`
   mapped the inherited `NFR2.x`, `NFR9–11` and `NFR23–24` rows to arbitrary console files and
   marked `NFR26.2` `OK`. `NFR26.2` is the console's deferred English/Vietnamese half
   (`security-requirements.md`; `nfr-requirements/traceability.json` marks the parent `Deferred`),
   so it is `Deferred` here too, with its reason as the target. The other rows now point at the file
   that serves each requirement — gateway `route.rs`/`rate_limit.rs`, Identity
   `user_registry.rs`/`support.rs`, `client.ts`, `useAdminUsers.ts`, the section and the form
   components. The sensor passed after the correction.

## Known gaps

- `check-boundaries.sh` fails on a pre-existing `reporting-domain` serde import; not fixed here
  because it is outside this unit's scope and would change another service's crate.
- `format:check` fails on 27 pre-existing files across the console package; reformatting the package
  would be an unrelated change, so it is reported rather than swept into this unit.
- The Structurizr model was edited but not validated (CLI absent).
- `TEST_DATABASE_URL` was unset, so the ignored database suites did not run in this unit run.