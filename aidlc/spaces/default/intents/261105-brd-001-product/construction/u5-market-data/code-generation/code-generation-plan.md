# Code Generation Plan — u5-market-data

## Summary

- Builds: the Market Data service — C6 prices and FX with priority selection, freshness, provenance
  and the unpriced result; two leased refresh jobs; the C11 run endpoint.
- Touches: `services/market-data/` (four crates, two binaries, migrations, Dockerfiles) and
  `infra/swarm-stack.yml`.
- Tests: 90+ unit and integration tests across the four crates, TDD per layer, 80% line coverage per
  crate.

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
    "testable_layers": ["Data model / database behavior", "Repository / data access", "Business logic", "API / endpoint", "Frontend behavior"],
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

Frontend behaviour is not applicable: Market Data is a backend service; the SPA reads valuations
through Portfolio.

## Story traceability

| Plan step | Stories / rules |
|---|---|
| 1–2 | service skeleton and runner |
| 3–5 | BR1.3, BR2.1, BR3.3 (observation/rate model) |
| 6–8 | BR1.1, BR4.1, BR5.3 (selection query, leases, retention) |
| 9–11 | BR1.1–BR1.5, BR2.1–BR2.4, BR3.1–BR3.3, BR4.1–BR4.5, BR5.1–BR5.3 |
| 12–14 | C6 routes, C11 run endpoints, US3.3, US3.4 |
| 15–17 | not applicable |
| 18 | Dockerfiles, swarm stack, health checks |
| 19 | code summary, traceability, source manifest |

## Steps

### Step 1: Project structure and production configuration skeleton
- [x] Create `services/market-data` workspace with `domain`, `application`, `infrastructure`,
      `http`, `src/bin/api.rs`, `src/bin/worker.rs`, `src/bin/migrate.rs`.
- [x] Mirror the identity/account crate layout and inward-only dependencies; add the workspace
      member to the root `Cargo.toml`.
- [x] Add `.env.example` entries for the DSN, run token, and provider endpoints.

### Step 2: Bootstrap the test runner
- [x] Confirm `cargo test -p market-data-domain -p market-data-application
      -p market-data-infrastructure -p market-data-http` runs.
- [x] Record the command in `unit-test-instructions.md`.

### Step 3: Data model — Red
- [x] Domain tests: `Freshness` transitions (delayed before live), decimal validation, rejection
      (non-positive, > 300 s clock skew), dedupe-free observation model, `RefreshJob` enum.
- [x] Migration tests: six tables, `(asset_id, source_name, observed_at DESC)` and FX indexes,
      `(job, status, lease_until)` index, rate-bucket row, 10 MiB-free schema.

### Step 4: Data model — Green
- [x] Implement domain types and `0001_init.up.sql`/`.down.sql` with the `market_data` schema and
      `market_data_svc` role grants.

### Step 5: Data model — Refactor

### Step 6: Repositories — Red
- [x] Tests for observation insert/select (`DISTINCT ON`), FX read, run claim/renew/sweep, tier
      upsert, rate bucket, service-token lookup, retention prune (keeps newest).

### Step 7: Repositories — Green
- [x] Implement sqlx repositories behind ports; all decimal columns `NUMERIC`.

### Step 8: Repositories — Refactor

### Step 9: Business logic — Red
- [x] Application tests against fakes: priority selection, fallback, delayed/live/stale/unpriced,
      provenance, degraded C4 read, FX direct and USD cross, run claim/409/202, per-job leases,
      retries and circuit breaker, rate-limit deferral, rejected reading, counts, audit events,
      token allow-lists including `smoke-reader`.

### Step 10: Business logic — Green
- [x] Implement use cases, ports, and error types.

### Step 11: Business logic — Refactor

### Step 12: API — Red
- [x] HTTP tests: C6 `getPrices` (max 1,000, 422, 401/403, 503), `getFxRate` (404/422/401/403/503),
      C11 `price-refresh-top100`/`price-refresh-tail` (200/202/409), `/healthz`, `/readyz`,
      `/metrics`.

### Step 13: API — Green
- [x] Implement routers, DTOs, middleware, ports 8093 (API) and 8094 (worker).

### Step 14: API — Refactor

### Step 15–17: Frontend
- [x] Not applicable.

### Step 18: Environment/build configuration
- [x] Add `Dockerfile.api`, `Dockerfile.worker`, `Dockerfile.migrate`; add both services to
      `infra/swarm-stack.yml` with networks, limits, health checks and update config.

### Step 19: Documentation and traceability
- [x] Write `code-summary.md`, `source-manifest.json`, `traceability.json`.
