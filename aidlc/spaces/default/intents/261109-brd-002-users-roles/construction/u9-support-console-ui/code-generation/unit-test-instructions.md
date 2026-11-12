# Unit Test Instructions — u9-support-console-ui

**Upstream:** `code-generation-plan.md`, `memory/team.md` (`## Testing Posture` — tdd, standard),
`memory/project.md` (canonical commands, 80% per-crate coverage, the blocking `#[ignore]` gate),
`.claude/rules/development.md`, `docs/development/README.md`, `functional-design/frontend-components.md`
§6 (the suite list).

## How this unit is tested

Test-first, and the suite lands in the same change as the code it covers. This is a brownfield
workspace with a green suite, so setup is only confirming that the commands below run — there is no
runner to install and no config to add.

Two layers carry the work, and each has its own runner:

| Layer | Location | Runs against |
|---|---|---|
| Frontend behaviour (primary) | `support-console/src/**/*.test.ts(x)` | Vitest 3 + Testing Library in jsdom, with MSW 2 standing in for the gateway |
| Data model, repository, business logic, API (S2's server half) | `services/identity/tests/{support_application,support_adapters,support_http}.rs`; `services/gateway/tests/{domain,http}.rs` | Port fakes, the real Axum routers, and (for `support_adapters`) a disposable Postgres |

Everything in this file is scoped to this unit. The workspace-wide gates (format, clippy,
`scripts/check-boundaries.sh`) are Step 24 of the plan; they are not test commands and are not
needed in the inner loop.

## Commands — run these, scoped to the unit

Run the first command before the first Red, to prove the runner works:

```sh
yarn --cwd support-console test src/domain/grants.test.ts
```

Then, one file at a time, in the order the plan's steps take them:

```sh
yarn --cwd support-console test src/domain/metadata.test.ts
yarn --cwd support-console test src/api/client.test.ts
yarn --cwd support-console test src/api/queryClient.test.ts
yarn --cwd support-console test src/hooks/useClearSupportCacheOnErrors.test.tsx
yarn --cwd support-console test src/hooks/useMyGrants.test.tsx
yarn --cwd support-console test src/hooks/useSupportMetadata.test.tsx
yarn --cwd support-console test src/components/GrantRequestForm.test.tsx
yarn --cwd support-console test src/components/GrantBanner.test.tsx
yarn --cwd support-console test src/components/SupportMetadataPanel.test.tsx
yarn --cwd support-console test src/components/GrantApprovalQueue.test.tsx
yarn --cwd support-console test src/screens/SupportShell.test.tsx
```

The whole console suite for this unit, once the last step lands:

```sh
yarn --cwd support-console test
```

`yarn test` is `vitest run`; a path argument narrows it to that file. `yarn --cwd support-console
test:watch` is the interactive form while iterating. Yarn 4.18.1 through corepack only — no `npm`,
`npx` or `pnpm`.

The Rust half, fast loop (no database):

```sh
cargo test -p gateway --test domain
cargo test -p gateway --test http
cargo test -p identity --test support_application
cargo test -p identity --test support_http
```

The database-backed adapter and migration cases — `#[ignore]`d by default and **blocking**, never
skipped when this unit ships:

```sh
TEST_DATABASE_URL="postgres://<user>:<pass>@<host>:<port>/<disposable-db>" cargo test -p identity --test support_adapters -- --ignored
```

The database must be **disposable**: those cases create grants in the real `support_access_grants`
table and apply `0008_support_mine_index.up.sql`, reverse it, and re-apply it. Never point this at
a shared or production database. The gate provisions a fresh database per run and drops it
afterwards.

## Coverage

- **Rust: 80% line coverage per crate**, measured with `cargo llvm-cov` — `identity` and `gateway`
  are the two this unit adds lines to. Run the `#[ignore]`d adapter suite first, or the repository
  lines read as uncovered. Coverage is read on the full suite by Build and Test; it is not a
  per-file gate in the inner loop, and the target is never lowered to make a step pass.
- **TypeScript: no threshold is set** (unresolved in team memory), so there is no number to hit.
  What is required is the per-file floor from `memory/phases/construction.md`: a happy path and at
  least two error or edge cases in every file, and no test that passes regardless of implementation.

## Fakes and mocks

- **MSW mocks the gateway** — `support-console/src/test/server.ts` and the handlers beside it.
  Every handler answers a real `/v1` path (`/support-grants`, `/support-grants/mine`,
  `/support-grants/{grantId}/metadata`) with a real status and an RFC 7807 body where the route
  fails. A test that needs a `403`, a `409`, a `429` with `Retry-After`, a `503`, or a slow answer
  asks for that variant explicitly rather than reinterpreting the happy one.
- **No handler invents a route.** The three paths above are the whole surface; a handler for
  anything else is a defect, because the console must not depend on an endpoint that does not exist.
- **Time is a parameter.** `useGrantClock` takes the tick interval and components take `now`; a
  test passes the instant it means. No `Date.now()` inside an assertion and no fake-timer sleep
  longer than the tick it is proving.
- **The Rust side uses hand-written port fakes**, in the style `services/identity/tests/common/`
  already sets: a struct that records what it was asked, so a test can assert on the call as well
  as on the return value. No mocking library.
- **HTTP tests drive the real router** through `tower::ServiceExt::oneshot` — status codes and the
  `ProblemDetails` body, no live server and no port binding.
- **The gateway tests drive the real route table**, proving the `mine` entry is reachable and not
  shadowed by a broader pattern — the table is first-match, so ordering is part of the assertion.

## Test data

- **Fixed ids as constants** in the test file (`AGENT_ID`, `TARGET_ID`, `ADMIN_ID`, `GRANT_ID`),
  never random. The support console tests reuse the shapes in `src/test/fixtures.tsx`; a fixture
  that differs from the contract's row is corrected there, not worked around in a test.
- **Time is explicit** — an `expiresAt` is always passed, because a missing one has its own case
  (an `active` grant with no parseable expiry must read as expired).
- **No credential, token or secret in a fixture.** The token-shaped strings in the console tests
  are placeholders and nothing asserts on their content beyond presence.
- **No personal data.** The target user is an opaque id, and the metadata fixture carries only the
  projection's whitelisted fields that the read may legitimately return.
- **The adapter suite namespaces its rows** to the test (or rolls its transaction back), so cases
  run in any order and in parallel.

## What each file must cover

`memory/phases/construction.md` requires a happy path plus at least two error or edge cases in
every test file. The plan's Steps 2–22 name the specific cases per file; in summary:

| File | Happy path | Edge and error cases |
|---|---|---|
| `grants.test.ts` | The four statuses render and expire on the clock | A 1-character and a 500-character reason pass, 501 fails; a missing expiry fails closed |
| `metadata.test.ts` | The whitelist projection keeps exactly its fields | Over-sent fields dropped; malformed projection throws |
| `client.test.ts` | The three calls send and map the documented shape | No duration or approver on the request; `grantId ?? id`; malformed rejects; the retired calls are gone |
| `useClearSupportCacheOnErrors.test.tsx` | Both hooks clear on failure | `401` clears all, `403` clears only `["support", …]`; U10's existing hook unchanged |
| `useMyGrants.test.tsx` | Adopts the newest own grant | Disabled without a token or support role; no request on focus or timer; `403` clears |
| `useSupportMetadata.test.tsx` | One read per active grant, plus Refresh | No read while not active; `403` ends access and drops the key |
| `GrantRequestForm.test.tsx` | One request per submit, success line | Empty id, empty and 501-character reason, `409`, `404`, `403`, `422`, `429`; axe on every state |
| `GrantBanner.test.tsx` | Each of the four statuses with remaining time | Expired clock; the metadata-only reminder |
| `SupportMetadataPanel.test.tsx` | Accounts and portfolios render | Empty both; filter without a request; `403`, `401`, `503`; no read while inactive |
| `GrantApprovalQueue.test.tsx` | An admin approves another agent's grant | A non-admin's approve is disabled; self-approval copy |
| `SupportShell.test.tsx` | A support session adopts its newest grant without a `requestedBy` match | An admin approving another agent's grant never adopts it; `#search` still lands; role gating; sign-out clears |
| `support_application.rs` | `mine` returns own grants, newest first, ≤ 20 | An agent with none gets an empty list; a 21st is dropped |
| `support_http.rs` | `200` with a bare array | `403` without the support role; `401` without a token; the `route_label` arm names the route |
| gateway `domain.rs` / `http.rs` | The `mine` route resolves and forwards | Not shadowed by a broader pattern; an undeclared sibling path stays `404` |
| `support_adapters.rs` | The read is ordered and bounded on the real table; `0008` up/down/re-up | A 21st row is dropped by `MY_GRANTS_LIMIT`; the down migration removes the index |

## Failure is information

The first run of a new suite is expected to fail — that is the Red step, not a defect. Record the
failing command and its output in `code-summary.md` before writing the implementation, so the Red
step is a fact rather than a claim. A test that passes before the implementation exists is either
wrong or already covered: fix the test, never the production code, to get a red.