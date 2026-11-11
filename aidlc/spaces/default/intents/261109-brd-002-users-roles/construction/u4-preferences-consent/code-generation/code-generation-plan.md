# Code Generation Plan — u4-preferences-consent

**Unit:** `u4-preferences-consent` (U4) · **Scope:** classic · **Depth:** Standard · **Test strategy:** Standard
**Source of truth:** `functional-spec.md`, `rules.md` (BR4.1–BR4.11), `entities.md`, `nfr-design/*`, `infrastructure-design/*`, `contract-summary.md` (C6, C9).

## Summary

- Builds: preferences read/write, the consent-state gate, grant/withdraw with a durable audit outbox, the history read, and migration `0005`
- Touches: `services/identity/{domain,application,infrastructure,http}`, `services/identity/migrations/0005_*`, `services/identity/tests/*`
- Tests: TDD across four layers — roughly 8 domain, 10 application, 7 repository and 9 HTTP cases — plus the migration's up/down against a disposable database

## Scope

No new service, no new Postgres role beyond `identity_erasure` (a login role for the erasure job),
and no new container: the outbox drainer is a task in `identity-api`.

## Decisions this plan settles (from the stages before it)

| Open point | Resolution in this build |
|---|---|
| `consent_records` exists | `0005` **alters** it: nullable new columns, batched backfill, one-to-one legacy mapping (`analytics`→`analytics`, `marketing`→`marketing`, `disclosures`→`terms`), `seq` added nullable and backfilled, `REVOKE UPDATE, DELETE` from `identity_app`, raising trigger, `identity_erasure` login role |
| Duplicate profile columns | `users.locale`/`base_currency`/`timezone` stay u1's provider-derived copy; `user_preferences` holds the explicit choice; the read returns `COALESCE(preference, users.*, default)` with a per-field `source` |
| Withdrawal serialisation | `pg_advisory_xact_lock(hash(user_id, kind))` before the read, `SET LOCAL statement_timeout = '2s'` |
| Audit durability | the consent insert and a `consent_audit_outbox` row commit together; the drainer posts with an `Idempotency-Key` |
| Down migration | `CREATE TABLE IF NOT EXISTS consent_records_legacy_archive`, copy `risk`/`age` events there first, then narrow — a second rollback re-runs cleanly |

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

- [x] 1. **Structure** — `domain/src/consent.rs` (ConsentKind incl. `marketing`, ConsentAction, ConsentEvent, PreferenceValue, closed sets), `application/src/preferences/` (read/write), `application/src/consent/` (state, grant, withdraw, history, ports), `infrastructure/src/preferences/` and `infrastructure/src/consent/` (repositories, outbox, drainer), `http/src/consent.rs`, `migrations/0005_preferences_consent.{up,down}.sql`.
- [x] 2. **Verify runner** — `cargo test -p identity --test consent_domain -- --list` (0 tests at this point); record the unit-scoped commands.
- [x] 3. **Data model — Red** — kind/action enums, the closed preference sets (locale, currency, timezone, notification keys), the `source` marker, refusal values.
- [x] 4. **Data model — Green** — implement the value types.
- [x] 5. **Data model — Refactor**.
- [x] 6. **Repository — Red** — `#[ignore]` tests: preferences upsert touching only supplied columns; the `DISTINCT ON` state read; the paged history; the transactional withdraw with the advisory lock; the outbox insert; the migration's up, its down, and a **second down** (the `IF NOT EXISTS` archive case).
- [x] 7. **Repository — Green** — the sqlx adapters and migration `0005`.
- [x] 8. **Repository — Refactor**.
- [x] 9. **Business logic — Red** — the state query's `missing` computation; grant validation; withdraw (no-op vs 409, inherited policy version, analytics non-blocking); the failure path when the outbox insert fails.
- [x] 10. **Business logic — Green** — the use cases behind their ports.
- [x] 11. **Business logic — Refactor**.
- [x] 12. **API — Red** — HTTP tests: `GET|PUT /v1/me/preferences` (422 on an out-of-set value), `GET /v1/me/consent/state`, `POST /v1/me/consent` grant, `POST /v1/me/consent/{kind}/withdraw` (200 no-op, 409 never-granted), `GET /v1/me/consent`, the C6 internal routes, 404 for another user, 503 on a read failure.
- [x] 13. **API — Green** — handlers, DTOs, route registration, the `consent_*` metrics and the `route` label.
- [x] 14. **API — Refactor**.
- [x] 15. **Frontend** — not applicable; U7 owns the screens.
- [x] 16. **Deployment** — the drainer task started in `identity-api`'s composition root; `chrono_tz` dependency; no stack change; the migration exercised by `identity-migrate`.
- [x] 17. **Environment/config** — the erasure role's credential path (secret, not env) and the drainer's C9 token reuse.
- [x] 18. **Documentation/traceability** — record the C6 and C9 amendments the design requires as open contract changes (C6 is owned by contract-design, C9 by the audit service); `code-summary.md`, `source-manifest.json`, `traceability.json`.

## Story-to-step traceability

| Story | Steps |
|---|---|
| US6.1 | 1, 3–14 |
| US6.2 (recording half) | 9–14 |
| US6.5 (recording half) | 9–14 |

## Definition of done

All steps checked; `cargo test --workspace` green; the migration up, down, and second down proven
against a disposable database; the identity crates at or above 80% coverage for the lines this unit
adds; fmt and clippy clean; the C6 and C9 amendments recorded rather than assumed.
