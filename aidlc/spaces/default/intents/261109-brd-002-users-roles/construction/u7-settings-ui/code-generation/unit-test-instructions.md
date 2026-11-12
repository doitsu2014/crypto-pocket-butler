# Unit Test Instructions — u7-settings-ui

**Test strategy:** Standard · **Methodology:** TDD (team contract) · **Project type:** brownfield
**Scope:** this Unit's Rust routes and SPA features only. Every command below is scoped to this
Unit; a project-wide command is never this Unit's run command.

## Test frameworks (already present — nothing to install)

| Side | Framework | Where it is configured |
|---|---|---|
| Rust | `cargo test` with integration targets under `services/identity/tests/` and `services/gateway/tests/` | the workspace `Cargo.toml` files; no new dependency |
| SPA | Vitest 3, Testing Library, `jest-axe`, MSW 2 | `web/package.json` + `web/vite` test config; T1 of `nfr-requirements/tech-stack-decisions.md` confirms no dependency is added |

## Exact commands for THIS Unit

Run from the repository root unless the command says otherwise. These are the commands the plan's
Red and Green steps use; a Red step is only valid once its command is known to run.

### Rust — Identity (D0, P1, P2, D1, D2, route labels)

```
cargo test -p identity --test http
cargo test -p identity --test application
cargo test -p identity --test repositories
```

- `--test http` carries the boundary tests over the Identity router (status codes, RFC 7807
  bodies, self-scoping, `route_label`).
- `--test application` carries the use-case tests against the hand-written port fakes in
  `services/identity/tests/common/fakes.rs`.
- `--test repositories` carries the adapter and projection tests. Its database-backed cases are
  `#[ignore]`d and run only with `TEST_DATABASE_URL` pointing at a disposable Postgres:
  `TEST_DATABASE_URL=... cargo test -p identity --test repositories -- --include-ignored`.
  Without that variable the suite runs its non-database cases and skips the rest, which is the
  expected local state.

### Rust — gateway (the eight route-table entries and the D3 budget)

```
cargo test -p gateway --test domain
cargo test -p gateway --test http
```

`--test domain` proves the eight paths are declared and every other path still `404`s;
`--test http` proves the entries resolve and that D3 answers `429` on the sixth request in a window.

### SPA (all settings and account-status behaviour)

From `web/`:

```
yarn test src/features/settings
yarn test src/features/account-status
yarn test src/api
```

`yarn test` is `vitest run`; the trailing argument is a path filter, so each command runs only this
Unit's files. The whole-package run is `yarn test` with no filter — that is the general gate, not
this Unit's run command.

## Expected coverage targets

- **Rust:** the project floor is **80% line coverage per crate**, measured with `cargo llvm-cov`.
  That floor is judged by Build and Test on the full suite of the assembled project, not by a
  Unit-scoped run: on this Unit's tests alone a per-crate threshold can fail on source other Units'
  tests cover, with every selected test passing. If that happens, record it with this Unit's
  results — it is not a failed test — and never lower the threshold.
- **TypeScript:** no coverage threshold is set for `web/` (recorded as unresolved in team memory).
  The floor that does apply is per test file: a happy path plus at least two error or edge cases,
  and no assertion that passes regardless of the implementation.
- **Volume (Standard strategy):** five to eight tests per component, unit tests plus integration
  tests for the key boundaries. Concretely: about 34 Rust tests across the six targets above, and
  about 44 SPA tests across the ten files named in `frontend-components.md` §6.

## Mocking and stubbing

- **Rust application layer:** hand-written port fakes in `services/identity/tests/common/fakes.rs`,
  extended only where the new use cases need a port the fakes do not yet carry. No mocking library,
  no network, no database.
- **Rust HTTP layer:** the router built with the same fakes/state helpers the existing boundary
  tests use; assertions on status codes and RFC 7807 bodies, never on rendered prose.
- **Rust repository layer:** a real Postgres schema through `TEST_DATABASE_URL`, `#[ignore]`d by
  default. Never a mocked `sqlx` connection — an adapter test that mocks its own SQL proves
  nothing.
- **SPA:** **MSW 2** mocks the gateway for every test in this Unit — no test reaches a real service
  and no test stubs `fetch` by hand. The handlers live with the feature tests and must include the
  cases the design depends on: a delayed handler and a never-resolving handler for the deadline
  paths, `404` with the pinned
  `https://crypto-pocket-butler.example/problems/no-record` type for D1's empty state, and a `404`
  with **any other** type to prove the client treats it as an error.
- **Accessibility:** `jest-axe` runs on every rendered state (shell, four panels, delete page, both
  account-status screens). An axe violation fails the test.

## Test data

- **SPA:** small typed fixtures per account status (`active`, `disabled`, `pending_deletion`,
  `deleted`), one `ProfileView` with and without a provider display name, one `DeletionView` with an
  empty `retainedCategories` and one non-empty, and consent history with and without a withdrawal
  event. Fixtures are shared through the feature's test folder, not duplicated per file.
- **Rust:** the existing fakes' builders; SQL fixtures go through the repository suite's own setup,
  which creates the rows each case needs and cleans up with the disposable database.
- **No personal data:** fixtures use obviously synthetic names and addresses, and no test asserts on
  a real user's data.

## Definition of done for this Unit's tests

- Every Red step recorded the failing command's output before its Green step.
- The tests landed in the same change as the code they cover.
- `cargo test -p identity --test http`, `-test application`, `-test repositories`,
  `cargo test -p gateway --test domain`, `-test http`, and the three `yarn test` filters all pass.
- `bash scripts/check-boundaries.sh` passes — the new code stays inside the Clean Architecture
  edges.