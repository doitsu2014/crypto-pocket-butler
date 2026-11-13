# Technology Stack — Crypto Pocket Butler

Versions for Rust crates are resolved from `Cargo.lock`; frontend versions are the ranges declared in each `package.json`.

## Languages

| Language | Edition / version | Where |
| --- | --- | --- |
| Rust | edition 2021, `rust-version = 1.85` (MSRV) | all backend services and the gateway |
| TypeScript | ^5.7.2 | `web/`, `support-console/` |
| SQL | PostgreSQL dialect | `services/*/migrations/`, `infra/sql/`, `deployment/localhost/init/` |
| Bash | — | `scripts/`, `tools/`, `deployment/localhost/scripts/` |
| YAML | — | `infra/swarm-stack.yml`, `deployment/localhost/docker-compose.yml`, `infra/prometheus/*.yml` |
| Markdown / Structurizr DSL | — | `docs/`, `design-system/` |

## Rust — runtime and web

| Crate | Declared | Resolved | Purpose |
| --- | --- | --- | --- |
| axum | 0.8 (macros) | 0.8.9 | HTTP routing, extractors, middleware |
| tokio | 1 (rt-multi-thread, macros, net, signal, sync, time) | 1.54.1 | async runtime |
| tower | 0.5 | 0.5.3 | middleware abstraction |
| tower-http | 0.6 (trace, limit, timeout) | 0.6 | HTTP middleware layers |
| http-body-util | 0.1 | 0.1 | body utilities |
| bytes | 1 | 1 | byte buffers |
| tokio-stream | 0.1 | 0.1 | streaming |
| serde | 1 (derive) | 1.0.229 | serialization at boundaries |
| serde_json | 1 | 1 | JSON |
| async-trait | 0.1 | 0.1 | async port traits |
| thiserror | 2 | 2 | typed domain errors |
| anyhow | 1 | 1 | binary/config error context |
| futures | 0.3 | 0.3 | concurrent scheduler work |

## Rust — persistence, security, observability

| Crate | Declared | Resolved | Purpose |
| --- | --- | --- | --- |
| sqlx | 0.8 (postgres, chrono, rust_decimal, uuid, json, migrate, macros, runtime-tokio-rustls) | 0.8.6 | Postgres access and migrations |
| reqwest | 0.12 (json, rustls-tls) | 0.12.28 | internal HTTP clients and JWKS |
| jsonwebtoken | 9 | 9.3.1 | JWT verification |
| base64 | 0.22 | 0.22 | encoding |
| chrono | 0.4 (serde) | 0.4.45 | timestamps |
| chrono-tz | 0.10 | 0.10 | timezone handling |
| rust_decimal | 1 (serde-with-str) | 1.43.0 | exact decimal money/quantity arithmetic |
| uuid | 1 (v4, serde) | 1.29.0 | identifiers |
| sha2 | 0.10 | 0.10 | subject hashes, content digests |
| hex | 0.4 | 0.4 | hash encoding |
| tracing | 0.1 | 0.1.44 | structured logs |
| tracing-subscriber | 0.3 (env-filter, json) | 0.3 | log subscriber |

## Rust — test-only

| Crate | Declared | Purpose |
| --- | --- | --- |
| rsa | 0.9 (pem) | signing test JWTs |
| rand | 0.8 | test data |

## Frontend — web and support console

| Package | Version | Where | Purpose |
| --- | --- | --- | --- |
| react / react-dom | 18.3.1 | both | UI |
| react-is | ^18.3.1 | web | Recharts interop |
| react-router-dom | ^6.30.1 | web | client routing |
| typescript | ^5.7.2 | both | types |
| vite | ^6.0.7 | both | build/dev server |
| @vitejs/plugin-react | ^4.3.4 | both | JSX transform |
| @tanstack/react-query | ^5.62.11 | both | server state |
| @supabase/supabase-js | ^2.47.10 | both | Auth in browser |
| recharts | ^3.10.1 | web | charts |
| @walletconnect/ethereum-provider | ^2.26.0 | web | read-only wallet sessions |
| tailwindcss / @tailwindcss/vite | ^4.0.0 | both | styling via tokens |
| vite-plugin-pwa | ^2.0.0 | web | PWA service worker |
| workbox-build / workbox-window | ^7.4.1 | web | precache/runtime caching |
| size-limit / @size-limit/preset-app | ^14.3.0 | web | bundle budget |
| vitest | ^3.0.0 | both | test runner |
| @testing-library/react | ^16.1.0 | both | component tests |
| @testing-library/user-event | ^14.5.2 | both | interaction tests |
| @testing-library/jest-dom | ^6.6.3 | both | DOM matchers |
| jest-axe / @types/jest-axe | ^10.0.0 / ^3.5.9 | both | accessibility assertions |
| jsdom | ^25.0.1 | both | DOM environment |
| msw | ^2.7.0 | support console | HTTP mocking |
| eslint | ^9.17.0 | both | linting |
| typescript-eslint | ^8.19.0 | both | TS lint rules |
| eslint-plugin-react-hooks | ^5.1.0 | both | hooks rules |
| prettier | ^3.4.2 | both | formatting |

**Package manager**: Yarn 4.18.1, pinned by `"packageManager": "yarn@4.18.1"` at the root and in each frontend package. `.yarnrc.yml` uses `nodeLinker: node-modules` and a global cache. npm/npx/pnpm and their lockfiles are forbidden by project rule. Note: project memory states "Yarn ≥ 6"; the repository pins 4.18.1 — an unresolved mismatch.

## Database Platform

| Component | Version / configuration | Source |
| --- | --- | --- |
| Supabase Postgres | 15 (`supabase/postgres`) with `pgsodium` | `deployment/localhost/docker-compose.yml` |
| Auth | GoTrue, RS256 JWTs, published JWKS | `deployment/localhost/README.md` |
| REST | PostgREST | local stack only |
| Storage | private buckets (`identity-exports`, reporting PDFs, audit exports) | `infra/sql/*`, service config |
| Vault | pgsodium secrets, per-subject keys | `infra/sql/identity-schema.sql` |
| Connection role model | one schema + one role per service (`*_app`, `*_readonly`, `*_migrator`, `*_retention`); NOLOGIN roles minted by Vault bootstrap | `infra/sql/*-schema.sql` |

## Deployment and Operations

| Technology | Where | Purpose |
| --- | --- | --- |
| Docker Swarm | `infra/swarm-stack.yml` | production orchestration, digest-pinned images, rolling updates |
| Docker Compose v2 | `deployment/localhost/docker-compose.yml` | local whole-system stack |
| nginx | `web/nginx.conf`, `deployment/localhost/nginx/default.conf` | static serving and `/v1` proxy |
| Prometheus + Alertmanager | `infra/prometheus/job-orchestration-rules.yml` | alert rules |
| Loki, Tempo, Grafana | modelled in `docs/architecture/workspace.dsl` | logs, traces, dashboards |
| Structurizr | `docs/architecture/workspace.dsl` | C4 model (generated `workspace.json` is gitignored) |
| Mailpit | local stack | auth email capture |
| Kong | local stack | Supabase API gateway |

## Tooling

| Tool | Config | Purpose |
| --- | --- | --- |
| rustfmt | `rustfmt.toml` (edition 2021, max_width 100) | Rust formatting |
| clippy | `clippy.toml` (msrv 1.85, too-many-arguments 8) | Rust linting |
| ESLint 9 (flat) | `web/eslint.config.js`, `support-console/eslint.config.js` | TS linting |
| Prettier 3 | `web/.prettierrc.json`, `support-console/.prettierrc.json` | TS formatting |
| Vitest | `web/vitest.config.ts`, `support-console/vitest.config.ts` | frontend tests |
| size-limit + `web/scripts/check-size.mjs` | `web/package.json` | bundle budgets (300 kB entry, 150 kB route, 150 kB styles, gzip) |
| boundary gate | `scripts/check-boundaries.sh` | rejects outward crate edges |

## Absent from the Stack

- **CI/CD**: no `.github/`, GitLab, Jenkins, CircleCI, Azure Pipelines or Buildkite configuration exists.
- **Coverage tooling**: no tarpaulin, llvm-cov, nextest or coverage threshold configuration, despite an 80% target in project memory.
- **Container/vulnerability scanning**: no scanner configuration in the repository.
- **OpenAPI/AsyncAPI**: no API specification document.
- **Message broker**: none by design (no queues).
