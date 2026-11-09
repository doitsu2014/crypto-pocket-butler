# CI/CD Pipeline — u7-ledger

Inputs: `infrastructure-specification.md`, `monitoring-design.md`, `memory/team.md`.

| Stage | Command / action | Gate |
|---|---|---|
| Build | `cargo build --workspace --all-targets` | Compiles |
| Format | `cargo fmt --all -- --check` | No diff |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | No warnings |
| Boundary | `bash scripts/check-boundaries.sh` | Inward only |
| Tests | `cargo test -p ledger-domain -p ledger-application -p ledger-infrastructure -p ledger-http` | Pass |
| Coverage | per crate `cargo llvm-cov -p ledger-<crate> --fail-under-lines 80` | ≥80% |
| Migration check | up/down/re-up on a disposable schema using `ledger_migrate_database_url`; asserts the three roles and their grants | Both directions |
| Image build | `ledger`, `ledger-worker`, digest-pinned | Build |
| Scan | `cargo audit` + Trivy | No high/critical |
| Migrate | release job runs `ledger-migrate up` before the stack update | Schema ready |
| Deploy | `docker stack deploy` | `/healthz` then `/readyz` |
| Smoke | `/readyz` on every task; `GET /v1/internal/smoke` with the smoke token | Green or rollback |

## Deployment strategy

- `ledger-migrate` is the API image with the `migrate` entrypoint, a one-shot job awaited before the
  stack update.
- The `ledger_anonymiser` role is used only by the erasure job (C7 anonymise), never by the running
  service.
- Rolling: API `start-first`, worker `stop-first`, `failure_action: rollback`.

## Rollback

1. Swarm restores the previous digest on failed health; the job runs `--rollback` on a failed
   smoke.
2. The job applies the down migration after the previous image is running.
3. Expand/contract keeps the previous image valid in between.
4. Rollback complete when `/readyz` and a smoke read are green.

## Secrets

- `ledger_database_url`, `ledger_migrate_database_url`, `ledger_anonymiser_database_url`,
  `ledger_smoke_token`, and `ledger_outbound_token` are Docker secrets from the operator store. The
  migrate and anonymiser secrets are mounted only into the jobs/handler that use them.
- No credential appears in logs or artifacts.
