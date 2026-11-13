# Code Quality Assessment — Crypto Pocket Butler

## Summary

| Dimension | Rating | Basis |
| --- | --- | --- |
| Naming clarity | good | Domain types and use cases are named for the business concept; module headers cite BR ids |
| Function size / cohesion | good | Use cases are small, single-purpose structs; repositories are one per aggregate |
| Error handling | good | Typed `DomainError` per service, `AppError` mapping, RFC 7807 at the boundary; no silent catches found |
| Test presence | good in volume, unmeasured in coverage | 71 Rust integration test files, 50 frontend test files; no coverage tool or threshold wired |
| Duplication | fair | Deliberate per-service duplication of `AppError`, `Role`, `ProblemDetails`; some repetition is by design, some is boilerplate |
| Dead code / markers | good | 0 `TODO`/`FIXME`/`HACK`/`XXX` markers in Rust or TypeScript |
| Automation | poor | no CI, no coverage job, no scanner |

## Test Coverage

### Test inventory

| Layer | Count | Location | Framework |
| --- | --- | --- | --- |
| Rust integration test files | 71 | `services/<svc>/tests/*.rs` and `services/<svc>/{domain,application,http,infrastructure}/tests/*.rs` | built-in `#[test]` / `#[tokio::test]` |
| Rust port fakes | per service | `tests/fakes.rs`, `tests/common/fakes.rs` | hand-written |
| Frontend test files | 50 (30+ web, 20+ support console) | `web/src/**/*.test.{ts,tsx}`, `support-console/src/**/*.test.{ts,tsx}` | Vitest 3 |
| Frontend test infrastructure | 4 web, 3 support | `web/src/test/{render,gateway,axe,setup}.ts(x)`, `support-console/src/test/{fixtures,server,setup}.ts(x)` | Testing Library, jest-axe, MSW |

### What is covered

- **Domain rules** — each service has `domain/tests/domain.rs` (or service-level `tests/domain.rs`) exercising state machines, validation and business rules.
- **Application use cases** — `application/tests/application.rs` (or `tests/application.rs`) with port fakes, covering happy path and error/edge cases.
- **HTTP boundary** — `http/tests/http.rs` (or service-level `tests/http.rs`) exercising routers, middleware, status codes and problem bodies.
- **Adapters** — `infrastructure/tests/repositories.rs` against a real Postgres schema, and provider-adapter suites (`account/infrastructure/tests/providers.rs`, `portfolio/infrastructure/tests/adapters.rs`, `market-data/infrastructure/tests/migrations.rs`).
- **Workers and scheduler** — `identity/tests/worker.rs`, `job-orchestration/application/tests/behaviour.rs`, smoke tests on asset-registry, audit, gateway, ledger, notification, portfolio, reporting.
- **Frontend** — component render/interaction tests, accessibility assertions with jest-axe, API-client and query-client tests, i18n, PWA bypass, support-console grant/incident/ticket domains.

### Coverage gaps

- **No coverage measurement**: there is no tarpaulin/llvm-cov config, no coverage script, no CI job, and no coverage report in the repository. The 80%-per-crate target in project memory is **not enforced by any artifact**.
- **Database suites are opt-in**: adapter tests require `TEST_DATABASE_URL` and are `#[ignore]`d by default (e.g. `services/identity/tests/common/mod.rs:1-4`). Nothing in the repository runs them automatically.
- **No end-to-end suite**: no Playwright/Cypress journey test; frontend tests mock the gateway.
- **No contract tests** between services; cross-service contracts are asserted only through hand-written fakes.
- **Unimplemented areas have no tests**: Billing/entitlements have neither code nor tests.

## Linting and Formatting

| Tool | Configuration | Scope |
| --- | --- | --- |
| rustfmt | `rustfmt.toml` — edition 2021, max_width 100, field-init shorthand | all Rust |
| clippy | `clippy.toml` — MSRV 1.85, too-many-arguments threshold 8 | all Rust; the documented command is `cargo clippy --workspace --all-targets -- -D warnings` |
| boundary gate | `scripts/check-boundaries.sh` | workspace crate edges |
| ESLint 9 | `web/eslint.config.js`, `support-console/eslint.config.js` — recommended + typescript-eslint + react-hooks, `no-explicit-any` = error | both frontends |
| Prettier 3 | `web/.prettierrc.json`, `support-console/.prettierrc.json` | both frontends |
| TypeScript | `tsc -b` in each `build` script | both frontends |

The documented commands in `docs/development/README.md` and project memory are:

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bash scripts/check-boundaries.sh
yarn lint && yarn test && yarn build   # per frontend package
```

## CI/CD

**There is no CI/CD pipeline in the repository.** There is no `.github/` directory and no GitLab, Jenkins, CircleCI, Azure Pipelines or Buildkite configuration. The quality gates described in `docs/development/README.md:39-42` ("CI checks crate boundaries…", "a failing check fails the build") are therefore **manual and unenforced**. Deployment artifacts exist (`infra/swarm-stack.yml`, per-service Dockerfiles) and assume a pipeline that builds and pins image digests, but that pipeline is not in this repository.

Consequence: the merge rules (fmt, clippy `-D warnings`, tests, boundary check, 80% coverage, dependency/image scanning) depend on a human running them.

## Documentation Quality

Strong, and unusually so for the code volume:

- `docs/brds/` — 21 BRDs with requirement ids (`FR-USR-*`, `BR-USR-*`, `AC-USR-*`, `NFR-*`), acceptance criteria, open questions, traceability and decisions.
- `docs/architecture/` — Structurizr DSL model of record, a README explaining boundary, containers, data ownership, communication rules, decisions and costs, and 10 views.
- `docs/development/` — stack, Clean Architecture rules and enforcement, testing posture.
- `design-system/DESIGN.md` — tokens, colour roles, contrast measurements and usage rules.
- Code — module-level `//!` headers explain the purpose and cite BR ids; public items carry `///` docs with `# Errors` where relevant. Example: `services/identity/domain/src/user.rs:1-5`, `services/identity/infrastructure/src/auth.rs:1-9`.

Weaknesses: the C4 model and README describe containers and behaviours that do not exist (Billing, gateway principal resolution, entitlement checks, non-registry routing), so documentation overstates implemented state. `docs/project-management/` contains only a short README; its `backlogs/` and `board/` folders are empty.

## Technical Debt

Ordered by impact on downstream work.

| # | Signal | Evidence | Impact |
| --- | --- | --- | --- |
| 1 | Billing service, entitlements and quotas absent | no `services/billing/`; 0 non-test `entitlement`/`quota` hits; C4 still lists the container (`workspace.dsl:54`) | BRD-016 cannot be implemented or verified; gateway narrative is false |
| 2 | Gateway routes only Asset Registry | `services/gateway/domain/src/route.rs:68-90` | SPA/console cannot reach Identity, Account, Portfolio, etc. through `/v1` |
| 3 | Gateway has no principal resolution or entitlement check | `services/gateway/application/src/authorize.rs` validates JWT and `requires_admin` only; no Identity client in `services/gateway/infrastructure/` | Architecture §6 is aspirational |
| 4 | No CI pipeline; documented gates unenforced | no `.github/` or other CI config | Merge discipline rests on humans; regressions not caught |
| 5 | No coverage tooling | no tarpaulin/llvm-cov/nextest config | 80% target unverifiable |
| 6 | Database tests ignored by default | `#[ignore]` + `TEST_DATABASE_URL` requirement | Adapter regressions only caught when run manually |
| 7 | User status enum narrower than BRD-002 | `users.status` is `pending|active|deleted` (`0001_init.up.sql:66`); BRD-002 FR-USR-20 declares `active|disabled|pending_deletion|deleted` | Requirement/implementation mismatch for disable and pending-deletion |
| 8 | Inconsistent service binary layout | `src/main.rs` (asset-registry, gateway) vs `src/bin/{api,worker,migrate}.rs` vs `bin/api.rs` only | No single runnable-unit convention for operators and pipelines |
| 9 | In-memory gateway rate limiting | `services/gateway/http/src/rate_limit.rs:1-6` | Per-replica limits; documented per-user/per-region dimensions absent |
| 10 | Yarn version mismatch | `packageManager: yarn@4.18.1` vs project memory "Yarn ≥ 6" | Toolchain rule is ambiguous |
| 11 | Docs overstate implementation | C4 container count 16 vs 12 service packages; entitlement claims | Downstream stages must verify against code, not docs |
| 12 | Local stack gaps | `deployment/localhost/README.md` "Known gaps" — registry-only routing, Mailpit incompatibility, no market-data provider, RPC endpoints | Local end-to-end validation is partial |

### Debt that is *not* present

- No hardcoded credentials or API keys in source; credentials are read from secret files (`services/*/src/config.rs`).
- Small `unwrap()` surface: 7 occurrences outside `tests/` directories across the workspace, and they are `Mutex::lock().unwrap()` in `market-data` metrics/refresh or in-file `#[cfg(test)]` modules; no panic-prone request-path parsing was found in the sampled code.
- Every `.up.sql` has a `.down.sql` (13/13).
- No message queue, no cross-schema grants, no RLS in services.
- No committed generated Structurizr artefacts (`workspace.json` and `.structurizr/` are gitignored).
