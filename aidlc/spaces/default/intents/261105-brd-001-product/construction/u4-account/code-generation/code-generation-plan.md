# Code Generation Plan — u4-account

## Summary

- Builds: the Account service — exchange, wallet, WalletConnect and CSV sources, sync runs with a
  lease, holding snapshots, the C5 reads, privacy export and anonymise, and the C11 run endpoint.
- Touches: `services/account/` (domain, application, infrastructure, http, bin, migrations,
  Dockerfiles) and `infra/swarm-stack.yml`.
- Tests: 120+ unit and integration tests across the four crates, TDD per layer, 80% line coverage
  per crate.

## Testing Contract

```json
{
  "version": 1,
  "methodology": "tdd",
  "source": "team",
  "ordering": "We write a failing test first for every piece of production code, then the implementation that makes it pass, and we land the test in the same change as the code it covers.",
  "scope": "classic",
  "test_strategy": "standard",
  "project_type": "greenfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "Standard: requirement-driven tests with a happy-path floor per component."
    },
    {
      "layer": "team",
      "text": "- **Methodology**: tdd\n- **Methodology evidence**: The team affirmed test-first development for all production code at\n  the practices interview. The organisation baseline still applies underneath it:\n  requirement-driven tests with a happy-path floor per component (`memory/org.md`), acceptance\n  criteria in Given/When/Then form (`memory/phases/inception.md`), business rules tested in\n  `application` use cases against port fakes, and one integration test per adapter against a\n  real Postgres schema (`docs/development/README.md`). TDD governs when each of those tests is\n  written, not what they cover. No project document mandated test-first before this decision,\n  so it is recorded here as the team's affirmed choice. *(Interview Q3.)*\n- **Ordering**: We write a failing test first for every piece of production code, then the\n  implementation that makes it pass, and we land the test in the same change as the code it\n  covers.\n- **Coverage**: 80% line coverage per Rust crate, enforced in CI. Every test file still covers\n  the happy path and at least two error or edge cases (`memory/phases/construction.md`), and a\n  test that always passes regardless of the implementation is not acceptable. The coverage tool\n  and the TypeScript coverage threshold are not chosen yet (unresolved). *(Interview Q4.)*\n- **Test types**: unit tests in `application` against port fakes; one integration test per\n  adapter against a real Postgres schema. The API and cross-service contract-testing approach\n  still needs agreement (unresolved).\n- **Quality gates**: a change merges only when the same build, lint, and test commands CI runs\n  pass locally and in CI (`memory/team.md`). Crate-boundary enforcement is mechanical and fails\n  the build; dependency and container-image vulnerability scanning runs before release\n  (`docs/development/README.md`, BRD-012 NFR-SEC-14). The CI platform and the full ordered gate\n  list are not configured yet (unresolved)."
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
    "runner_step": "Bootstrap the minimal test runner/configuration and record the exact unit-scoped command.",
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
      "Bootstrap the minimal test runner/configuration and record the exact unit-scoped command.",
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
  "input_sha256": "sha256:11051b759669571e98d125b24b25a8610902eff0f244cceaa6724770a7254c38",
  "contract_sha256": "sha256:7d1a8c2aedc4bcb706f31ebb282bc5359f3e85ec270ac7a4b81f955485af1f93"
}
```

## Scope note

Frontend behaviour is not applicable to this Unit: Account is a backend service, and the SPA screens
belong to u2-web-app. Its Red/Green/Refactor steps are omitted as genuinely inapplicable.

## Story traceability

| Plan step | Stories / rules |
|---|---|
| 1–2 | US1.2, US2.3 (service skeleton and runner) |
| 3–5 | BR6.1, BR6.2 (snapshot model, decimals) |
| 6–8 | BR1.2, BR4.1, BR4.2, BR5.2, BR5.4 (repositories and lease/dedupe indexes) |
| 9–11 | BR1.1–BR1.4, BR2.1–BR2.5, BR3.1–BR3.3, BR4.3–BR4.5, BR5.1–BR5.4, BR6.3, BR7.1–BR7.2, BR8.1–BR8.2 |
| 12–14 | C1 routes, C5 reads/privacy, C11 run endpoint, US1.2, US1.3, US2.1, US2.2 |
| 15–17 | not applicable (no frontend in this Unit) |
| 18 | Dockerfiles, swarm stack entries, health checks |
| 19 | code summary, traceability, source manifest |

## Steps

### Step 1: Project structure and production configuration skeleton
- [x] Create `services/account/Cargo.toml` as a workspace with `domain`, `application`,
      `infrastructure`, `http`, and a `src/bin/account.rs` API binary plus `src/bin/worker.rs`.
- [x] Add the four crates with inward-only dependencies and shared lint settings mirroring
      `services/identity`.
- [x] Add `.env.example` entries for the DSN, service role, service-token registry, and provider
      endpoints; no secret values.

### Step 2: Bootstrap the test runner and record the command
- [x] Confirm `cargo test -p account-domain -p account-application -p account-infrastructure
      -p account-http` runs before the first test-first cycle.
- [x] Record the exact unit-scoped command in `unit-test-instructions.md`.

### Step 3: Data model / database behaviour — Red
- [x] `domain`: tests for `AccountKind`, `AccountStatus`, `Venue`, `SyncRunStatus`,
      `CredentialStatus`, `HoldingSnapshot` decimal validation, `DedupeKey` derivation, label
      uniqueness and case folding, and private-key/seed-phrase detection.
- [x] `infrastructure`: migration tests against a disposable schema for the eight tables, the
      per-account unique label index, the unique `dedupeKey` index, the single-live-run constraint,
      and the `import_upload` staging row.

### Step 4: Data model / database behaviour — Green
- [x] Implement the domain types and the sqlx migrations `0001_init.up.sql` / `.down.sql`.
- [x] Implement the `import_upload` staging table with `batch_id` PK/FK, `bytes`, `byte_size`,
      `created_at`, and the 10 MiB cap check.

### Step 5: Data model / database behaviour — Refactor
- [x] Split large entities into typed modules; keep decimal parsing in one place; re-run tests.

### Step 6: Repository / data access — Red
- [x] Tests for `AccountRepository`, `CredentialRepository`, `WalletConnectionRepository`,
      `SyncRunRepository` (claim, renew, sweep, fenced snapshot write), `HoldingRepository`,
      `ImportRepository` (create, poll pending, complete, delete staging), and
      `ServiceTokenRegistryRepository`.

### Step 7: Repository / data access — Green
- [x] Implement the sqlx repositories behind the application ports; every query filters by owner
      or service identity.

### Step 8: Repository / data access — Refactor
- [x] Consolidate query helpers and error mapping; keep `sqlx::Error` inside infrastructure.

### Step 9: Business logic — Red
- [x] `application` tests against port fakes for: add-exchange-account (verify-first, no row on
      failure), replacement verify-then-swap, add-wallet (C4 chain resolution), add-WalletConnect,
      label uniqueness, resync 409/202, sync claim/lease/renew/fence, sync failure keeps holdings,
      periodic re-verification and revocation, CSV detection/dedupe/row errors/counts, holdings
      read, privacy export and anonymise, audit and notification events.

### Step 10: Business logic — Green
- [x] Implement the use cases, ports, and error types in `application`; map every BR1–BR8 rule.

### Step 11: Business logic — Refactor
- [x] Extract shared validation and event helpers; re-run the application suite.

### Step 12: API / endpoint — Red
- [x] `http` tests for the C1 routes (add exchange/wallet/WalletConnect, resync, list, import),
      the C5 routes (holdings, transactions, privacy export/anonymise), and the C11 run endpoint:
      status codes, DTO mapping, RFC 7807 errors, JWT/JWKS and service-token middleware, and
      correlation ids.

### Step 13: API / endpoint — Green
- [x] Implement the Axum routers, DTOs, middleware, `/healthz`, `/readyz` (8091 API, 8092 worker),
      and `/metrics`.

### Step 14: API / endpoint — Refactor
- [x] Remove duplication between handlers; keep DTOs out of `application`.

### Step 15–17: Frontend behaviour
- [x] Not applicable — this Unit has no frontend surface.

### Step 18: Environment/build configuration
- [x] Add `Dockerfile.api` and `Dockerfile.worker`; add both services to `infra/swarm-stack.yml`
      with replicas, limits, `/healthz` health check, rolling-update config, `internal` and
      `public` networks, and Docker secrets.
- [x] Add the migration job entrypoint and the Vault, C3, C4, C9, C10, C15 client configuration.

### Step 19: Documentation and traceability
- [x] Write `code-summary.md`, `source-manifest.json`, and `traceability.json`; record any test
      not executed and why.
