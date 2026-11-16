# Code Quality Assessment

No test, lint, coverage, boundary, or build command was run. Nothing below is a measured score.

## Tests that exist (not executed)

- `web/src/auth/session.test.ts` is present. The developer scan says it covers session loading, provider sign-out, unconfigured auth, and signup-gate metadata. This synthesis did not re-read that file line by line.
- `services/identity/tests/http.rs` is present. The scan cites missing/invalid JWT rejection and correlation, and own-session list/revoke cases.
- `services/identity/tests/authorisation_adapters.rs` is present. The scan cites provider-role and audit adapter cases.
- Reporting-domain tests use Rust's built-in `#[cfg(test)]` and `#[test]` harness embedded in the analyzed source modules; no separate test directory was found within the focused package.
- Other service test trees and support-console tests were only inventoried.
- `web/package.json` scripts include Vitest, Playwright, ESLint, and Prettier checks. No coverage threshold appears in that manifest.
- Project knowledge names `cargo-llvm-cov` 0.9.1 as the Rust coverage tool. No coverage config was confirmed in the manifests opened.

## Static checks named but not run

Project knowledge and the scan name `rustfmt.toml`, `clippy.toml`, `yarn lint`, `yarn format:check`, and `bash scripts/check-boundaries.sh`. Those commands were not run. The formatter and clippy files were not opened. The focused scan reports that the boundary script forbids `serde` and `serde_json` in domain crate manifests.

## CI

The scan found no `.github/workflows/*` paths in its glob. No pipeline file was read. Deployment files were inventoried only: `infra/swarm-stack.yml`, `deployment/localhost/docker-compose.yml`.

## Documentation

BRD-003 is specific (requirement ids, rules, acceptance criteria, five open questions). Rust modules that were read carry purpose comments and rule ids. `problem-details.md` documents a shared refusal shape; project knowledge says the shape is duplicated per service on purpose.

## Debt visible in the read files

- Authentication policy is still open in the BRD (token lifetimes, IdP logout, outage fallback, phone OTP, dual loss recovery).
- PKCE verifier persistence is an intentional exception and should stay limited to the verifier key.
- `SignInPage` is a partial auth surface: email/password only, fixed jurisdiction and age flags.
- Identity maps provider unavailability to 503 and copies `detail` into the body (`middleware.rs`).
- Gateway allowlist and Identity session routes are out of step in the files read.
- Migration SQL, session repository, and Auth admin adapter were not read, so revocation persistence is unverified.
- `services/reporting/domain/Cargo.toml` declares `serde` and `serde_json`, although none of the analyzed domain source uses them; the boundary script forbids them for domain crates. The focused scan did not run the script.
- Existing shared CodeKB prose outside the focused reporting area is retained; the prior verified scope is unioned with the focused scan in the timestamp artifact.
