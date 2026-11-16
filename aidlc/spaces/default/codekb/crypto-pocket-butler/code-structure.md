# Code Structure

## Layout actually read

| Path | What was read |
|---|---|
| `Cargo.toml` | Workspace members and shared dependency versions. Edition 2021, rust-version 1.85. |
| `package.json` | Root package name `crypto-pocket-butler`, Yarn `packageManager` 4.18.1. No application dependencies. |
| `web/package.json` | SPA scripts and dependency ranges. |
| `web/src/auth/` | `client.ts`, `storage.ts`, `session.ts`, `session.test.ts`. |
| `web/src/features/auth/SignInPage.tsx` | Email/password sign-in and sign-up UI. |
| `web/src/api/client.ts` | `GatewayClient` request path. The rest of the file was not fully read. |
| `services/gateway/domain/src/route.rs` | Deny-by-default `RouteTable`. |
| `services/gateway/infrastructure/src/auth.rs` | JWT/JWKS validator declarations. The whole file was not line-read. |
| `services/identity/Cargo.toml` | Package `identity`, bins `identity-api`, `identity-worker`, `identity-migrate`, deps on the four crates. |
| `services/identity/http/src/routes.rs` | Router groups. |
| `services/identity/http/src/middleware.rs` | `require_jwt` and nearby problem helpers. |
| `services/identity/application/src/sessions.rs` | `SessionService::list` and `revoke`. |
| `services/identity/tests/http.rs` | Scan handoff cites auth and session cases; not re-read here. |
| `services/identity/tests/authorisation_adapters.rs` | Scan handoff cites provider-role and audit adapter cases; not re-read here. |
| `docs/brds/BRD-003-authentication-and-session-management.md` | Requirements, rules, acceptance criteria, open questions. |
| `.gitignore` | Listed as read by the scan. Ignored build and generated trees were not opened. |
| `services/reporting/domain/Cargo.toml` | Reporting domain manifest and dependency declarations. |
| `services/reporting/domain/src/` | `audit.rs`, `brief.rs`, `compose.rs`, `error.rs`, `ids.rs`, `lib.rs`, and `service.rs` deeply read by the focused scan. |

Identity migration filenames `0001_init` through `0008_support_mine_index` (up and down) were inventoried. The SQL was not read. Do not treat schema shape as verified by this run.

## Patterns in the read code

- Rust modules state the business-rule id they serve.
- HTTP handlers stay in `identity-http`. The session use case depends on ports, not on Axum or sqlx.
- The SPA keeps auth I/O in `web/src/auth` and gateway I/O in `web/src/api/client.ts`.
- The reporting domain crate groups domain concepts into modules for audit, brief, composition, errors, identifiers, and service authorization. The manifest's `serde` and `serde_json` entries have no corresponding import or derive use in the deeply analyzed source.
- The reporting domain manifest has no package-to-package dependencies declared; its listed external used dependencies are chrono, rust_decimal, thiserror, and uuid.

## Skimmed, not classified file by file

`services/` other than the files above, most of `web/src/`, `support-console/src/`, `deployment/`, `infra/`, `supabase/`, `scripts/`, `docs/` other than BRD-003, and `design-system/`. The root workspace lists twelve service trees (account, analytics, asset-registry, audit, gateway, identity, market-data, ledger, notification, portfolio, reporting, job-orchestration), each with `domain`, `application`, `infrastructure`, and `http` members.

## Left unopened

`target/`, `node_modules/`, frontend `dist/`, Playwright artifacts, generated Structurizr output, and the AI-DLC trees except the named knowledge files. The reporting-domain focused scan reported no omitted paths within its focused directory.
