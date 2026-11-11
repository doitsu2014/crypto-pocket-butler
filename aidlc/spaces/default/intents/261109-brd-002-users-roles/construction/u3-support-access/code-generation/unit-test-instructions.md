# Unit Test Instructions — u3-support-access

**Upstream:** `code-generation-plan.md`, `memory/team.md` (`## Testing Posture` — tdd, standard),
`memory/project.md` (canonical commands, 80% coverage, blocking `#[ignore]` gate),
`.claude/rules/development.md`, `docs/development/README.md`.

## How this unit is tested

Test-first, one unit of work. Every production change in this unit starts with a failing test in a
file that ships in the same change. The runner already exists — this is a brownfield workspace with
a green suite — so the only setup is confirming the commands below run.

The four layers this unit exercises and the file each one lives in:

| Layer | File | Runs against |
|---|---|---|
| Domain rules | `services/identity/tests/support_domain.rs` | Nothing but the `identity-domain` types |
| Application use cases | `services/identity/tests/support_application.rs` | Hand-written port fakes in `services/identity/tests/common/` |
| HTTP boundary and adapters | `services/identity/tests/support_http.rs` (router) and `support_adapters.rs` (`#[ignore]`, real Postgres) | The Axum router, and a disposable Postgres |
| Cross-service boundary | `services/account/tests/support_metadata.rs`, `services/portfolio/tests/support_metadata.rs`, `services/gateway/tests/{domain,http}.rs` | Each service's own router; the gateway's route table and limiter |

## Commands — run these, scoped to the unit

Fast loop, no database. Run each while iterating; every one must exit 0 before moving on.

```sh
cargo test -p identity --test support_domain
cargo test -p identity --test support_application
cargo test -p identity --test support_http
cargo test -p gateway --test domain
cargo test -p gateway --test http
cargo test -p account --test support_metadata
cargo test -p portfolio --test support_metadata
```

Database-backed adapters and the migration pair — `#[ignore]`d by default, **blocking**, never
skipped when this unit ships:

```sh
TEST_DATABASE_URL="postgres://<user>:<pass>@<host>:<port>/<disposable-db>" \
  cargo test -p identity --test support_adapters -- --ignored
```

The database must be **disposable**: the migration cases apply `0007_support_access.up.sql`, reverse
it, and re-apply it. Never point this at a shared or production database. The gate provisions a
fresh database per run and drops it afterwards.

Whole-unit check before handing back:

```sh
cargo test -p identity -p gateway -p account -p portfolio
```

The workspace gates are unchanged and still run at the end of the unit:
`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`bash scripts/check-boundaries.sh`, `cargo test --workspace`.

## Coverage

`cargo llvm-cov --workspace` — the affirmed target is **80% line coverage per Rust crate**. This
unit adds code to four crates; check all four, not the workspace total:

```sh
cargo llvm-cov --package identity --package gateway --package account --package portfolio
```

A line that only a `#[ignore]`d adapter test covers still counts once that suite runs against a
disposable database — run the ignored suites before reading the number.

## Fakes and mocks

- **Application tests use hand-written fakes**, in the style the repository already uses
  (`tests/fakes.rs`, `tests/common/fakes.rs`). This unit needs fakes for the ports it adds:
  the grant repository (with a mode that returns zero rows from a conditional update), the
  audit-outbox sink (passing, failing, and failing only on the second call), and the two metadata
  sources (each can answer, time out, or answer slowly enough to prove they run concurrently).
- **No mocking library.** No test double is generated; every fake is a struct that records what it
  was asked, so a test can assert on the call and not only on the return value.
- **HTTP tests drive the real router** through `tower::ServiceExt::oneshot`, asserting status codes
  and the RFC 7807 `ProblemDetails` body — no live server, no port binding.
- **The gateway tests drive the real route table and limiter**; the upstream is a stub service. The
  429 cases must prove the token-hash bucket is separate from the IP buckets: an exhausted support
  budget must leave a normal route responding.
- **`contract` tests are not used.** No field of the C1/C2 owner contract is asserted by a
  separate contract suite; the Account and Portfolio boundary tests assert the shape directly.

## Test data

- Fixed ids as constants in the test file (`AGENT_ID`, `TARGET_ID`, `ADMIN_ID`), never random —
  no `Math.random`-style nondeterminism, and no `Date::now()` read inside an assertion. Time is a
  parameter: the grant's expiry and the outbox's backoff both take the clock, so a test passes the
  instant it means.
- The adapter suite creates its own rows inside a transaction it rolls back, or with ids namespaced
  to the test, so cases can run in any order and in parallel.
- No real token, JWT or secret in any fixture: the service-token tests hash a literal placeholder
  string, exactly as the existing registry tests do.
- No personal data in a fixture — the target user is an opaque id.

## What each file must cover

`memory/phases/construction.md` requires the happy path plus at least two error or edge cases in
every test file, and forbids a test that passes regardless of implementation. Concretely:

| File | Happy path | Edge and error cases |
|---|---|---|
| `support_domain.rs` | A grant in each of the four statuses resolves its effective status | Window outside 1–60 min rejected; blank or 501-character reason rejected; `expire_if_due` on an already-expired grant is a no-op |
| `support_application.rs` | Request → approve → read → revoke, each on the fakes | Self-approval refused; pending-deletion target refused; zero-rows conditional update → 409; audit append failure → 503 and the read never fans out |
| `support_http.rs` | The four routes answer with the documented status and body | Unknown grant and non-holder both get the same 403 body; a revoked or expired grant gets its own code; the backend-service token on a user route is refused |
| `support_adapters.rs` | Migration up, the write and its outbox row in one transaction, a clean drain | A second live grant is rejected by the index; the down migration refuses a non-empty outbox; two concurrent claims never take the same row |
| `support_metadata.rs` (×2) | The identity service token with the new operation gets the projection | A token without the operation gets 403; an unknown target gets 404; the response carries no holding, balance, value, transaction or credential field |
| gateway `domain.rs` | The four route entries resolve to the identity service | A missing `max_requests_per_minute` field defaults rather than failing to compile into a wrong limit |
| gateway `http.rs` | A request under the limit passes through | The limit trips with `Retry-After`; the token-hash bucket does not evict the IP windows |

## Failure is information

A red test in the discovery order this plan uses (data model → repository → business logic → API)
is the expected first result, not a surprise. Record the failing command and its output in
`code-summary.md` before writing the implementation, so the Red step is a fact rather than a claim.