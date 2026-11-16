# Dependencies

No install, audit, or `cargo metadata` was run. Edges below are from manifests and imports read in the contributing scans and project knowledge; focused reporting-domain edges are explicitly scoped to that crate.

## External, Rust workspace

Declared once in the root `Cargo.toml` `[workspace.dependencies]` and available to members. Auth-relevant edges:

- `jsonwebtoken` — gateway JWT validation (`services/gateway/infrastructure/src/auth.rs`).
- `reqwest` — declared; call sites were not read. Session revocation goes through the `AuthAdmin` port, whose HTTP client was not opened.
- `sqlx` — declared; Identity session repository SQL was not opened.
- `axum` / `tower` — Identity HTTP router.

## Reporting domain crate

From `services/reporting/domain/Cargo.toml` and its analyzed source:

- Source-used external dependencies: `chrono`, `rust_decimal`, `thiserror`, and `uuid`.
- Declared but unused in all analyzed domain source: `serde` and `serde_json`. The repository boundary script explicitly forbids these dependencies in domain crates.
- No package-to-package dependency is declared in this crate manifest.

## External, SPA

From `web/package.json` dependencies: `@supabase/supabase-js` (auth client), `@tanstack/react-query`, `react-router-dom`, `recharts`, and `@walletconnect/ethereum-provider`. WalletConnect is declared; no call site was read.

Dev-only: Vitest, Testing Library, Playwright, ESLint, Prettier, MSW, Tailwind, and Vite. MSW's support-console use is stated in project knowledge and was not confirmed in that package's source.

## Internal

- Root workspace lists every service package and its four crates. Front ends are not Cargo members.
- `identity` (the composition package) depends on path crates `identity-domain`, `identity-application`, `identity-infrastructure`, and `identity-http`.
- `identity-application` session code depends on identity domain types and application ports. It does not import Axum or sqlx in the file that was read.
- Gateway infrastructure auth code depends on `gateway_application::ports` and `gateway_domain`.
- Gateway route names `asset-registry` and `identity` are service names, not Cargo dependencies.
- The SPA auth modules depend on the gateway client at the page layer: `SignInPage` calls `createGate` from the API client and `signIn` / `signUp` from the session module.

## Not established

Cross-service HTTP clients, schema grants, and gateway upstream base URLs were not read. `scripts/check-boundaries.sh` was not opened or executed in this reverse-engineering synthesis; its prohibition of serde and serde_json for domain crates is reported in the focused scan handoff.
