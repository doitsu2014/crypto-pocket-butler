# Code Generation Plan — u3-identity

## Summary

- Builds: the Identity service as a four-crate Cargo workspace plus a `bin` package producing `identity-api` and `identity-worker`, its `identity` Postgres schema and pgsodium protect/unprotect functions, the C3/C1 identity HTTP surface, the jurisdiction gate, and the export/deletion worker
- Touches: `services/identity/`, `infra/swarm-stack.yml` (two services, internal encrypted overlay, worker wake endpoint) and `infra/sql/identity-schema.sql`
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
| US1.1 | AC1.1.1–AC1.1.3 | 1–5, 9–17 (verification → active; the sign-up form is u2-web-app) |
| US3.5 | AC3.5.1–AC3.5.3 | 3–5, 9–11 (preferences stored and validated) |
| US7.1 | AC7.1.1–AC7.1.3 | 9–14 (MFA gate from the validated JWT) |
| US7.2 | AC7.2.1–AC7.2.3 | 6–17 (session view, revoke, warn) |
| US7.3 | AC7.3.1–AC7.3.3 | 6–17 (export request, worker, signed download) |
| US7.4 | AC7.4.1–AC7.4.3 | 3–11, 15–17 (deletion, anonymise, Auth delete, key destroy) |
| US7.5 | AC7.5.1–AC7.5.3 | 3–5, 9–11 (consent history) |
| US7.6 | AC7.6.1–AC7.6.3 | 9–14 (jurisdiction gate and signed gate token) |
| US9.3 | AC9.3.1–AC9.3.3 | 6–11 (support grant approval, expiry, audited read) |

## Implementation steps

### Step 1 — Project structure and production configuration skeleton

- [ ] `services/identity/` Cargo workspace: `domain`, `application`, `infrastructure`, `http`, and a `bin` package producing `identity-api` and `identity-worker`
- [ ] Dependencies point inward only; `domain` imports no framework
- [ ] `.env.example` with non-secret names only; root `Cargo.toml` gains `services/identity`

### Step 2 — Bootstrap the test runner

- [ ] `cargo test --manifest-path services/identity/Cargo.toml` exits 0 on the skeleton
- [ ] Record the exact unit-scoped commands in `unit-test-instructions.md`

### Steps 3–5 — Data model (Red → Green → Refactor)

- [ ] Red: failing tests for the `User` lifecycle, consent append-only history, support grant constraints, export/deletion states, `JurisdictionRule` seed, `ShreddingKey`, `DeviceSession`
- [ ] Green: domain types plus `migrations/0001_init.up.sql` / `.down.sql`, the `identity.protect` / `identity.unprotect` SECURITY DEFINER functions, and the service role grants
- [ ] Refactor: typed ids, parse-don't-validate

### Steps 6–8 — Repository / data access (Red → Green → Refactor)

- [ ] Red: failing adapter tests for principal lookup, consent newest-per-purpose, session upsert, grant claim and expiry, export/deletion lease claim and re-claim, `protect`/`unprotect`, Audit client
- [ ] Green: sqlx repositories behind port traits
- [ ] Refactor: one prepared statement per shape; pool setting 12 per replica

### Steps 9–11 — Business logic (Red → Green → Refactor)

- [ ] Red: failing `application` tests against port fakes for JIT provisioning, verified → active, MFA gate, ownership, consent, preferences, session listing/revoke, support approval/expiry/audit, export orchestration, deletion (anonymise, Auth delete, key destroy), jurisdiction gate
- [ ] Green: use cases against the port traits
- [ ] Refactor: remove duplication; keep the audit write ahead of the answer

### Steps 12–14 — API / endpoint (Red → Green / Refactor)

- [ ] Red: failing HTTP tests for every C3/C1 operation, JWT/JWKS rejection, RFC 7807, role and ownership denials, the pre-auth sign-up gate, the worker wake endpoint
- [ ] Green: Axum routes, middleware, DTO mapping at the `http` boundary
- [ ] Refactor: extract the auth, correlation, and error middleware

### Steps 15–17 — Worker (Red → Green → Refactor)

- [ ] Red: failing tests for export assembly and progress, partial-failure marking, deletion steps, lease expiry and re-claim, wake vs sweep, archive deletion after expiry
- [ ] Green: the `identity-worker` binary, wake endpoint, and sweep
- [ ] Refactor: batch sizing and lease constants in one place

### Step 18 — Environment/build configuration

- [ ] Two Dockerfiles (multi-stage, non-root, read-only root filesystem, digest-pinned bases)
- [ ] `infra/sql/identity-schema.sql`; `infra/swarm-stack.yml` entries for both services, the internal encrypted overlay, health checks, `failure_action: rollback`, and Docker-secret references
- [ ] `scripts/check-boundaries.sh` covers the new packages

### Step 19 — Documentation and traceability

- [ ] `code-summary.md`, `source-manifest.json`, `traceability.json`

## Build order note

Tests come first at every layer and land with the code they cover. There is no frontend behavior in
this unit: the SPA screens for US1.1, US3.5, US7.x, and US9.3 belong to u2-web-app, so steps 15–17
are the worker rather than a frontend layer. The exact unit-scoped commands are in
`unit-test-instructions.md`.
