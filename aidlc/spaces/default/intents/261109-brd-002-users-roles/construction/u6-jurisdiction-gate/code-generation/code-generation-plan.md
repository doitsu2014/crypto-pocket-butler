# Code Generation Plan — u6-jurisdiction-gate

**Unit:** `u6-jurisdiction-gate` (U6) · **Scope:** classic · **Depth:** Standard · **Test strategy:** Standard
**Source of truth:** `functional-spec.md`, `rules.md`, `entities.md`, `contract-summary.md` (C8), `nfr-design/*`, `infrastructure-specification.md`.

## Scope

Add the Jurisdiction Gate to the Identity service: two tables and their migration pair, a domain decision model, application use cases (`CheckJurisdiction`, `PublishList`), ports for the provider/list/audit/clock, a Postgres repository, an HTTP client with circuit breaker, audit client, and the two routes. No new service, no new Postgres role, no new message transport.

## Design decisions this plan settles (from the stage reviews)

| Open point | Resolution in this build |
|---|---|
| Migration number | Identity holds `0001_init` and `0002_contact`; this unit uses **`0003_jurisdiction_list`** (`0003_jurisdiction_list.up.sql` / `.down.sql`). The three Identity units take 0003/0004/0005; this unit takes 0003 first because it is built first. |
| Publish role | Publish writes with the existing **`identity_app`** role (one schema, one role per service). Authorisation is the admin JWT role, checked in the application layer; no second Postgres role and no extra Docker secret. |
| Client IP trust | The gate accepts only a value the gateway derived. This build adds a **gateway change**: strip any inbound `x-forwarded-for` at the edge and set `x-client-ip` from the observed peer (or the trusted edge header), then U1 passes that value in the C8 body. A client-supplied header is ignored. |
| Readiness | `/healthz` stays the Swarm health check. Provider/list state appears on `/readyz` and in metrics but does **not** flip Swarm readiness (so a provider outage cannot fail a rolling update or remove every replica). |
| Provider choice | The provider is behind a port; the HTTP adapter is configured by `JURISDICTION_PROVIDER_URL` and `JURISDICTION_PROVIDER_KEY_FILE`. SLA and quota are targets pending provider selection. |

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

### Step 1 — Project structure and configuration skeleton
- [x] Create the module skeleton in `services/identity`: `domain/src/jurisdiction.rs` (value types `CountryCode`, `JurisdictionStatus`, `RestrictionSource`, `Decision`), `application/src/jurisdiction.rs` (use cases and ports), `infrastructure/src/repositories/jurisdiction.rs`, `infrastructure/src/clients/geo_locator.rs`, `http/src/handlers.rs` additions.
- *Trace:* US6.3, US6.4 (structure only).

### Step 2 — Verify the runner
- [x] Verify `cargo test -p identity-domain` runs; record the exact unit-scoped commands in `unit-test-instructions.md`.
- *Trace:* Testing Contract runner step.

### Step 3 — Data model / database — Red
- [x] Failing tests for `CountryCode` parsing (valid, lowercase, wrong length, non-ASCII).
- *Trace:* BR1.4, AC6.3.1.

### Step 4 — Data model / database — Green
- [x] Implement `CountryCode` validation (`^[A-Z]{2}$`).
- *Trace:* BR1.4.

### Step 5 — Data model / database — Refactor
- [x] Refactor with tests green; no behavior change.

### Step 6 — Repository / data access — Red
- [x] `#[ignore]` integration tests against a disposable Postgres: create/read a list version with its rules; checksum mismatch is refused; previous version stays readable after a new publish.
- *Trace:* BR1.6, BR1.10, W3.

### Step 7 — Repository / data access — Green
- [x] Add migration pair `0003_jurisdiction_list.{up,down}.sql` creating `jurisdiction_list_versions` and `jurisdiction_rules` (PK `(version, country_code)`, FK to version, checksum column); implement `PgJurisdictionListRepository` with the `identity_app` role; publish is a transaction.
- *Trace:* NFR19.2, NFR23.1, W3.

### Step 8 — Repository / data access — Refactor
- [x] Refactor with tests green.

### Step 9 — Business logic — Red
- [x] Failing application tests against port fakes: self-declared restricted denies; IP restricted denies; both allowed allows; stricter signal wins; absent country defaults to **allow only if present in the list**, otherwise deny (fail closed); provider unavailable → `ServiceUnavailable`; continued use on provider failure → `ServiceUnavailable`; every decision emits an audit event; audit failure → `ServiceUnavailable`.
- *Trace:* BR1.1–BR1.5, BR1.7–BR1.9, NFR23.4, NFR1.4.

### Step 10 — Business logic — Green
- [x] Implement `CheckJurisdiction` and `PublishList` in `application` behind `GeoLocator`, `ListRepository`, `AuditSink`, `Clock` ports; enforce the 800 ms/one-retry/1.5 s provider budget and the 300 ms/one-retry audit budget with an idempotency key.
- *Trace:* NFR2.2–NFR2.4, NFR1.7.

### Step 11 — Business logic — Refactor
- [x] Refactor with tests green.

### Step 12 — API / endpoint — Red
- [x] Failing HTTP tests over the Axum router: `POST /internal/jurisdiction/check` (200 allowed/denied with `listVersion`, 422 invalid country, 503 provider/audit failure); `POST /internal/jurisdiction/publish` (admin JWT required, 403 otherwise, 200 on success).
- *Trace:* C8, NFR10.1, NFR23.2.

### Step 13 — API / endpoint — Green
- [x] Implement handlers, DTOs mapped at the boundary, RFC 7807 problems, metrics, and the gateway route declarations; reject a request whose IP did not come from the trusted caller.
- *Trace:* C8, NFR23.3.

### Step 14 — API / endpoint — Refactor
- [x] Refactor with tests green.

### Step 15 — Frontend behavior
- [x] Not applicable to this backend unit; no frontend files are produced. The sign-up UI belongs to U8.

### Step 16 — Gateway client-IP change
- [x] Modify the gateway middleware: strip inbound `x-forwarded-for`, derive the client address from the observed peer (or the trusted edge header), set `x-client-ip`, and pass it to U1 (the C8 body). Add tests.
- *Trace:* NFR23.3.

### Step 17 — Environment / build configuration
- [x] Add the provider URL/key Docker secret references to the Identity service config (`JURISDICTION_PROVIDER_URL`, `JURISDICTION_PROVIDER_KEY_FILE`); update `infra/swarm-stack.yml` only for the two secrets; no replica or resource change.
- *Trace:* infrastructure-specification.

### Step 18 — Documentation and traceability
- [x] Module `//!` headers citing BR ids; write `code-summary.md`; write `source-manifest.json`; update `traceability.json`.
- *Trace:* all.

## Story-to-step traceability

| Story | Steps |
|---|---|
| US6.3 | 1, 3–14, 16 |
| US6.4 | 1, 6–10, 13 |

## Definition of done

All steps checked, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and the `#[ignore]` adapter suites against a disposable Postgres all pass; the 80% per-crate coverage target is met; the gateway change is tested.
