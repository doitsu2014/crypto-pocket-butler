# Code Generation Plan — u7-ledger

## Summary

- Builds: the Ledger service — append-only transactions with cross-source dedupe, FIFO cost basis,
  transfers, corrections, P&L with coverage, privacy export/anonymise, and the reconcile run.
- Touches: `services/ledger/` (four crates, two binaries, migrations, Dockerfiles) and
  `infra/swarm-stack.yml`.
- Tests: 120+ unit and integration tests across the four crates, TDD per layer, 80% line coverage per
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

Frontend behaviour is not applicable: Ledger is a backend service; the SPA reads through Portfolio
and C7.

## Story traceability

| Plan step | Stories / rules |
|---|---|
| 1–2 | service skeleton and runner |
| 3–5 | BR1.1, BR1.2, BR2.1 (append-only model, canonical keys, lots) |
| 6–8 | BR1.2, BR2.1, BR2.4, BR6.1 (repositories) |
| 9–11 | BR1.1–BR1.4, BR2.1–BR2.5, BR3.1–BR3.2, BR4.1, BR5.1–BR5.2, BR6.1 |
| 12–14 | C7 reads, corrections, recorded trades, privacy export/anonymise, C11 reconcile |
| 15–17 | not applicable |
| 18 | Dockerfiles, swarm stack, roles/secret inventory |
| 19 | summary, traceability, manifest |

## Steps

### Step 1: Project structure
- [x] `services/ledger` workspace: domain, application, infrastructure, http, `src/bin/{api,worker,migrate}.rs`; add workspace members.

### Step 2: Test runner
- [x] Confirm `cargo test -p ledger-domain -p ledger-application -p ledger-infrastructure -p ledger-http`; record it.

### Step 3–5: Data model
- [x] Red/Green/Refactor: transactions (canonical/external keys), corrections, transfer pairs, lots,
  snapshots, ledger_subjects; indexes; migrations up/down; three roles and grants.

### Step 6–8: Repositories
- [x] Red/Green/Refactor: insert/dedupe, list keyset, lot create/consume/move, snapshot read/write,
  subject anonymise, corrections, partitions-ready.

### Step 9–11: Business logic
- [x] Red/Green/Refactor: canonical dedupe, FIFO with fees, transfer basis move, corrections with
  per-field validation and reflow, P&L/coverage, recorded trade/user price, privacy export and
  anonymise, reconcile.

### Step 12–14: API
- [x] Red/Green/Refactor: C7 reads, correction/trade writes, privacy endpoints, C11 reconcile,
  `/healthz`, `/readyz`, `/metrics` on 8097/8098.

### Step 15–17: Frontend
- [x] Not applicable.

### Step 18: Environment/build
- [x] Dockerfiles, swarm-stack entries, secrets (`ledger_database_url`,
  `ledger_migrate_database_url`, `ledger_anonymiser_database_url`, `ledger_smoke_token`,
  `ledger_outbound_token`), health checks, update config.

### Step 19: Documentation
- [x] `code-summary.md`, `source-manifest.json`, `traceability.json`.
