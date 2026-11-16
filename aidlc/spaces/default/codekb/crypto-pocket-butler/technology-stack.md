# Technology Stack

Versions are manifest declarations from files read in the contributing scans, not lockfile resolutions and not a build. Tests were not run.

## Rust workspace

From root `Cargo.toml`:

- Edition 2021, `rust-version` 1.85, resolver 2, package version 0.1.0, `publish = false`.
- HTTP: axum 0.8, tower 0.5, tower-http 0.6, http-body-util 0.1.
- Data: sqlx 0.8 (Postgres, chrono, rust_decimal, uuid, json, migrate; default features off), rust_decimal 1, chrono 0.4, chrono-tz 0.10, uuid 1.
- Runtime: tokio 1, tracing 0.1, tracing-subscriber 0.3.
- Auth/HTTP client: jsonwebtoken 9, reqwest 0.12 (json, rustls-tls), base64 0.22.
- Other shared crates include async-trait 0.1, thiserror 2, anyhow 1, serde 1, serde_json 1, sha2 0.10, hex 0.4, futures 0.3, bytes 1, tokio-stream 0.1.
- Test helpers in the workspace: rsa 0.9, rand 0.8.

The focused `services/reporting/domain/Cargo.toml` scan found dependencies on workspace `chrono`, `rust_decimal`, `thiserror`, and `uuid`, and declarations for `serde` and `serde_json`. The scanned reporting domain source uses the former set but has no use of either serialization crate. The mechanical domain boundary check forbids serde and serde_json in domain crate manifests.

`rustfmt.toml` and `clippy.toml` were named by project knowledge but not opened in this run.

## JavaScript

- Yarn 4.18.1 (`package.json`, `web/package.json` `packageManager`).
- SPA `crypto-pocket-butler-web` 0.1.0, `type: module`.
- Runtime ranges in `web/package.json`: React and React DOM 18.3.1, `@supabase/supabase-js` ^2.47.10, `@tanstack/react-query` ^5.62.11, `react-router-dom` ^6.30.1, `recharts` ^3.10.1, `@walletconnect/ethereum-provider` ^2.26.0.
- Tooling ranges: TypeScript ^5.7.2, Vite ^6.0.7, Tailwind CSS ^4.0.0, Vitest ^3.0.0, Playwright ^1.65.0, ESLint ^9.17.0, Prettier ^3.4.2, MSW ^2.7.0, jest-axe ^10.0.0.

`support-console/package.json` was not read. Project knowledge identifies it as a second Yarn package; its dependency set is not freshly verified here.

## Data and deploy (knowledge, not re-read in infra files)

Project knowledge describes Supabase Postgres with pgsodium, Docker Swarm (`infra/swarm-stack.yml`), and a local Compose stack (`deployment/localhost`). Those YAML files were not opened in this run.

## Identity binaries

`identity-api`, `identity-worker`, and `identity-migrate` (`services/identity/Cargo.toml`).
