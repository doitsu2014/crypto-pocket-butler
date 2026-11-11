# CI/CD Pipeline — u1-user-registry

**Upstream:** `infrastructure-specification.md`, `monitoring-design.md`, `reliability-design.md`, `security-design.md`.

## Current state

The repository has **no CI platform** (recorded as an intention). The stages below run locally with the same commands a future CI will run; the intent-level verification command is authoritative.

## Stages

| # | Stage | Command | Gate |
|---|---|---|---|
| 1 | Format check | `cargo fmt --all -- --check` | Fails on any diff |
| 2 | Lint | `cargo clippy --workspace --all-targets -- -D warnings` | Warnings are errors |
| 3 | Unit and HTTP tests | `cargo test --workspace` | Any failure blocks |
| 4 | Adapter integration tests | `cargo test --workspace -- --ignored` with `TEST_DATABASE_URL` | Blocking |
| 5 | Boundary check | `bash scripts/check-boundaries.sh` | Outward edge blocks |
| 6 | Coverage | `cargo llvm-cov --workspace` | 80% per crate |
| 7 | Frontend checks | `yarn --cwd web test`, `yarn --cwd web lint`, `yarn --cwd web build`, `yarn --cwd support-console test`, `yarn --cwd support-console lint` | Any failure blocks |
| 8 | Migrate | one-shot `identity-migrate` applies `0004_user_registry_extensions.up.sql` before the API update | Failure stops the release |
| 9 | Deploy | Swarm rolling update, health check gating | Health failure rolls back |
| 10 | Smoke | `/healthz`, `/readyz`, one principal resolution | Failure rolls back |

## Rollback

Failed health check → `failure_action: rollback`; the release job applies `0004_user_registry_extensions.down.sql`; the previous image digest is restored; smoke re-runs.

## Secrets

No new secret for this unit. Existing Vault materialisation applies.

## Feature flags

None for this unit; the status values are data-driven.
