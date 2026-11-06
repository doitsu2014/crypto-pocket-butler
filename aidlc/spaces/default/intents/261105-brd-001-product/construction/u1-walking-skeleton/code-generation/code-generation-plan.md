# Code Generation Plan — u1-walking-skeleton

## Summary

- Builds: the walking skeleton — API Gateway, Asset Registry service, their shared Rust workspace, the SPA shell, migrations, and Swarm manifests
- Touches: `services/gateway/`, `services/asset-registry/`, `web/`, `infra/`, `Cargo.toml` at the repo root
- Tests: TDD unit and integration tests per layer, 5–8 per component, 80% line coverage per Rust crate

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

## Story-to-step traceability

| Story | Criteria | Plan steps |
|---|---|---|
| US1.2 | AC1.2.1–AC1.2.3 | 1–20 (asset resolution, gateway path, SPA first-value shell) |
| US9.2 | AC9.2.1–AC9.2.3 | 1–20 (registry schema, administration, audit emission) |

## Implementation steps

### Step 1 — Project structure and production configuration skeleton

- [x] Root `Cargo.toml` workspace with `services/gateway` and `services/asset-registry`
- [x] Each service: `domain`, `application`, `infrastructure`, `http` crates, dependencies inward only
- [x] Shared dev dependencies, rustfmt and clippy configuration
- [x] `.env.example` with non-secret names only

### Step 2 — Bootstrap the test runner

- [x] `cargo test` works for each service with a placeholder test
- [x] Record the exact unit-scoped command in `unit-test-instructions.md`
- [x] Add `web/` Vite + React + TypeScript shell with vitest configured

### Steps 3–5 — Data model / database behavior (Red → Green → Refactor)

- [x] Red: failing tests for `Asset`, `Chain`, `TokenContract`, `PriceSource`, `UnmappedObservation` invariants (BR1.1, BR1.2)
- [x] Green: migrations `0001_init.up.sql` / `0001_init.down.sql` and the domain types to pass
- [x] Refactor: tighten types, keep tests green

### Steps 6–8 — Repository / data access (Red → Green → Refactor)

- [x] Red: failing tests for resolve-by-contract, resolve-native-by-symbol, quarantine insert, registry version bump
- [x] Green: sqlx repositories behind port traits
- [x] Refactor: query and pool tuning

### Steps 9–11 — Business logic (Red → Green → Refactor)

- [x] Red: failing tests for BR1.1, BR1.3, BR2.1, BR2.2, BR3.1
- [x] Green: application use cases against port fakes
- [x] Refactor: clear duplication

### Steps 12–14 — API / endpoint (Red → Green → Refactor)

- [x] Red: failing tests for `GET /v1/assets/resolve`, RFC 7807 errors, gateway JWT rejection (BR4.1), deny-by-default routing (BR4.2), correlation ID (BR4.3)
- [x] Green: Axum routes, middleware, DTO mapping at the `http` boundary
- [x] Refactor: middleware extraction

### Steps 15–17 — Frontend behavior (Red → Green → Refactor)

- [x] Red: failing vitest tests for the SPA shell: shows a resolved asset, shows the network error, keeps the address on error
- [x] Green: minimal React shell calling the gateway
- [x] Refactor: component extraction

### Step 18 — Environment/build configuration

- [x] Dockerfile per service (multi-stage, non-root)
- [x] Swarm stack file with 2–4 replicas, health checks, rolling update with `failure_action: rollback`
- [x] `web/` static build output

### Step 19 — Documentation and traceability

- [x] `code-summary.md` (what was built, how to run it, known gaps)
- [x] `source-manifest.json` listing every application path
- [x] `traceability.json` mapping the acceptance criteria

## Build order note

Tests are written first at every layer and land with the code they cover. The exact unit-scoped
commands are in `unit-test-instructions.md`.
