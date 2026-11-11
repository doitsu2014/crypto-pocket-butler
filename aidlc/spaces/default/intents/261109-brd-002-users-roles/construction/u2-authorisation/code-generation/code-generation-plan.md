# Code Generation Plan — u2-authorisation

**Unit:** `u2-authorisation` (U2) · **Scope:** classic · **Depth:** Standard · **Test strategy:** Standard
**Source of truth:** `functional-spec.md`, `rules.md` (BR2.1–BR2.14), `entities.md`, `nfr-design/*`, `infrastructure-design/*`, `contract-summary.md` (C4, C9).

## Summary

- Builds: the authorisation decision as an in-process Identity module, the C4 `/internal/authorize` adapter, the admin role-change use case, and its adapters
- Touches: `services/identity/{domain,application,infrastructure,http}`, `services/identity/tests/*`, `infra/swarm-stack.yml`, `docs/architecture/`
- Tests: TDD across four layers — roughly 8 domain, 8 application, 6 repository, 10 HTTP cases — plus one smoke script

## Scope

No new service, no new table, no new Postgres role. The decision is a module of the Identity
service's `application` crate; the C4 route is a thin `http` adapter over it; the role change is
an `application` use case so BR2.10's ordering never sits in `http`.

## Decisions this plan settles (from the stages before it)

| Open point | Resolution in this build |
|---|---|
| Where the decision lives | `application` module, called in-process inside Identity; the C4 route adapts it for callers outside |
| Status read | One adapter over the service's own pool: 200 ms acquire, 100 ms statement, no retry; outage-only cache consulted after a failed or timed-out read |
| C4 identity | Subject and role from the validated JWT; a body carrying `callerSubject` or `roleClaim` is rejected with 400 |
| Denial audit | Inside the decision through `AuditSink`; an audit failure returns `audit_unavailable` → 503 |
| Role change | `RoleChange` use case: intent entry, provider write, outcome entry; abort before the write if the intent cannot be recorded |
| Support | `support:metadata:read` gets its own class, decided by the u3 grant check, not by ownership |
| Metrics | New `authorisation_*` metrics plus a `route` label on the existing request counter so the C4 route can be isolated |
| Caller wiring | `identity_service_tokens` entries for account and portfolio; add `account-api` `IDENTITY_URL`; fix `portfolio` from `:8089` to `:8083` |
| C4 contract | `contract-summary.md` C4 updated (identity from JWT, operation enum incl. `confirm`, 503, `not_declared`/`audit_unavailable`), with `docs/architecture/README.md` and the C4 model |

## Testing Contract

```json
{
  "version": 1,
  "methodology": "tdd",
  "source": "team",
  "ordering": "We write a failing test first for every piece of production code, then the implementation that makes it pass, and we land the test in the same change as the code it covers.",
  "scope": "classic",
  "test_strategy": "standard",
  "project_type": "brownfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "Standard: requirement-driven tests with a happy-path floor per component."
    },
    {
      "layer": "team",
      "text": "- **Methodology**: tdd\n- **Methodology evidence**: The team affirmed test-first development for all production code at\n  the previous practices interview and re-confirmed it in this run's evidence review. The\n  organisation baseline still applies underneath it: requirement-driven tests with a happy-path\n  floor per component (`memory/org.md`), acceptance criteria in Given/When/Then form\n  (`memory/phases/inception.md`), business rules tested in `application` use cases against port\n  fakes, and one integration test per adapter against a real Postgres schema\n  (`docs/development/README.md`). TDD governs when each of those tests is written, not what they\n  cover. The repository cannot prove write-order from history — there are 71 `.rs` files under\n  `services/*/tests` and the crate `tests/` trees (suites plus shared fakes and helpers), 50\n  frontend `*.test.ts(x)` files, and ten test files whose database-backed cases are marked\n  `#[ignore]` — so ordering remains a team commitment rather than an observable.\n- **Ordering**: We write a failing test first for every piece of production code, then the\n  implementation that makes it pass, and we land the test in the same change as the code it\n  covers.\n- **Test types and layout**: domain rule tests per service; application use cases against\n  hand-written port fakes in `tests/fakes.rs` or `tests/common/fakes.rs`; HTTP boundary tests\n  over the Axum routers with status codes and RFC 7807 problem bodies; infrastructure adapter\n  tests against a real Postgres schema, which are `#[ignore]`d by default and run only when\n  `TEST_DATABASE_URL` is set. The ignored adapter and migration suites are now a blocking gate,\n  not a developer opt-in: every run provisions a disposable Postgres database and points\n  `TEST_DATABASE_URL` at it, so an adapter regression fails the change. Until the CI platform is\n  chosen, that gate is run by the same local command a developer runs today. The frontends use\n  Vitest 3 with Testing Library and jest-axe; MSW mocks the gateway in the support console.\n  There is no end-to-end browser journey suite and no cross-service contract test (unresolved).\n- **Commands and runners**: the canonical build, lint and test commands are:\n  - Rust — `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,\n    `cargo test --workspace`, and `bash scripts/check-boundaries.sh` (the mechanical Clean\n    Architecture gate).\n  - Frontend — in both `web/` and `support-console/`: `yarn test` (`vitest run`), `yarn lint`\n    (ESLint 9 flat config), `yarn build` (`tsc -b && vite build`) and `yarn format:check`\n    (Prettier). `web` also defines `yarn size` (size-limit budgets: 300 kB initial entry chunk,\n    150 kB route chunk, 150 kB stylesheet, all gzip).\n\n  `aidlc/spaces/default/memory/project.md` holds this list as the single authoritative source;\n  `docs/development/README.md` and the per-package `package.json` scripts remain the\n  implementation detail behind it.\n- **Coverage**: 80% line coverage per Rust crate remains the affirmed target and is measured\n  with `cargo llvm-cov`, the coverage tool chosen at this interview. No TypeScript coverage\n  threshold has been set (unresolved). The happy-path-plus-two-error/edge-case floor per test\n  file still applies and a test that always passes regardless of implementation is not\n  acceptable (`memory/phases/construction.md`).\n- **Quality gates**: `bash scripts/check-boundaries.sh` is a real mechanical gate — it parses\n  `cargo metadata --no-deps` and fails on any outward crate edge. The Rust gates are\n  `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and\n  `cargo test --workspace`; the frontend gates are `yarn test`, `yarn lint`, `yarn build` and\n  `yarn format:check`. There is still no CI platform in the repository — no `.github/`, GitLab,\n  Jenkins or CircleCI configuration — and the team chose at this interview to leave the platform\n  open for now and record CI as an intention, so today these gates run when a person runs them.\n  Dependency and container-image scanning (`cargo audit`, Trivy) is required by BRD-012\n  NFR-SEC-14 and named in unit design records but has no configuration or pipeline."
    }
  ],
  "obligations": {
    "strategy": "standard",
    "strategy_volume": [
      "Five to eight tests per component.",
      "Unit tests plus integration tests for key boundaries.",
      "Add E2E, performance, or security tests when requirements demand them."
    ],
    "scope_floor": [
      "Keep the existing test suite green.",
      "This scope adds no extra new-test floor beyond the selected test strategy."
    ],
    "combination_rule": "Apply every selected-strategy obligation and every scope-floor obligation; neither replaces the other, and a targeted scope regression may add the narrowest necessary test type beyond the strategy default."
  },
  "plan_profile": {
    "methodology": "tdd",
    "runner_step": "Verify the existing test runner/configuration and record the exact unit-scoped command.",
    "runner_ready_before_first_test": true,
    "testable_layers": [
      "Data model / database behavior",
      "Repository / data access",
      "Business logic",
      "API / endpoint",
      "Frontend behavior"
    ],
    "steps": [
      "Project structure and production configuration skeleton.",
      "Verify the existing test runner/configuration and record the exact unit-scoped command.",
      "Data model / database behavior - Red: write the failing tests and record the failing command output.",
      "Data model / database behavior - Green: implement only enough behavior to pass.",
      "Data model / database behavior - Refactor: improve the implementation while tests stay green.",
      "Repository / data access - Red: write the failing tests and record the failing command output.",
      "Repository / data access - Green: implement only enough behavior to pass.",
      "Repository / data access - Refactor: improve the implementation while tests stay green.",
      "Business logic - Red: write the failing tests and record the failing command output.",
      "Business logic - Green: implement only enough behavior to pass.",
      "Business logic - Refactor: improve the implementation while tests stay green.",
      "API / endpoint - Red: write the failing tests and record the failing command output.",
      "API / endpoint - Green: implement only enough behavior to pass.",
      "API / endpoint - Refactor: improve the implementation while tests stay green.",
      "Frontend behavior - Red: write the failing tests and record the failing command output.",
      "Frontend behavior - Green: implement only enough behavior to pass.",
      "Frontend behavior - Refactor: improve the implementation while tests stay green.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:28b1b8f64ea2370ac2e8ea34ac6ae154b256412b60b0b2b72311ed876bfb0f5c",
  "contract_sha256": "sha256:3f8863a9358b0b5e19a34ad2fc282a634f0d5323fb50ae1f7429a92a216766c3"
}
```

## Steps

- [x] 1. **Structure** — `application/src/authorisation/` (`mod.rs`, `catalogue.rs`, `decision.rs`, `role_change.rs`, `ports.rs`), `domain/src/authorisation.rs` (Role, OperationClass, DenialReason, Decision), `infrastructure/src/authorisation/` (status reader, status cache, audit sink, observation sink, Supabase role writer), `http/src/authorisation.rs` (routes, DTOs), `services/identity/tests/authorisation_{domain,application,http}.rs`.
- [x] 2. **Verify runner** — `cargo test -p identity --test authorisation_domain -- --list` (0 tests at this point); record the unit-scoped commands in `unit-test-instructions.md`.
- [x] 3. **Data model — Red** — failing tests for `Role` mapping (`admin`/`support`/anything else → owner), `OperationClass` (unknown → undeclared), `DenialReason`/`Decision` totals (503 only from `audit_unavailable`).
- [x] 4. **Data model — Green** — implement the value types.
- [x] 5. **Data model — Refactor** — collapse the mapping into one `from_claim` entry point.
- [x] 6. **Repository — Red** — `#[ignore]` tests: status read by user id over the real schema; the cache serves only after a failed read and never past 30 s; the cache cap evicts.
- [x] 7. **Repository — Green** — `StatusReaderLocal` (200 ms acquire, 100 ms statement), `StatusCache` (TTL 30 s, 10 000 entries, LRU).
- [x] 8. **Repository — Refactor**.
- [x] 9. **Business logic — Red** — the catalogue and the W1 order: declared → status → support → administrative (role alone) → collection (filter) → ownership; a denial writes through `AuditSink` before returning; an `AuditSink` failure yields `audit_unavailable`; `RoleChange` writes intent, then provider, then outcome, and aborts on a failed intent write.
- [x] 10. **Business logic — Green** — `AuthorisationDecision`, `OperationCatalogue`, `RoleChange`.
- [x] 11. **Business logic — Refactor**.
- [x] 12. **API — Red** — HTTP tests: `/internal/authorize` 400 on `callerSubject`/`roleClaim`, 401 without a JWT, 200 allowed, 403 `no_role`/`status_denied`, 404 `not_owner`, 503 on audit failure; the role-change endpoint 403 non-admin, 422 without a reason, 200 success.
- [x] 13. **API — Green** — `AuthorizeRoute`, `RoleChangeRoute`, DTOs, route registration, `authorisation_*` metrics, `route = internal.authorize` label on the request counter.
- [x] 14. **API — Refactor**.
- [x] 15. **Frontend** — not applicable.
- [x] 16. **Deployment** — `infra/swarm-stack.yml`: `identity_service_tokens` entries for account and portfolio, `account-api` `IDENTITY_URL`, `portfolio` port corrected 8089 → 8083. `scripts/smoke-identity.sh` as specified in `cicd-pipeline.md`.
- [x] 17. **Environment/build** — config wiring for the service-token allowlist; no new crate dependency.
- [x] 18. **Documentation/traceability** — `contract-summary.md` C4 (identity from JWT, operation enum, 503, two reasons) with `docs/architecture/README.md` and `docs/architecture/workspace.dsl`; `code-summary.md`, `source-manifest.json`, `traceability.json`.

## Story-to-step traceability

| Story | Steps |
|---|---|
| US2.1–US2.9 | 1, 3–14 |
| US5.1–US5.3 | 9–14 (the rule; the Account/Portfolio services call C4) |
| US7.1–US7.4 | 9–14, 16 |

## Definition of done

All steps checked; `cargo test --workspace` green; the identity crates at or above 80% line
coverage for the lines this unit adds; `cargo fmt` and `cargo clippy -D warnings` clean; the
boundary gate unchanged (its pre-existing `reporting-domain` edge is not this unit's); the
smoke script runnable against the local stack; the C4 contract and the C4 model updated together.
