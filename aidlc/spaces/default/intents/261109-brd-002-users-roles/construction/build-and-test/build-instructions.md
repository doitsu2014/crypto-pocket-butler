# Build Instructions — 261109-brd-002-users-roles

Assembly instructions for the whole project after all ten Units have been generated. Scope:
`classic`, ten Units (`u1`–`u10`), brownfield repository, Standard test strategy, TDD.

Commands are the canonical list in `aidlc/spaces/default/memory/project.md`; the per-package
`package.json` scripts and `docs/development/README.md` are the implementation detail behind it.

## 1. Prerequisites and dependency installation

| Tool | Version in use | What it builds |
|---|---|---|
| Rust toolchain (`cargo`, `rustc`) | 2021 edition, MSRV 1.85 (`clippy.toml`) | every Rust service, the gateway, the migrate binaries |
| `rustup` component `llvm-tools-aarch64-apple-darwin` | installed | `cargo llvm-cov` (the 80 % per-crate coverage gate) |
| `cargo-llvm-cov` | 0.9.1 | coverage report |
| Node | 24.22.1 | `web/` and `support-console/` build and test |
| Yarn | 4.18.1 through corepack (`packageManager: yarn@4.18.1` at the root and in both packages) | every JavaScript/TypeScript command — `npm`, `npx` and `pnpm` are forbidden |
| Docker | 29.9.2 | the disposable PostgreSQL the `#[ignore]` adapter and migration suites need |

Install the locked JavaScript dependencies once per checkout, in each package:

```bash
yarn --cwd web install --immutable
yarn --cwd support-console install --immutable
```

Rust dependencies are resolved from the workspace `Cargo.lock` by `cargo` itself; no separate
install step. The root `package.json` carries no scripts — it exists only to pin the Yarn version.

## 2. Environment setup

Nothing is required to build or to run the default (non-database) suites: no secret, no config
file, no running service. Two optional variables extend the run:

| Variable | Needed for | Notes |
|---|---|---|
| `TEST_DATABASE_URL` | the `#[ignore]`d adapter and migration suites — a **blocking** gate, not an opt-in | must point at a **disposable** PostgreSQL. Those suites create and drop roles, apply and reverse migrations. Never point it at a shared or production database. |
| `POSTGRES_PORT` / `POSTGRES_PASSWORD` | `deployment/localhost` only | developer convenience stack, not an environment tier and not needed by this stage |

The disposable database is a single throwaway container of the same image the project pins
(`supabase/postgres:15.8.1.060`); it carries `pgsodium`, which several Identity migrations and
adapter cases require. The 80 % coverage floor is read **after** the ignored suites run, or the
repository lines read as uncovered.

The following are deliberately **not** part of this build: Structurizr CLI (the C4 model is
validated in its own stage), a real browser engine (no Playwright/Chromium run exists in the
project), and any deployed environment (production is the only tier and no deployment is part of
this stage).

## 3. Build commands

```bash
# Rust — the whole workspace, every service, the gateway and the migrate binaries
cargo build --workspace

# SPA
yarn --cwd web build

# Support console
yarn --cwd support-console build
```

`cargo build --workspace` is the compile check for all Rust crates. Both frontend builds are
`tsc -b && vite build`, so they are the TypeScript type-check as well as the bundle.

## 4. Build verification

| Check | Command | Passing looks like |
|---|---|---|
| Rust compiles | `cargo build --workspace` | exit 0, `Finished` |
| Rust formatting | `cargo fmt --all -- --check` | exit 0, no diff |
| Rust lints | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, `No issues found` (warnings are errors) |
| Clean Architecture boundaries | `bash scripts/check-boundaries.sh` | exit 0 — parses `cargo metadata --no-deps` and fails on any outward crate edge |
| SPA bundles | `yarn --cwd web build` | exit 0, `route-*.js` chunks emitted for every lazy route |
| Support console bundles | `yarn --cwd support-console build` | exit 0, lazy section chunk emitted |
| SPA size budgets | `yarn --cwd web size` | initial entry ≤ 300 kB gzip, route chunk ≤ 150 kB, stylesheet ≤ 150 kB |

A build is assembly-verified when all seven pass together. A single crate failing to compile is
reported with the crate and the first error, never with a truncated log.

## 5. Troubleshooting common build issues

| Symptom | Cause | Action |
|---|---|---|
| `error: failed to find llvm-tools-preview` | the `llvm-tools` component is missing from the active toolchain | `rustup component add llvm-tools-preview`, then re-run `cargo llvm-cov` |
| `reporting-domain must not import framework crates: ['serde', 'serde_json']` | pre-existing boundary violation in `services/reporting`, outside this intent's ten Units | build-and-test finding; fixing it is a separate scoped change (no test was relaxed to hide it) |
| `yarn --cwd <pkg> format:check` reports many unformatted files that this intent did not write | the committed tree is already dirty under the locked Prettier version | reported as a pre-existing red; reformatting the whole package is an unrelated change and is not swept into this intent |
| `schema "pgsodium" does not exist` or a role-create failure in the ignored suites | the disposable database is not a `supabase/postgres` image, or the connection is not a superuser | recreate the container from `supabase/postgres:15.8.1.060` and point `TEST_DATABASE_URL` at it |
| an adapter suite deadlocks or a migration hangs | two suites migrating the same schema at once | run `--test-threads=1` for the migration-bearing suites (the U4 migration applies statement by statement) |
| a stale `sqlx` macro build hides a newly added migration | cargo cache | `cargo clean -p identity-infrastructure`, then rebuild |
| `Expected signal to be an AbortSignal` in a console test | Node's `fetch` rejecting a jsdom signal | already handled by the console test setup shim; if it reappears, check `src/test/setup.ts` is loaded |
| `EADDRINUSE` on 5174 | a dev server is already running | stop it; the test suite binds no port |

## 6. What this build does not do

- It does not apply migrations to any shared database, deploy to the Swarm, or run a smoke test
  against a live stack.
- It does not measure TypeScript coverage (no threshold is set in team memory) or run a real
  browser engine.
- It does not validate the Structurizr model (`docs/architecture/`) — no CLI is installed here.