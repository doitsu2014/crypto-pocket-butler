# Code Generation Plan — u12-audit

## Summary

- Builds: the Audit service as a four-crate Cargo workspace with two binaries — `audit-api` and `audit-retention` — its Supabase Postgres schema with three runtime roles plus a migrator, the append/query/export/support-session/privacy-deletion HTTP surface, and the retention sweep
- Touches: `services/audit/`, `infra/swarm-stack.yml` and `infra/sql/audit-schema.sql` at the workspace root, and `scripts/check-boundaries.sh`
- Tests: TDD per layer, 5–8 per component, 80% line coverage per Rust crate

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
| US7.3 | AC7.3.1–AC7.3.3 | 1–22 (audit side only: export recorded, no part in the user's own export) |
| US7.4 | AC7.4.1–AC7.4.3 | 1–22 (retention decision and legal-basis labelling) |
| US9.2 | AC9.2.1–AC9.2.3 | 1–22 (the audited change record) |
| US9.3 | AC9.3.1–AC9.3.3 | 1–22 (support session, expiry, per-read recording) |

## Implementation steps

### Step 1 — Project structure and production configuration skeleton

- [x] Done — five crates plus the `audit` package with `audit-api` and `audit-retention`.

- `services/audit/` Cargo workspace: `domain`, `application`, `infrastructure`, `http`, and a
  `bin` package producing `audit-api` and `audit-retention`
- Dependencies point inward only; `domain` imports no framework
- `.env.example` with non-secret names only
- Root `Cargo.toml` gains `services/audit` as a workspace member; `rustfmt.toml` and `clippy.toml`
  already cover it

### Step 2 — Bootstrap the test runner

- [x] Done — `cargo test -p audit` is the recorded unit-scoped command; 88 non-database tests pass.

- `cargo test --manifest-path services/audit/Cargo.toml` exits 0 on the skeleton
- Record the exact unit-scoped commands in `unit-test-instructions.md`

### Steps 3–5 — Data model (Red → Green → Refactor)

- [x] Done — domain entities plus `0001_init.up.sql`/`.down.sql`; 28 domain tests pass.

- Red: failing tests for `AuditEvent` required fields (BR1.2), secret-shape rejection (BR1.3),
  the context bound (BR1.4), `SupportAccessRecord` constraints (BR3.1), `RetentionRun` windows
  (BR4.1), `AuditExport` states
- Green: domain types plus `migrations/0001_init.up.sql` / `.down.sql` — partitioned
  `audit_events`, `append_idempotency`, the workflow-record tables, and the three runtime roles
  with column-scoped grants
- Refactor: tighten types; parse, don't validate

### Steps 6–8 — Repository / data access (Red → Green → Refactor)

- [x] Done — sqlx repositories behind the ports; the 10-test adapter suite passes against Postgres.

- Red: failing tests for append-with-idempotency (201 existing id, 409 changed payload), cursor
  query with a mandatory `recordedAt` range, session read, export cursor stream, retention batch
  delete and partition drop
- Green: sqlx repositories behind port traits
- Refactor: one prepared statement per shape; connection budgets as designed

### Steps 9–11 — Business logic (Red → Green → Refactor)

- [x] Done — append, query, export, support, retention, and privacy use cases; 34 tests pass.

- Red: failing tests in `application` against port fakes for BR1.1–BR1.4, BR2.1–BR2.2,
  BR3.1–BR3.3, BR4.1–BR4.3, BR5.1–BR5.2 — including the event class derived from sensitivity,
  the system-attribution allow-list, expiry evaluating as deny, and the export cap
- Green: use cases against the port traits
- Refactor: remove duplication; keep the audit write ahead of the answer

### Steps 12–14 — API / endpoint (Red → Green → Refactor)

- [x] Done — Axum routes, middleware, and DTO mapping; 22 HTTP tests pass.

- Red: failing tests for `POST /v1/events` (201/409/422/503), `GET /v1/events`,
  `POST /v1/exports`, `POST|GET /v1/support-sessions`, `POST /v1/privacy/deletions`, RFC 7807
  errors, service-token rejection, deny-by-default roles, correlation id propagation
- Green: Axum routes, middleware, DTO mapping at the `http` boundary
- Refactor: extract the auth and correlation middleware

### Steps 15–17 — Retention worker

- [x] Done — `audit-retention` supervisor, sweep decision, and privacy execution; covered in the application suite.

- Red: failing tests for the sweep's decision rule (drop an entire past-window partition; batch
  delete a partly in-window one; never delete a legally held row), the run record, and the
  failure path
- Green: the `audit-retention` binary and its supervisor loop
- Refactor: batch sizing and pause as constants in one place

### Step 18 — Environment/build configuration

- [x] Done — two Dockerfiles, `infra/sql/audit-schema.sql`, both stack services, and `scripts/check-boundaries.sh`.

- Two Dockerfiles (multi-stage, non-root, read-only root filesystem)
- `infra/sql/audit-schema.sql`; `infra/swarm-stack.yml` entries for both services with health
  checks, `failure_action: rollback`, and Docker-secret references
- `scripts/check-boundaries.sh` (the boundary gate the pipeline calls)

### Steps 19–22 — Documentation and traceability

- [x] Done — `code-summary.md`, `source-manifest.json`, and `traceability.json`.

- `code-summary.md`, `source-manifest.json`, `traceability.json`

## Build order note

Tests come first at every layer and land with the code they cover. The exact unit-scoped
commands are in `unit-test-instructions.md`.
