# Code Generation Plan — u2-web-app

## Summary

- Builds: the full SPA — sign-in/verification/onboarding, connect (exchange, wallet, WalletConnect,
  CSV import), portfolio dashboard and management, targets, intents, trades, activity, snapshots,
  briefs, notifications, settings/MFA/sessions, privacy and help — on the existing `web/` shell,
  with the C1 API client, in-memory session (PKCE verifier exception), i18n en/vi, Parchment
  Ledger tokens, PWA isolation and client telemetry.
- Touches: `web/` (routes, features, components, api, auth, i18n, pwa, telemetry, tests),
  `web/nginx.conf`, `web/Dockerfile`, `web/package.json` (size-limit, i18n fixture), and the web
  service in `infra/swarm-stack.yml` (read-only rootfs + tmpfs).
- Tests: Vitest + Testing Library per feature (happy path + two edge cases), axe checks, a
  Playwright-free test layer in jsdom (browser matrix job is recorded in the infra pipeline).

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

Data model, repository and business-logic layers are not applicable: the SPA renders backend
data; the frontend behaviour layer is the unit's testable surface, and the API client is its
integration boundary.

## Story traceability

| Plan step | Stories / rules |
|---|---|
| 1–2 | web shell, router, test runner |
| 3–5 | not applicable |
| 6–8 | API client, query hooks and error mapping (NFR9.4, BR7.1) |
| 9–11 | i18n, formatting rules, status model (BR2, BR4, BR6) |
| 12–14 | not applicable |
| 15–17 | every U2 journey (AC1.1–AC8.1.3) |
| 18 | nginx CSP/cache, Dockerfile build arg, size-limit, PWA, stack read-only |
| 19 | code summary, manifest, traceability |

## Steps

### Step 1: Project structure
- [x] `web/src/{app,auth,api,i18n,components,features,pwa,telemetry}`, router and providers; keep the existing styles.

### Step 2: Test runner
- [x] Confirm `yarn --cwd web test`; add size-limit + `yarn size`.

### Step 3–14: Backend layers
- [x] Not applicable (frontend-only unit).

### Step 15–17: Frontend behaviour
- [x] Red/Green/Refactor per journey: auth/verification, onboarding, connect (exchange, wallet, WalletConnect, CSV), portfolios and accounts, dashboard, targets, intents, trades, activity/P&L, snapshots, briefs, notifications, settings/MFA/sessions, privacy, help; shared Money/status/provenance components; axe and keyboard checks.

### Step 18: Environment/build
- [x] nginx CSP (WalletConnect + fonts + report-to), cache locations, gzip, stub_status; Dockerfile `VITE_GATEWAY_URL` build arg; PWA injectManifest with `shouldBypass`; stack read-only + tmpfs; Lighthouse/Playwright/axe CI entries.

### Step 19: Documentation
- [x] `code-summary.md`, `source-manifest.json`, `traceability.json`.
