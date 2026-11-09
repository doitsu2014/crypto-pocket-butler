# CI/CD Pipeline — u8-reporting

Inputs: `infrastructure-specification.md`, `monitoring-design.md`, `memory/team.md`.

| Stage | Command / action | Gate |
|---|---|---|
| Build | `cargo build --workspace --all-targets` | Compiles |
| Format | `cargo fmt --all -- --check` | No diff |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | No warnings |
| Boundary | `bash scripts/check-boundaries.sh` | Inward only |
| Tests | `cargo test -p reporting-domain -p reporting-application -p reporting-infrastructure -p reporting-http` | Pass |
| Coverage | per crate `cargo llvm-cov -p reporting-<crate> --fail-under-lines 80` | ≥80% |
| Migration check | up/down/re-up under the migrator DSN; asserts roles | Both directions |
| Image build | `reporting`, `reporting-worker` digest-pinned | Build |
| Scan | `cargo audit` + Trivy | No high/critical |
| Migrate | release job runs `reporting-migrate up` before the update | Schema ready |
| Deploy | `docker stack deploy` | `/healthz` then `/readyz` |
| Smoke | `/readyz` on the API (8103) and worker (8104); `GET /v1/internal/smoke` on the API with the smoke token | Green or rollback |

## Deployment strategy

- `reporting-migrate` is the API image with the migrate entrypoint, awaited before the stack update.
- Rolling: API `start-first`, worker `stop-first`, `failure_action: rollback`.

## Rollback

1. Swarm restores the previous digest on failed health; the job runs `--rollback` on a failed smoke.
2. The job applies the down migration after the previous image runs.
3. Expand/contract keeps the previous image valid in between.
4. Complete when `/readyz` and the smoke are green.

## Secrets

- `reporting_database_url`, `reporting_migrate_database_url`, `reporting_smoke_token`,
  `reporting_outbound_token` from the operator store; the outbound token is allow-listed at
  C3/C7/C8/C9/C10.
