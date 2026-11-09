# CI/CD Pipeline — u11-job-orchestration

Inputs: `infrastructure-specification.md`, `monitoring-design.md`, `memory/team.md`.

| Stage | Command / action | Gate |
|---|---|---|
| Build | `cargo build --workspace --all-targets` | Compiles |
| Format | `cargo fmt --all -- --check` | No diff |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | No warnings |
| Boundary | `bash scripts/check-boundaries.sh` | Inward only |
| Tests | disposable Postgres (`docker run postgres`), `job-orchestration-migrate up`, then `TEST_DATABASE_URL=... cargo test -p job-orchestration-*` | Pass |
| Integration tests | `TEST_DATABASE_URL=... cargo test -p job-orchestration-infrastructure --features integration -- --ignored` | Pass |
| Query plan | `EXPLAIN (FORMAT JSON)` on the due query; asserts the `(job, scheduled_for desc)` index is used | Index used |
| Coverage | per crate `cargo llvm-cov -p job-orchestration-<crate> --fail-under-lines 80`, including the integration run | ≥80% |
| Migration check | up/down/re-up under the migrator DSN | Both directions |
| C4 check | `structurizr validate` after updating `docs/architecture/workspace.dsl` and `docs/architecture/README.md` (Cron removed, orchestrator container added) | Valid, docs updated |
| Image build | `job-orchestration`, digest-pinned | Build |
| Scan | `cargo audit` + Trivy | No high/critical |
| Migrate | release job runs `job-orchestration-migrate up` before the update | Schema ready |
| Seed | `infra/sql/job-orchestration-service-tokens.sql` | Registry ready |
| Deploy | `docker stack deploy` | `/healthz` then `/readyz` |
| Smoke | `/readyz` and `GET /v1/internal/smoke` with the smoke token only; the release job holds no operator JWT and does not call `/v1/jobs` | Green or rollback |
| Alert rules | `promtool check rules` and reload | Valid |

## Deployment strategy

- Rolling `start-first` for the two replicas; `failure_action: rollback`.
- The orchestrator lease makes the overlap safe: the new replica claims only after the old one's
  lease expires or is released by shutdown.

## Rollback

1. Swarm restores the previous digest on failed health; the job runs `--rollback` on a failed smoke.
2. The job applies the down migration after the previous image runs.
3. Expand/contract keeps the previous image valid in between.
4. Complete when `/readyz` and the smoke are green.

## Secrets

`job_orchestration_database_url`, `job_orchestration_migrate_database_url`,
`job_orchestration_outbound_token`, `job_orchestration_smoke_token` from the operator store.
