# Unit Test Instructions — u10-admin-users-ui

**Testing Contract:** `sha256:3f8863a9358b0b5e19a34ad2fc282a634f0d5323fb50ae1f7429a92a216766c3` (TDD, Standard strategy, brownfield)
**Target marker for this unit:** `AIDLC-UNIT: u10-admin-users-ui`

## Frameworks and configuration (already present — nothing is added)

| Layer | Framework | Configuration |
|---|---|---|
| Rust domain/application/HTTP | `cargo test` with the crates' existing `tests/` suites | Workspace `Cargo.toml`; no new crate or dev-dependency |
| Rust adapter (`#[ignore]`) | `cargo test -- --ignored` with `TEST_DATABASE_URL` | The disposable-Postgres gate the team affirmed; the probe in `scripts/` is unchanged |
| Console | Vitest 3 + Testing Library + jest-axe | `support-console/vitest.config.*`; `msw@^2.7.0` is already a devDependency — no dependency change |

There is no console coverage threshold (unresolved, team memory). The Rust floor is **80% line coverage per touched crate**, measured with `cargo llvm-cov` on the full suite by Build and Test; a unit-scoped run does not judge it.

## Exact commands for THIS unit

Run from the repository root unless stated.

```
# Rust — the five crates this unit touches
cargo test -p gateway-domain -p gateway-http -p identity-application -p identity-http -p identity-infrastructure

# Rust — adapter suites that need a real Postgres (blocking gate)
TEST_DATABASE_URL=<disposable-db> cargo test -p identity-infrastructure -- --ignored

# Console — the U10 suites only (explicit paths; never a bare project-wide run)
yarn --cwd support-console test src/screens/admin/ src/components/RoleBadge.test.tsx src/api/client.admin.test.tsx

# Console — the existing suites U10 must keep green (client, shell, queue, cache)
yarn --cwd support-console test src/api/client.test.ts src/components/GrantApprovalQueue.test.tsx src/hooks/useClearSupportCacheOnErrors.test.tsx
```

The whole-project gates (`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `bash scripts/check-boundaries.sh`, `cargo test --workspace`, and the console's `test`/`lint`/`build`/`format:check`) run in Step 17 of the plan; they are not the per-step loop.

## Coverage targets

- **Rust:** 80% line coverage per touched crate (`gateway-domain`, `identity-application`, `identity-http`, `identity-infrastructure`), checked by Build and Test on the assembled project's full suite with `cargo llvm-cov`. A unit-scoped `cargo test` run is not judged against it.
- **Console:** no numeric threshold exists. The floor is the team's rule — one happy path plus at least two error or edge cases per suite, `jest-axe` on every rendered state, and no test that passes regardless of the implementation.

## TDD order for this unit

Per layer, Red first, then Green, then Refactor while green:

1. **Repository/data access** — `SupportGrantRepository::list_by_status` (fake-backed application test, plus one `#[ignore]` adapter test).
2. **Business logic** — `RoleReader::read_role`, then `AdminLookup::by_email_admin` including the audit ordering (`Failed` with `provider-unavailable` before `503`) and the retained `Success` event.
3. **API/endpoint** — gateway `Route` table first-match budgets and the removal of `GET /v1/admin/users`; Identity router statuses and `route_label` arms.
4. **Frontend behavior** — client functions and the deadline/`Retry-After` parsing, then `RoleBadge`, lookup, record, role change, status change, approvals, and the section/shell wiring.

Record each Red command's failing output before writing the implementation.

## Mocking and stubbing

- **Rust application tests** use hand-written port fakes in the crates' existing `tests/fakes.rs` / `tests/common/fakes.rs`; never a live provider.
- **Rust HTTP tests** drive the router with the crate's existing test harness; assert status codes and RFC 7807 problem bodies.
- **Console** uses MSW for every gateway call. U10 handlers must cover `200`, `403`, `404`, `409`, `422`, `429` (with `Retry-After`), `503`, a network error, and a delayed response for the 15 s deadline case. No test reaches a real service.
- The provider role read is faked at the port in application tests and exercised once against the adapter's own harness in the `#[ignore]` suite.

## Test data

- Emails: one exactly-matching address, one unregistered address (`404`), one malformed (`422`).
- Roles: `owner`, `admin`, `support`; the provider read returns each mapping that `Role::from_claim` defines (`owner`/legacy `user`/absent → `owner`).
- Statuses: all five (`pending`, `active`, `disabled`, `pending_deletion`, `deleted`) so the action table and the presentation mapping are each exercised.
- Grants: at least two pending grants (newest first) and one that the signed-in admin created, for the self-approval case.
- Reasons: a 1-character value, a 500-character value, and a 501-character value (client refusal plus the server's `422`).

## Not run in this stage

The `#[ignore]` adapter suite needs a disposable Postgres (`TEST_DATABASE_URL`) and is recorded as written-but-not-run in `code-summary.md` when no database is available in this environment. It is a blocking gate, not an opt-in: Build and Test runs it against the provisioned database.