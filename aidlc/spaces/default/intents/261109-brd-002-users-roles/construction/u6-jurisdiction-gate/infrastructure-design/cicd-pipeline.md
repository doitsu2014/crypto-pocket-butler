# CI/CD Pipeline — u6-jurisdiction-gate

**Upstream:** `infrastructure-specification.md`, `monitoring-design.md`, `reliability-design.md`, `security-design.md`.

## Current state

The repository has **no CI platform** (the team recorded this as an intention, not an implemented gate). Until one is chosen, the pipeline below is executed locally, by the same commands the future CI will run. The intent-level verification command recorded at Delivery Planning is authoritative.

## Stages

| # | Stage | Command / action | Gate |
|---|---|---|---|
| 1 | Format check | `cargo fmt --all -- --check` | Fails on any diff |
| 2 | Lint | `cargo clippy --workspace --all-targets -- -D warnings` | Warnings are errors |
| 3 | Unit and HTTP tests | `cargo test --workspace` | Any failure blocks |
| 4 | Integration tests (adapter) | Run with a disposable Postgres via `TEST_DATABASE_URL` | Blocking per the team's testing posture |
| 5 | Boundary check | `bash scripts/check-boundaries.sh` | Outward crate edge blocks |
| 6 | Coverage | `cargo llvm-cov --workspace` (tool not installed on this machine yet) | 80% per crate |
| 7 | Frontend checks | `yarn --cwd web test lint build`; `yarn --cwd support-console test lint` | Any failure blocks |
| 8 | Image build | Multi-stage Dockerfile, digest-pinned base | Image is the deployed artifact |
| 9 | Image scan | Trivy (not yet configured) | High/critical findings block |
| 10 | Migrate | One-shot `identity-migrate` service applies the up pair before the API update | Failure stops the release |
| 11 | Deploy | Swarm rolling update, `start-first`, health check gating | Health failure triggers rollback |
| 12 | Smoke | `/healthz`, `/readyz`, and one gate decision against a known country | Failure triggers rollback |

## Deployment strategy

Rolling update on the production Swarm, using the stack's configured policy (`parallelism: 1`, `failure_action: rollback`, `monitor: 30s`). No staging tier; production is the only environment. The migration runs before the API and worker update, as the service's migrate binary header states.

## Rollback

1. The failed post-update health check triggers `failure_action: rollback`.
2. The release job applies the matching `.down.sql` for any migration in the failed release (the down step is required by team policy; it must be built and tested).
3. The previous image digest is restored.
4. The smoke check confirms recovery.

## Secrets in CI/CD

- Provider key and publish credential are Docker secrets materialised from Supabase Vault; the pipeline never echoes them.
- No secret is stored in the repository, an env file, a log, or an image layer.
- The publish credential is scoped to the publish role only.

## Feature flags

No flag gates this unit; the gate is always active once deployed. The jurisdiction list itself is the operational control (an empty list is inert but alerts loudly).
