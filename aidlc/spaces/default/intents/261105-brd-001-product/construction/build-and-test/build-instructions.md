# Build Instructions — Crypto Pocket Butler

The workspace is a single repository holding the Rust services and the API gateway under
`services/`, the SPA under `web/`, the separately deployed support console under
`support-console/`, and the Swarm stack under `infra/`.

## Prerequisites

- Rust toolchain with the workspace's pinned edition (cargo, rustfmt, clippy).
- Yarn 4.18.1 pinned through corepack (`corepack enable`), for `web/` and `support-console/`.
- Node.js 20 or newer for the two Vite builds.
- For the adapter integration suites: a disposable PostgreSQL 15+ database with the `pgsodium`
  extension available, reached through `TEST_DATABASE_URL`. The suites run with `--ignored` (or
  `--include-ignored`) and create their own disposable schemas.
- Docker is only needed to provide that database locally; the build itself does not use it.

## Dependency installation

```
corepack enable
yarn --cwd web install
yarn --cwd support-console install
cargo fetch --manifest-path Cargo.toml
```

## Environment setup

- `TEST_DATABASE_URL` — a disposable Postgres database:
  `postgres://postgres:test@localhost:55432/postgres`.
- `VITE_GATEWAY_URL` — the API gateway base URL for the SPA build (same-origin `/v1` when unset).
- `VITE_SUPABASE_URL`, `VITE_SUPABASE_ANON_KEY` — Supabase Auth configuration for the SPA build.
- No secret is read from an env file; production credentials come from Supabase Vault.

## Build commands

```
cargo build --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
yarn --cwd web lint
yarn --cwd web build
yarn --cwd web size
yarn --cwd support-console build
```

The recorded Construction Verification Command is
`cargo test --workspace && cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && yarn --cwd web test && yarn --cwd web lint && yarn --cwd web build`.

## Build verification steps

- `cargo build --workspace` exits 0.
- `cargo fmt --all -- --check` reports no diff.
- `cargo clippy --workspace --all-targets -- -D warnings` exits 0.
- `yarn --cwd web build` produces `web/dist` and the PWA `dist/sw.js`.
- `yarn --cwd web size` reports the initial chunk set within 300 kB gzip and every route chunk
  within 150 kB.
- `yarn --cwd support-console build` writes the support bundle under `web/dist/support/`.

## Troubleshooting

- **`make_interval(days => $1) does not exist`** — the argument is `bigint`; PostgreSQL needs an
  explicit integer cast (`$1::int`). Fixed by the current Build and Test loop-back.
- **`schema "identity" does not exist`** — a repository or function call hardcodes the `identity.`
  schema; under a disposable test schema the migration creates the objects in the test schema and
  the search path resolves unqualified names.
- **`does not contain this feature: integration`** — only `market-data`, `ledger`, `portfolio`,
  `reporting` and `job-orchestration` infrastructure crates declare an `integration` feature;
  `account`, `notification` and `analytics` run their repository tests with
  `--test repositories -- --include-ignored` instead.
- **Bytea assertion counts `_sqlx_migrations.checksum`** — sqlx's own bookkeeping table is bytea;
  exclude it from schema-shape assertions.
- **A test that uses `Date.now()` with a fixed `NOW`** — build fixtures relative to the test's own
  clock.
