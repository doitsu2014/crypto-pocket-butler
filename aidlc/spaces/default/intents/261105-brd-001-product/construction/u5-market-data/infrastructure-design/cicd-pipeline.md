# CI/CD Pipeline — u5-market-data

Inputs: `infrastructure-specification.md`, `monitoring-design.md`, and the project's `memory/team.md`.

## Stages

| Stage | Command / action | Gate |
|---|---|---|
| Build | `cargo build --workspace --all-targets` | Compiles |
| Format | `cargo fmt --all -- --check` | No diff |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | No warnings |
| Boundary check | `bash scripts/check-boundaries.sh` | Inward-only imports |
| Tests | `cargo test -p market-data-domain -p market-data-application -p market-data-infrastructure -p market-data-http` | All pass |
| Coverage | per crate `cargo llvm-cov -p market-data-<crate> --fail-under-lines 80` | ≥ 80% per crate |
| Migration check | sqlx up/down/re-up on a disposable schema | Both directions succeed |
| Image build | Digest-pinned `market-data` and `market-data-worker` images | Build succeeds |
| Scan | `cargo audit` + Trivy | No high or critical |
| Migrate (pre-deploy) | The release job runs `market-data-migrate up` to completion before any stack update; a non-zero exit aborts the release | Schema is at the new version |
| Deploy | `docker stack deploy` with digest env vars | `/healthz` then `/readyz` |
| Smoke | `/readyz` on every task; a one-shot `market-data-smoke` task on `internal` calls `getPrices` and `getFxRate` with the `smoke-reader` token from `market_data_smoke_token` | Green or rollback |

## Deployment strategy

- `market-data-migrate` is the API image with the `migrate` entrypoint, run as a one-shot Swarm job
  (`--mode replicated-job --replicas 1 --constraint node.role==manager`) and awaited before the
  stack update; the stack is not redeployed when it fails.
- Rolling update: API `start-first`, worker `stop-first`, `failure_action: rollback`; the release
  job runs the `/readyz` smoke before promoting.

## Rollback

1. Swarm restores the previous digest on a failed health check; the job triggers
   `docker service update --rollback` on a failed `/readyz` smoke.
2. The job applies the matching down migration after the previous image is running.
3. Migrations are expand/contract so the previous image stays valid in between.
4. Rollback completes when `/readyz` and one price read are green.

## Secrets in CI/CD

- `market_data_database_url` (role `market_data_svc`) and `market_data_smoke_token` are Docker
  secrets created at deploy from the operator store. The per-caller token hashes (Portfolio,
  Reporting, Job Orchestration, `smoke-reader`) are seeded into
  `market_data.service_token_registry` by the deploy job; rotation inserts the new hash and deletes
  the old row. The C11 run token is read from Vault at startup.
- Provider keys are not in CI; they are written to Vault at runtime.
- No credential appears in logs or artifacts.
