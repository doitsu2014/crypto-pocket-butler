# CI/CD Pipeline — u6-portfolio

Inputs: `infrastructure-specification.md`, `monitoring-design.md`, `memory/team.md`.

| Stage | Command / action | Gate |
|---|---|---|
| Build | `cargo build --workspace --all-targets` | Compiles |
| Format | `cargo fmt --all -- --check` | No diff |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | No warnings |
| Boundary | `bash scripts/check-boundaries.sh` | Inward only |
| Tests | `cargo test -p portfolio-domain -p portfolio-application -p portfolio-infrastructure -p portfolio-http` | Pass |
| Coverage | per crate `cargo llvm-cov -p portfolio-<crate> --fail-under-lines 80` | ≥80% |
| Migration check | up/down/re-up under `portfolio_migrate_database_url`; asserts roles/grants | Both directions |
| Image build | `portfolio`, `portfolio-worker` digest-pinned | Build |
| Scan | `cargo audit` + Trivy | No high/critical |
| Migrate | release job runs `portfolio-migrate up` before the update | Schema ready |
| Deploy | `docker stack deploy` | `/healthz` then `/readyz` |
| Smoke | `/readyz` on API (8101) and worker (8102); `GET /v1/internal/smoke` with the smoke token | Green or rollback |

## Deployment strategy

- `portfolio-migrate` is the API image with the migrate entrypoint, awaited before the stack update.
- Rolling: API `start-first`, worker `stop-first`, `failure_action: rollback`.

## Rollback

1. Swarm restores the previous digest on failed health; the job runs `--rollback` on a failed smoke.
2. The job applies the down migration after the previous image runs.
3. Expand/contract keeps the previous image valid in between.
4. Complete when `/readyz` and the smoke are green.

## Secrets

- `portfolio_database_url`, `portfolio_migrate_database_url`, `portfolio_smoke_token`,
  `portfolio_outbound_token` from the operator store; Portfolio's outbound token is allow-listed in
  C5–C10 registries.
