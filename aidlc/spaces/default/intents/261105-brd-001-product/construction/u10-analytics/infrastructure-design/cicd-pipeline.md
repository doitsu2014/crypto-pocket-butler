# CI/CD Pipeline — u10-analytics

Inputs: `infrastructure-specification.md`, `monitoring-design.md`, `memory/team.md`.

## Stages

| Stage | Command / action | Gate |
|---|---|---|
| Build | `cargo build --workspace --all-targets` | Compiles |
| Format | `cargo fmt --all -- --check` | No diff |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | No warnings |
| Boundary | `bash scripts/check-boundaries.sh` | Inward only |
| Tests | `cargo test -p analytics-domain -p analytics-application -p analytics-infrastructure -p analytics-http` | Pass |
| Coverage | per crate `cargo llvm-cov -p analytics-<crate> --fail-under-lines 80` | ≥80% |
| Migration check | up/down/re-up on a disposable schema | Both directions |
| Image build | `analytics` and `analytics-worker`, digest-pinned | Build |
| Scan | `cargo audit` + Trivy | No high/critical |
| Migrate | release job runs `analytics-migrate up` before any stack update | Schema ready |
| Deploy | `docker stack deploy` with digests | `/healthz` then `/readyz` |
| Smoke | `/readyz` on every task; `GET /v1/internal/smoke` with the `smoke-reader` token (checks Postgres and salt reachability) | Green or rollback |

## Deployment strategy

- `analytics-migrate` is the API image with the `migrate` entrypoint, a one-shot Swarm job waited on
  before the stack update.
- Rolling: API `start-first`, worker `stop-first`, `failure_action: rollback`.

## Rollback

1. Swarm restores the previous digest on failed health; the job runs `--rollback` on a failed
   `/readyz` smoke.
2. The job applies the down migration after the previous image is running.
3. Expand/contract keeps the previous image valid in between.
4. Rollback complete when `/readyz` and one flag read are green.

## Secrets

- `analytics_database_url`, `analytics_smoke_token`, and `analytics_outbound_token` are Docker
  secrets from the operator store. The outbound token is used only for C3 and C10.
- Token hashes are seeded into `analytics.service_token_registry` by the deploy job; rotation
  inserts then deletes.
- Vault salt is read at runtime; never in CI.
