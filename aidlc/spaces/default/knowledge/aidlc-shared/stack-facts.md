# Stack facts

The committed facts about what this repository is built with. Every one of these is verifiable in
the tree; none of them is a preference. When a plan or a design proposes a different tool, that is
a decision to surface, not a detail to swap quietly.

## Backend

- Rust 2021, one Cargo workspace at the root; MSRV 1.85 (`clippy.toml`).
- Axum + tower (HTTP), sqlx (Postgres, no ORM), `rust_decimal` (all quantities and values), tokio,
  chrono, serde at the boundary only.
- `rustfmt.toml`: edition 2021, `max_width = 100`, field-init shorthand.
- `clippy.toml`: MSRV 1.85, `too-many-arguments-threshold = 8`. Clippy runs with `-D warnings`, so
  a warning is a build failure.
- Every service: four crates (`domain`, `application`, `infrastructure`, `http`) plus a composition
  root. Dependencies point inward only; `bash scripts/check-boundaries.sh` fails the build when they
  do not.

## Frontend

- React 18.3 + TypeScript 5.7, Vite 6, TanStack Query 5, Recharts, `@supabase/supabase-js`,
  Tailwind CSS v4 mapped to the Parchment Ledger tokens.
- Two separate packages: `web/` (SPA) and `support-console/`. Each has its own `package.json`,
  lockfile and test setup.
- Vitest 3 + Testing Library + `jest-axe`; MSW 2 mocks the gateway in the support console.
- Prettier 3 (double quotes, `printWidth` 100, semicolons, trailing commas); ESLint 9 flat config
  with `@typescript-eslint/no-explicit-any` as an error.

## Toolchain

| Tool | Pinned to | Where it is pinned |
|---|---|---|
| Yarn | 4.18.1 | `packageManager` in root, `web/` and `support-console/` |
| Node | 24.22.1 | the build instructions for this intent |
| cargo-llvm-cov | 0.9.1 | coverage tool; needs the `llvm-tools` rustup component |
| PostgreSQL for tests | `supabase/postgres:15.8.1.060` | a throwaway container, because several Identity migrations need `pgsodium` |

`npm`, `npx` and `pnpm` and their lockfiles are forbidden; `yarn` is the only package manager.

## Data and infrastructure

- Supabase Postgres, self-hosted: Postgres with `pgsodium`, Auth, REST, Storage, Studio, Mailpit,
  Kong. One schema and one role per service.
- Secrets reach services from Supabase Vault (pgsodium) through `*_FILE` environment variables
  pointing at Docker secrets. A credential never appears in source, an env file, a log or an error.
- Docker Swarm for production (`infra/swarm-stack.yml`); `deployment/localhost` Compose for a
  one-machine run.

## Repository map

```
services/<name>/…        the twelve services and the gateway (see service-map.md)
web/                     the SPA
support-console/         the support console
infra/                   swarm stack, SQL seeds, Prometheus rules
deployment/localhost/    the developer Compose stack
design-system/           Parchment Ledger tokens and DESIGN.md
docs/brds/               the business requirements
docs/architecture/       the C4 model (workspace.dsl) and its README
docs/development/        the development guide behind the rules
scripts/                 check-boundaries.sh, smoke-identity.sh
aidlc/                   the AI-DLC workspace: memory, knowledge, intents, codekb
```