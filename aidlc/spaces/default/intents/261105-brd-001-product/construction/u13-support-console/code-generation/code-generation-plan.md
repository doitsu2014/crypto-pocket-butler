# Code Generation Plan — u13-support-console

## Summary

- Builds: the support console SPA — role gate, grant request/approval/revocation, grant-scoped
  account search and metadata, ticket list, incident console, accessible by keyboard.
- Touches: `support-console/` (React 18 + TS 5 + Vite + TanStack Query + Tailwind) and the web image
  build that serves `/support`.
- Tests: 45+ component and integration tests with Vitest + RTL and an axe check.

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

The unit is frontend-only: the API and data layers are C2 on U1's support API. The TDD Red/Green/
Refactor steps apply to the frontend layer, the API client module, and the client-side state logic;
data-model and server layers are not applicable.

## Story traceability

| Plan step | Stories / rules |
|---|---|
| 1–2 | app skeleton and test runner |
| 3–8 | client-side model/state for grants, metadata, incidents |
| 9–11 | BR1.1–BR1.4, BR2.1, BR3.1–BR3.3, BR4.1 |
| 12–14 | C2 client module and error handling |
| 15–17 | screens: role gate, grant request/approval, search, metadata, tickets, incidents, help |
| 18 | web image `/support` packaging, CSP |
| 19 | summary, traceability, manifest |

## Steps

### Step 1: Project structure
- [ ] `support-console/` Vite app with `src/{api,components,screens,hooks}`; package.json with Yarn.

### Step 2: Test runner
- [ ] Vitest + RTL + axe; record `yarn --cwd support-console test`.

### Step 3–5: Client model
- [ ] Failing tests then implementation for grant status/expiry, metadata shape, incident shape.

### Step 6–8: API client
- [ ] Red/Green/Refactor for `src/api/client.ts`: C2 calls, bearer, RFC 7807 mapping, 401/403).

### Step 9–11: Business logic
- [ ] Red/Green/Refactor: role gate, grant validation (reason ≥20, 1–480 min, approver differs),
      grant-scoped search, expiry handling, incident severity/validation.

### Step 12–14: API/endpoint layer
- [ ] Red/Green/Refactor: query hooks, cache clearing on sign-out/403/expiry, in-memory session.

### Step 15–17: Frontend behaviour
- [ ] Red/Green/Refactor: every screen in `frontend-components.md`, keyboard flow, ARIA live
      regions, axe assertions, status label+shape.

### Step 18: Environment/build
- [ ] Build output to `web/dist/support`; exclude from the PWA service-worker scope; CSP applied by
      the gateway.

### Step 19: Documentation
- [ ] `code-summary.md`, `source-manifest.json`, `traceability.json`.
