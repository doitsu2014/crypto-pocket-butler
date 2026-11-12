# Integration Test Instructions — 261109-brd-002-users-roles

Standard test strategy: the key boundaries and the cross-Unit interactions. Unit-level suites and
their commands live in each Unit's `code-generation/unit-test-instructions.md`; this file covers
what a Unit's own run cannot prove — that the pieces still fit together after all ten Units landed.

All commands run from the repository root unless stated otherwise.

## 1. What counts as integration evidence here

| Boundary | What the test drives | Where it lives |
|---|---|---|
| HTTP edge → service | the real Axum router through `tower::ServiceExt::oneshot`, asserting status codes and RFC 7807 `ProblemDetails` bodies; no port is bound and no live server runs | `services/*/http` integration suites under `services/<svc>/tests/` |
| Service → Postgres | the real sqlx adapter against a real schema on a disposable PostgreSQL, migrations applied and reversed by the test | the `#[ignore]` adapter and migration suites |
| Gateway → upstream | the real route table, middleware and rate limiter against a stub upstream service; the JWT is forwarded unchanged and the upstream URL map is the real one | `services/gateway/tests/{domain,http}.rs` |
| Cross-service call | Account and Portfolio calling Identity's internal support-metadata route with a real service token; the Identity router answers from its own store | `services/account/tests/support_metadata.rs`, `services/portfolio/tests/support_metadata.rs` |
| Frontend → gateway | the real client module against MSW handlers bound to the real `/v1` paths and RFC 7807 bodies, in **jsdom** | `web/src/**/*.test.tsx`, `support-console/src/**/*.test.tsx` |

Browser caveat, stated once and carried into the results: every frontend "integration" run here
executes in **jsdom, a DOM emulator**, driven by Vitest 3. It proves component, hook, cache and
transport behaviour against contract-shaped answers. It cannot prove native browser behaviour
(fetch receiver binding, real navigation, real timers under load). No test in this project runs
in a real browser engine, so any target that requires one is `Unverified`, not `Met`.

There is no cross-service contract-test suite and no end-to-end browser journey suite anywhere in
the repository; both remain unresolved in team memory and are recorded as such rather than
implied by these runs.

## 2. Framework setup and configuration

Nothing to install — every framework is already configured and no Unit added a new one.

- Rust: `cargo test` with integration targets under `services/<service>/tests/`. Hand-written port
  fakes in `tests/common/fakes.rs` (or `tests/fakes.rs`) for application-level cases; no mocking
  library, no generated doubles. Database-backed cases carry `#[ignore]` and run only with
  `TEST_DATABASE_URL` set.
- SPA: Vitest 3 + Testing Library + jest-axe + MSW 2 (`web/package.json`).
- Support console: Vitest 3 + Testing Library + jest-axe + MSW 2
  (`support-console/package.json`); `src/test/supportHandlers.ts` is an opt-in handler module, not
  a global `setupServer` default.
- Accessibility: `jest-axe` runs on every rendered state. It runs in jsdom, so an axe pass is a
  markup and ARIA check, not a user-agent conformance run.

## 3. How to run the boundary suites

```bash
# Gateway route table, middleware, limiter and upstream forwarding
cargo test -p gateway --test domain
cargo test -p gateway --test http

# Identity HTTP boundary across every Unit's routes (C1, C3, C6, C7 and the /v1 owner routes)
cargo test -p identity --test http
cargo test -p identity --test application
cargo test -p identity --test authorisation_http --test support_http --test consent_http \
  --test account_lifecycle_http

# Cross-service calls into Identity's internal metadata route
cargo test -p account --test support_metadata
cargo test -p portfolio --test support_metadata

# Frontend integration (jsdom + MSW) — whole packages, the assembled-bundle view
yarn --cwd web test
yarn --cwd support-console test
```

## 4. The blocking database gate

The `#[ignore]`d adapter and migration suites are a gate, not an opt-in: every run provisions a
disposable PostgreSQL and points `TEST_DATABASE_URL` at it, so an adapter or migration regression
fails the change. Until a CI platform is chosen, the same local command a developer runs is the
gate.

```bash
# 1. a disposable server of the pinned image (pgsodium is required by several Identity cases)
docker run -d --rm --name cpb-test-pg -e POSTGRES_PASSWORD=postgres -p 55432:5432 \
  supabase/postgres:15.8.1.060

# 2. one database for the run
docker exec cpb-test-pg createdb -U postgres identity_test

# 3. the suites, serially for the migration-bearing ones
TEST_DATABASE_URL=postgres://postgres:postgres@localhost:55432/identity_test \
  cargo test --workspace -- --ignored --test-threads=1
```

What this gate covers: every versioned `<NNNN>_<name>.{up,down}.sql` pair applies, reverses and
re-applies (`0003`–`0008` plus the earlier ones); the one-role-per-schema rules; the insert-only
grants and the trigger that raises for `identity_app`; the pgsodium `protect`/`unprotect` path;
the consent outbox drainer's claim/backoff/dead semantics; the support-grant live-pair index and
the 20-row bound; and the account-lifecycle tombstone and erasure ordering.

Test data is namespaced per case (or the case rolls its transaction back) so cases run in any
order. No fixture contains a real token, credential, personal address or production identifier;
IP values are documentation ranges.

## 5. Cross-Unit interaction checks

These are the seams where one Unit's output is another Unit's input. Each is exercised by an
existing suite; none needs a new file.

| Seam | Interaction to prove | Suite |
|---|---|---|
| U1 → U2 | a resolved principal reaches the authorisation decision with its role, status and ownership, and the audit write precedes a denial | `cargo test -p identity --test authorisation_application` |
| U1 → U6 | the gateway-derived `x-client-ip` reaches the jurisdiction check's decision idempotency and audit | `cargo test -p gateway --test http client_ip`, `cargo test -p identity --test application jurisdiction` |
| U4 → U5 | a deletion request reads and erases the consent and preference rows through the retention role, and a pending-deletion account is refused elsewhere | `cargo test -p identity --test account_lifecycle_application`, the U4/U5 adapter suites |
| U3 → U9 → U10 | a support grant requested by an agent is readable by that agent (`/mine`), by an admin (pending list), and its metadata is served to Account and Portfolio with the service token | `cargo test -p identity --test support_http`, `cargo test -p account --test support_metadata`, `yarn --cwd support-console test src/screens` |
| U7/U8 → U2 | a settings or sign-up surface reaches only the routes the gateway declares, and an undeclared sibling path stays `404` with zero upstream calls | `cargo test -p gateway --test domain`, `cargo test -p gateway --test http` |
| U8 → U6 | a restricted jurisdiction is a terminal notice; the 403 carries no `restrictedBy` | `yarn --cwd web test src/features/signup-gates` |

## 6. Coverage targets appropriate to this level

- **Rust: 80 % line coverage per crate** (`cargo llvm-cov`), read from the full suite of the
  assembled project with the ignored suites included — never from a Unit-scoped run, and never
  lowered to make a run pass.
- **TypeScript:** no threshold is set (recorded as unresolved in team memory). The floor is per
  file: happy path plus at least two error or edge cases, and no test that passes regardless of
  the implementation.
- **Standard volume:** five to eight tests per component, unit tests plus integration tests for the
  key boundaries — which is what the ten Units deliver.

## 7. Environment and data management

- The disposable container is removed after the run (`docker rm -f cpb-test-pg`); nothing in it
  survives, so no case may depend on state another case left.
- The local developer stack (`deployment/localhost`) is **not** used by these suites: it is a
  convenience environment with permissive local tokens, not a test fixture, and not a production
  credential model.
- No test reads production configuration, secrets or data. Services read credentials through
  `*_FILE` variables from Docker secrets in a real deployment; the suites never exercise that
  path, which is a parity gap worth naming rather than a covered boundary.
- A live-stack smoke run (`scripts/smoke-identity.sh`) needs running services and real tokens. It
  is not part of this stage, and it still cannot cover the support-grant `mine` read, which has no
  support-role fixture.