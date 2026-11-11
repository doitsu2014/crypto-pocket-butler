# Code Summary — u2-authorisation

## What was built

U2 adds the authorisation decision to the existing Identity service — no new
deployable, no new table, no new Postgres role. One implementation is reachable
two ways: in-process from every Identity module, and through the C4
`POST /internal/authorize` adapter for services outside Identity.

### Domain (`services/identity/domain`)

- `authorisation.rs`: `Role` (re-export of the claims role, now including the
  job-only `System` variant), `OperationClass`, `Operation`,
  `ResourceScope`/`ResourceOwnership`, `CallerIdentity`, `DenialReason`, and
  `Decision`. `Role::provider_value()` names the provider vocabulary
  (`owner`/`admin`/`support`); `Role::from_claim` remains the single claim entry
  point and can never produce `System` (BR2.2).
- `Decision` maps every reason to its status: 200 allow, 403 for
  `no_role`/`status_denied`/`not_declared`, 404 for `not_owner`, and 503 only for
  `audit_unavailable` (BR2.6, BR2.11).

### Application (`services/identity/application`)

- `authorisation::catalogue::OperationCatalogue`: the closed table — owner-scoped
  names, collections, `guard:confirm`, `support:metadata:read`, any `admin:*`,
  and `Undeclared` for everything else (BR2.1).
- `authorisation::decision::AuthorisationDecision`: the W1 order — declared,
  status, support, class, ownership. Administrative operations are decided by the
  role alone, collections return the caller's own owner filter, a single row is
  compared to the caller, a job acts only for a stated owner, and a missing
  status row fails closed. Every denial writes through `AuditSink` before it
  returns; an audit failure becomes `audit_unavailable` (503). A status-source
  failure is `DecisionError::StatusUnavailable`, not a decision.
- `authorisation::role_change::RoleChange`: admin check, reason validation, target
  load, intent entry, provider write, outcome entry. It aborts before the provider
  write when the intent cannot be recorded, and records `failed` when the provider
  refuses (BR2.10).
- `authorisation::ports`: `StatusReader`, `StatusReadError`,
  `AuthorisationEvent` (the fixed audit field set), `AuditSink`, `RoleWriter`,
  `ObservationSink`.

### Infrastructure (`services/identity/infrastructure`)

- `authorisation::status::StatusReaderLocal`: one indexed `SELECT status` over the
  service pool, 200 ms pool acquire, 100 ms `SET LOCAL statement_timeout`, no
  retry. A malformed status is a failure, never `active`.
- `authorisation::status_cache::StatusCache`: outage-only. Successful reads warm
  it; it is consulted only after a failed read; TTL 30 s, cap 10 000, LRU
  eviction. A deleted/vanished user removes its entry.
- `authorisation::audit::AuthorisationAuditSink`: one C9 write, 250 ms ceiling, no
  retry, with the redaction backstop (JWT-shaped value, `0x`+40-hex wallet,
  secret-shaped context key) refusing the write.
- `authorisation::role_writer::SupabaseRoleWriter`: reads the target user's
  current `app_metadata`, merges `role` into it, then writes
  `PUT /admin/users/{subject}`; no retry. Reading first preserves
  `has_exchange_credentials`, which drives the MFA requirement, and every other
  provider key.

### HTTP (`services/identity/http`)

- `POST /internal/authorize` and `PUT /internal/admin/role`, reached directly on
  the internal network and absent from the gateway's `/v1` table.
- `PUT /v1/admin/users/{userId}/role`: the admin surface through the gateway,
  JWT-authenticated; the gateway requires the admin role at the edge and
  Identity re-checks it from the same forwarded token. Both role-change
  surfaces call the one `RoleChange` use case.
- `require_internal_caller` validates the caller's service token (`x-service-token`)
  and the user's JWT (`Authorization`, unchanged) against JWKS, attaching both.
- `/internal/authorize` rejects `callerSubject`/`roleClaim` with 400, enforces the
  configured service allowlist plus a **per-operation** allowlist, resolves
  the JWT subject to the internal user, and answers the decision: the `Decision`
  document on allow; an RFC 7807 problem whose `type` ends in the reason on a
  denial.
- `/internal/admin/role` maps `{userId,newRole,reason}` to `RoleChange` and answers
  `{userId,previousRole,newRole}`; the gateway route takes `{newRole,reason}` and
  the target from the path.
- New `authorisation_*` metrics (decisions by class/outcome/reason, a duration
  histogram, status-lookup failures, audit-write failures, role changes, cache age
  and entries) and a `route="internal.authorize"` label on `http_requests_total`.

### Configuration and deployment

- `IDENTITY_AUTHORIZE_SERVICES` (default `account-api,portfolio,identity-smoke`)
  wires the C4 service allowlist. No new crate dependency.
- `infra/swarm-stack.yml`: `account-api` gains `IDENTITY_URL: http://identity-api:8083`;
  `portfolio`'s `IDENTITY_URL` corrected from `:8089` to `:8083`; the gateway
  gains an explicit `IDENTITY_URL: http://identity-api:8083`; the
  `identity_service_tokens` header comment now names the `account-api`,
  `portfolio`, and `identity-smoke` entries the C4 endpoint needs.
- Gateway `PUT /v1/admin/users/{userId}/role` declared (`requires_admin`), so the
  admin role change is reachable from the user surface without a gateway service
  token: the gateway forwards the admin's JWT and Identity re-checks it.
- `scripts/smoke-identity.sh`: health, readiness, one allowed decision, and one
  `not_owner` decision.

### Documentation

- `contract-summary.md` C4 raised to version 2.0: identity from the JWT, the
  `x-service-token` header, the catalogue operation enum (including
  `guard:confirm` and the classes), 400/401/403/404/503, `not_declared` and
  `audit_unavailable`, the role-change body, and the resolved open question.
- `docs/architecture/README.md` and `docs/architecture/workspace.dsl` updated
  together: the communication rule for the decision and the Account/Portfolio →
  Identity relationships.

## How to run

```bash
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/check-boundaries.sh
```

Unit-scoped commands:

```bash
cargo test -p identity --test authorisation_domain
cargo test -p identity --test authorisation_application
cargo test -p identity --test authorisation_http
cargo test -p identity --test authorisation_adapters
cargo test -p identity --test repositories authorisation            # cache tests, no DB
TEST_DATABASE_URL=postgres://… cargo test -p identity --test repositories -- --ignored authorisation
```

## Red/Green evidence (this build)

TDD ordering: each layer's test was written first and the failing run recorded
before the implementation.

| Step | Red | Green |
|---|---|---|
| Runner (step 2) | — | `cargo test -p identity --test authorisation_domain -- --list` → `0 tests, 0 benchmarks` |
| Data model (steps 3–5) | `cargo test -p identity --test authorisation_domain` → `error[E0432]`: unresolved `CallerIdentity`, `Decision`, `DenialReason`, `Operation`, `OperationClass`, `ResourceOwnership`, `ResourceScope`, `Role` | `cargo test -p identity --test authorisation_domain` → 8 passed |
| Repository (steps 6–8) | `cargo test -p identity --test repositories authorisation` → `error[E0432]`: unresolved `StatusReader`, `StatusReaderLocal`, `StatusCache`; then `E0308` on the TTL type | `cargo test -p identity --test repositories authorisation` → 3 passed, 1 ignored (the DB-backed status read) |
| Business logic (steps 9–11) | `cargo test -p identity --test authorisation_application` → `error[E0432]`: unresolved `OperationCatalogue`, `AuthorisationDecision`, `DecisionError`, `RoleChange` | `cargo test -p identity --test authorisation_application` → 18 passed |
| API (steps 12–14) | `cargo test -p identity --test authorisation_http` → `error[E0425]: cannot find function authorisation_harness in this scope` | `cargo test -p identity --test authorisation_http` → 12 passed |
| Whole suite | baseline before changes: 121 passed, 16 ignored | `cargo test --workspace` → every suite `ok`; identity: 170 passed, 17 ignored; gateway: 68 passed |

### Review fixes (the round after the first review)

All four majors and three minors from the review are fixed, each with its test
first.

| Finding | Red | Green |
|---|---|---|
| 1. A job was not bound to an owner: a system caller allowed any row and a filterless collection | `cargo test -p identity --test authorisation_domain --test authorisation_application` → `E0599` no method `owner`; `E0061` `CallerIdentity::system` takes 1 argument (2 supplied) | 8 domain + 19 application tests pass; the system variant now carries a required `owner: UserId`, a row is allowed only when its owner equals it, and a collection always filters to it |
| 2. The admin role change was unreachable through the gateway | `cargo test -p gateway --test domain --test http` → `the_admin_role_change_route_is_declared_and_requires_admin` panicked on the missing route; `cargo test -p identity --test authorisation_http` → the `/v1` role-change test got the fallback 404 | gateway `PUT /v1/admin/users/{userId}/role` (requires admin) forwards the unchanged JWT; Identity serves the same path and calls the one `RoleChange` use case; gateway 12 + 16 and identity 13 HTTP tests pass |
| 3. The role writer erased `app_metadata` | `cargo test -p identity --test authorisation_adapters` → `the_role_writer_merges_the_role_and_keeps_the_rest_of_app_metadata` saw 1 request (expected a read then a write); the 5xx-on-read case did not fail | the adapter GETs the user, merges `role` into the existing `app_metadata`, then PUTs; 6 adapter tests pass |
| 4. The two security adapters had no adapter tests | the same test target did not exist | `cargo test -p identity --test authorisation_adapters` → 6 pass: fixed audit field set + headers, 4xx/5xx, token-shaped and wallet-shaped values refused without sending, merged role write, refused/unavailable provider |
| 5. `may_ask` was per class | a new HTTP case asked `account-api` for `admin:users:list` and `guard:confirm` | the allowlist names operations; `account-api`/`portfolio` get owner-scoped and collection names only, `identity-smoke` gets `principal:read` only |
| 6. C4 consumers did not match the allowlist | documentation only | the C4 row and prose now name `account-api`, `portfolio`, `identity-smoke`, the gateway admin surface, and in-process Identity modules; U3 is no longer listed as an HTTP consumer |
| 7. A redaction refusal on the role change surfaced as 422 | a new test makes the audit sink return `RepositoryError::Validation` | every audit failure on the role-change path maps to `AppError::Unavailable` (503) through one `audit_failure` helper |

`cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`
were re-run clean after the fixes.

`cargo fmt --all -- --check` → clean. `cargo clippy --workspace --all-targets -- -D warnings`
→ clean.

## Coverage (honest report)

`cargo llvm-cov -p identity-domain -p identity-application -p identity-infrastructure
-p identity-http --summary-only` **could not run** in this environment:
`error: failed to find llvm-tools-preview, please install llvm-tools-preview, or
set LLVM_COV and LLVM_PROFDATA environment variables`. `cargo-llvm-cov 0.9.1` is
installed; its `llvm-tools-preview` rustup component is not. The 80% per-crate
line target for this unit's lines is therefore **unverified**, not met-by-claim.
To run it:

```bash
rustup component add llvm-tools-preview
cargo llvm-cov -p identity-domain -p identity-application -p identity-infrastructure -p identity-http
```

The non-ignored suites cover the decision, catalogue, role change, cache, HTTP
adapter, and metrics paths. The adapter crates gain their remaining coverage from
the `#[ignore]` Postgres suite.

## Known gaps / issues

- **Coverage unverified** (above): the tool is present, its LLVM component is not.
- **The admin role change uses the gateway's JWT-authenticated `/v1` surface**, not
  a new gateway service token: the gateway already enforces `requires_admin` and
  Identity re-checks the role from the same forwarded JWT. The C4
  `PUT /internal/admin/role` remains for a service caller that presents
  `x-service-token`; it is not on the admin UI's path. No new gateway secret was
  introduced, and the existing `identity_service_tokens` `gateway` entry (C3)
  still covers the gateway's internal reads.
- **The DB-backed status test did not run here**: no disposable Postgres /
  `TEST_DATABASE_URL` in this environment. `authorisation_status_read_by_user_id_sees_the_stored_status`
  is `#[ignore]`d as designed and needs `pgsodium`.
- **Structurizr validation was blocked by the environment**, not by the model:
  `docker run … structurizr validate` parsed the workspace and then failed on
  `The content from https://static.structurizr.com/themes/default/theme.json could
  not be loaded: static.structurizr.com: Name does not resolve` (no outbound DNS
  from the container here). Re-run the README §3 command on a networked Docker
  host before review; no generated artefact was committed.
- **`portfolio-worker` still points `IDENTITY_URL` at `:8089`.** This unit's plan
  named only the `portfolio` API service; the same wrong port remains on the
  worker (and on `job-orchestration`, `reporting-api`, `reporting-worker`). Those
  callers own their own wiring; flagging it rather than silently widening scope.
- **The smoke script needs one more environment variable than the CI/CD design
  listed**: `SMOKE_USER_ID` (the test user's internal id) beside
  `C4_SMOKE_TOKEN` and `SMOKE_USER_JWT`. The C4 body's `resourceOwnerId` must be
  that id, and the service token alone cannot discover it without also computing
  the subject hash and calling C3.
- **Service-token transport choice**: the user's JWT stays in `Authorization`
  (the project's forwarding rule) and the calling service's token moves to
  `x-service-token`. The C4 contract now states this; callers must use it.
- **Metric path label**: `authorisation_decision_duration_seconds` is recorded
  with `path="decision"` rather than the observability design's `in_process` /
  `c4_endpoint`, because threading a transport fact into the application decision
  would violate the layer rule. The C4 hop is isolated by the new
  `route="internal.authorize"` label on `http_requests_total` instead.
- **The duration histogram emits no zero series** before the first decision; the
  decision counter emits one zero series (`not_declared`) so the family always
  exists.
- **Pre-existing boundary-gate failure** (`reporting-domain` → `serde`/`serde_json`)
  is unchanged and is not this unit's.
- The frontends are untouched; step 15 was correctly "not applicable".
