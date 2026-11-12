# Code Summary — u9-support-console-ui

**Plan:** `code-generation-plan.md` (approved, 24 steps / 65 tasks, every checkbox ticked)
**Tests:** `unit-test-instructions.md`
**Upstream:** `functional-design/functional-spec.md` (S1–S3, W1–W7), `nfr-design/`, `infrastructure-design/`.

## What was built

The console's support-grant surface, rebuilt on the routes that actually exist, plus the one
server route it depends on and that no open change delivered.

- **Request (W1):** `GrantRequestForm` posts `{userId, reason}` — no duration, no approver — and
  renders `201`, `409` (offers Check your current grant), `404`, `403`, `422`, `429` with its
  `Retry-After`.
- **Session state (W4):** `SupportShell` reads `GET /v1/support-grants/mine`, adopts the newest
  grant unconditionally, holds one clock for the banner and the metadata panel, renames the
  `search` section to `metadata` (a `#search` hash still lands), and gates the section by role.
- **Banner (W5):** four statuses with clock-derived expiry; an `active` grant with no parseable
  expiry reads as expired (fail closed).
- **Metadata (W3):** `SupportMetadataPanel` calls `GET /v1/support-grants/{grantId}/metadata`,
  projects through the whitelist, filters already-returned rows with no request, and renders
  `403` as access-ended, `401`, and `503` with Refresh.
- **Approvals (W6):** `GrantApprovalQueue` disables approve for a non-admin viewer and never
  adopts another agent's grant into the admin's own banner.
- **Retired (W7):** the by-id grant read, the account search and the account metadata panel are
  gone — components, hooks, client calls, types and tests.
- **Server (S2):** the gateway route, Identity's `my_grants` use case and `list_by_support_agent`
  repository read (`MY_GRANTS_LIMIT = 20`), the HTTP handler answering a bare array, the
  `support.grants.mine` `route_label` arm, and the `0008_support_mine_index` migration pair.

## Files

### Created (10)

| Path | What |
|---|---|
| `support-console/src/hooks/useMyGrants.ts` | The own-grant read hook |
| `support-console/src/hooks/useMyGrants.test.tsx` | Its suite |
| `support-console/src/hooks/useSupportMetadata.ts` | The metadata read hook |
| `support-console/src/hooks/useSupportMetadata.test.tsx` | Its suite |
| `support-console/src/components/SupportMetadataPanel.tsx` | The panel that replaces the dead chain |
| `support-console/src/components/SupportMetadataPanel.test.tsx` | Its suite |
| `support-console/src/test/supportHandlers.ts` | MSW handlers for the three §8 routes |
| `support-console/src/test/supportHandlers.test.ts` | Their suite |
| `services/identity/migrations/0008_support_mine_index.up.sql` | `support_grants_mine_idx (support_agent_id, requested_at DESC)` |
| `services/identity/migrations/0008_support_mine_index.down.sql` | `DROP INDEX` |

### Deleted (6)

`support-console/src/components/AccountSearch.tsx` and `.test.tsx`,
`support-console/src/components/AccountMetadataPanel.tsx` and `.test.tsx`,
`support-console/src/hooks/useSupportGrant.ts` and `.test.tsx`.

### Modified (36)

**Console (27):** `src/api/{client.ts, client.test.ts, queryClient.ts, queryClient.test.ts,
types.ts}`, `src/components/{GrantRequestForm,GrantBanner,GrantApprovalQueue}.{tsx,test.tsx}`,
`src/domain/{grants,metadata,presentation}.ts` with the two test files, `src/hooks/
useClearSupportCacheOnErrors.{ts,test.tsx}`, `src/screens/SupportShell.{tsx,test.tsx}`,
`src/test/fixtures.tsx`, `support-console/README.md`.

**Gateway (4):** `domain/src/route.rs` (one `Route` entry), `tests/domain.rs`, `tests/http.rs`,
`infrastructure/src/upstream.rs` (a comment).

**Identity (7):** `application/src/ports.rs`, `application/src/support.rs`,
`infrastructure/src/repositories/support.rs`, `http/src/{handlers,dto,routes,middleware}.rs`.

**Tests outside the unit's own files (3):** `services/identity/tests/{support_application,
support_adapters,support_http}.rs` (extended), `tests/common/fakes.rs` (the read's fake).

**Other (2):** `web/nginx.conf` (a log-format comment), plus the record-side
`inception/contract-design/contract-summary.md` (C1/C2 and C5 entries).

## Key implementation decisions

1. **S2 is built here.** `GET /v1/support-grants/mine` had no gateway `Route`, no handler, no
   `route_label` arm and no delivering change; the console's own-grant recovery is dead without
   it. The plan recorded this as an explicit, separable scope block and the person approved it.
2. **The `mine` route carries no per-route budget.** It rides the shared per-caller-address
   budget rather than inventing a limit; the plan and the infrastructure design both say so.
3. **`route_label` gets a dedicated arm.** Without it `/v1/support-grants/mine` would be counted
   as a revoke (`middleware.rs:140`). The arm is placed before the generic arm, and the HTTP test
   proves `/metrics` reports `route="support.grants.mine"` with no revoke series from that path.
4. **The shell ignores `requestedBy`.** `frontend-components.md` §3.3 is stale there: `requestedBy`
   is Identity's internal id, not the Supabase auth id, and the list is already caller-scoped. The
   newest own grant is adopted unconditionally, and a grant adopted from `201` is never replaced
   by a stale list.
5. **`useClearSupportCacheOnErrors` keeps its old behaviour** (clear everything on `401` and
   `403`); the narrower `useClearSupportQueriesOnRouteErrors` is a second hook, so U10's admin
   callers are untouched.
6. **MSW handlers are an opt-in module**, imported only by tests, not registered as `setupServer`
   defaults: registering them would answer the three routes in every suite and defeat the existing
   `onUnhandledRequest: "error"` policy. Nothing under `src/test/` reaches the bundle.
7. **`MY_GRANTS_LIMIT` is `usize` = 20**, matching the existing `pending_grants` bound and
   converted for SQL. The plan's port note said `u32`; the repository's own convention won.
8. **`DELETE /v1/support-grants/mine` is not a route.** It stays matched by the parameterised
   revoke pattern (`grantId="mine"`) and is refused by Identity as an invalid grant id; no
   dedicated route is declared for it, and the gateway test records that rather than special-casing.
9. **Migration ordering is now part of every older down-migration test.** `migrations::revert`
   reverts the newest applied migration, so `0008` being newest required one extra revert in
   `support_adapters.rs`, `account_lifecycle_adapters.rs` and `repositories.rs`. No assertion was
   weakened.

## Test coverage

- **Console: 38 suites, 272 tests, all passing** (up from 37 / 241 before this unit). Every suite
  named in `unit-test-instructions.md` exists and covers its happy path plus the error and edge
  cases the plan lists; jest-axe runs on every rendered state of the form, banner and panel.
- **Rust: `gateway` 19 domain + 27 http; `identity` 27 `support_application`, 27 `support_http`,
  25 `support_adapters`** (the adapter suite includes the `0008` up / down / re-up case, the
  20-row bound and the newest-first order against the real table).
- **Coverage:** no Rust crate was measured separately for this unit; `cargo llvm-cov` was not run
  in the build (the plan defers it to Build and Test). The 80% per-crate target is unchanged and
  unverified here — recorded as an open gap, not as met.
- **TDD evidence:** every step's Red run and its failing output are recorded for Steps 2, 4, 6, 8,
  10, 13, 15, 20 and 22. For **Steps 16–19 the exact failing text is not available** — the
  developer's session context was compacted between the Red run and the report, and the report
  says so rather than reconstructing it. What survives is the Green counts and the fact that the
  tests compile only against the symbols the step introduced. This is an evidence gap in the TDD
  record, not a claim that the order was followed without it.

## Tests written but not executed in this stage

None left to a pipeline. Every suite this unit adds ran here, including the `#[ignore]`d adapter
and migration suite against a disposable Postgres. There is still no CI platform, so nothing is
deferred to one.

## Gate results

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass — "No issues found" |
| `cargo test --workspace` (with `TEST_DATABASE_URL`) | pass |
| `cargo test --workspace -- --ignored` | 2 failures, both pre-existing and outside this unit (see below) |
| `bash scripts/check-boundaries.sh` | **FAIL — pre-existing, outside this unit** |
| `yarn --cwd support-console lint` | pass |
| `yarn --cwd support-console build` | pass (`✓ built in 619ms`) |
| `yarn --cwd support-console test` | pass — 38 files, 272 tests |
| `yarn --cwd support-console format:check` | **FAIL — 11 files, all pre-existing and untouched** |

**`check-boundaries.sh`** fails on `reporting-domain must not import framework crates:
['serde', 'serde_json']`. `services/reporting` has no change in this working tree and this unit
does not touch it; the gate was already red. **Not fixed here** — repairing another crate's
architecture is not this unit's change, and silently widening the check would hide it.

**`format:check`** is red on `src/auth/supabase.{ts,test.ts}`, `src/components/IncidentForm.*`,
`src/components/IncidentList.tsx`, `src/components/SupportSignInGate.*`, `src/components/
TicketQueue.tsx`, `src/components/ui.ts`, `src/pwa/isolate.ts` and `src/screens/SupportApp.test.tsx`.
Each was checked against `main` with the installed Prettier 3.9.10 and is already dirty there
(`package.json` declares `^3.4.2`). Every file this unit created or changed was formatted; the 11
left are other units' and were deliberately not reformatted. **Not relaxed** — the gate is
recorded red and the cause named.

**Two pre-existing `#[ignore]`d failures** in `identity`'s repository suite:
`a_user_is_found_by_the_hashed_subject_and_the_subject_is_protected_at_rest`
(`Conflict("the account is no longer pending deletion; the erasure was refused")`) and
`protect_and_unprotect_are_security_definer_functions_the_app_may_execute`
(`a destroyed key no longer decrypts the subject`). Neither calls `migrations::revert`, neither
touches support access, and both sit in U5 account-lifecycle and pgsodium territory. They are
reported, not converted.

## Deviations from the plan

1. **`"the caller's unrouted secret"` (Step 16) is not implementable.** The gateway holds no
   support secret and `ForwardRequest` has no secret field. The HTTP test proves what exists:
   the path routes to Identity, the JWT is forwarded unchanged, and an undeclared sibling GET is
   `404` with zero upstream calls.
2. **`DELETE /v1/support-grants/mine`** remains matched by the revoke pattern (decision 8).
3. **`u32` → `usize`** for the read's bound (decision 7).
4. **No support-specific metric** was added for the new handler; observability comes from the
   existing `http_requests_total` route label. The plan asked for no metric, and the
   infrastructure design records the missing latency series as a platform gap.
5. **Three test files outside the unit's own list** were edited for migration ordering and one
   pre-existing stale adapter test (`the_repository_lists_one_status_newest_first_and_bounded`,
   whose seeds violated `support_grants_live_pair_idx`). Necessary because `0008` is now newest;
   no assertion weakened.
6. **A `cargo clean -p identity-infrastructure` was needed** once: a stale sqlx macro build hid
   the new migration until the crate was rebuilt. Build cache, not logic.
7. **One temporary test log was written to `/tmp`** during debugging and deleted immediately; no
   project file was affected. Two throwaway diagnostic captures in `/tmp` were also removed.
8. **`SupportShell.test.tsx` cleanup** now uses `history.replaceState`: jsdom queues a
   `hashchange` when `location.hash = ""` runs on a URL with no fragment, and the stray event
   reset the section in the next test. The shell's behaviour was correct; the test's cleanup was
   not. The incidents test also gained an `aria-current="page"` assertion (stronger, not weaker).
9. **`useSupportMetadata.test.tsx`** waits on `onAccessEnded` and the dropped cache key rather
   than on `isError`, because those paths recreate the observer and `isError` is unobservable
   there. Behaviour unchanged; the assertion now matches what the plan specifies.
10. **Shared message constants** live in `src/domain/presentation.ts` (the one home Step 12
    allowed).

## Open items handed forward

- `check-boundaries.sh` and `format:check` are red for reasons this unit did not cause; both need
  a decision outside it.
- The two pre-existing `#[ignore]`d identity failures need the same.
- 80% per-crate Rust coverage remains unmeasured for this change.
- `scripts/smoke-identity.sh` still cannot cover S2: it has no support-role fixture, so the live
  path needs a manual pass under a real support session until that fixture exists.