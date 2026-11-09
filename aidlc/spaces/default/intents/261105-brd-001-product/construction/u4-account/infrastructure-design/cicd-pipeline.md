# CI/CD Pipeline — u4-account

Inputs: `infrastructure-specification.md`, `monitoring-design.md`,
`nfr-design/reliability-design.md`, and `inception/contract-design/contract-summary.md`.

## Stages

| Stage | Command / action | Gate |
|---|---|---|
| Build | `cargo build --workspace --all-targets` for the Account workspace | Compiles |
| Format | `cargo fmt --all -- --check` | No diff |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | No warnings |
| Boundary check | `bash scripts/check-boundaries.sh` (the repo's clean-architecture check over the four crates) | Inward-only imports |
| Unit + integration tests | `cargo test --workspace` against a disposable Postgres schema | All pass |
| Coverage | Per crate: `cargo llvm-cov -p account-domain --fail-under-lines 80`, and the same for `account-application`, `account-infrastructure`, `account-http` | ≥ 80% line coverage in each crate; the tool is the working choice pending the team's confirmation |
| Migration check | sqlx up/down pair applied, then down, then up on a disposable schema | Both directions succeed |
| Image build | Digest-pinned `account` and `account-worker` images | Build succeeds |
| Vulnerability scan | `cargo audit` for dependencies and Trivy for both images (BRD-012 NFR-SEC-14) | No high or critical |
| Deploy | See "Deployment strategy" | `/readyz` health check passes |
| Smoke | `/healthz` and `/readyz` on every task; one C5 read through the gateway; one staged import round trip | All green, or rollback starts |

## Deployment strategy

- The deployable input is `infra/swarm-stack.yml`. The release job exports
  `ACCOUNT_IMAGE_DIGEST` and `ACCOUNT_WORKER_IMAGE_DIGEST`, runs
  `docker stack deploy --with-registry-auth`, and Swarm pulls the digest-pinned image.
- Migrations run as a one-shot Swarm job before the update:
  `docker service create --mode replicated-job --replicas 1 --constraint 'node.role==manager'
  --restart-condition none account-migrate:<digest> up`; the job waits on
  `docker service ps account-migrate --format '{{.CurrentState}}'` and fails the release on a
  non-zero exit. `account-migrate` is the `account` image with the `migrate` entrypoint, so it is
  built and scanned with the API image.
- A release is a Swarm rolling update of `account` (`start-first`) and `account-worker`
  (`stop-first`). Each task must pass its `/healthz` check, and the release job then runs the
  `/readyz` smoke on the new tasks before promoting; the update config sets
  `failure_action: rollback`. The job watches progress with `docker service ps`.
- No staging tier: the same commands run locally and in CI, and the production stack is the only
  deployment target.

## Rollback

One order, executed by the release job:
1. The job watches the stack update (`docker service ps`); on a failed `/healthz` Swarm restores
   the previous image digest, and on a failed `/readyz` smoke the job runs
   `docker service update --rollback` itself.
2. Once the previous image is running, the job runs the matching `down` pair with the same one-shot
   job, then re-runs the smoke check. The down step is automatic; the alert matches this path.
3. Migrations must be expand/contract and backward compatible, so the previous image keeps working
   against the up schema for the short window between the rollback and the down step.
4. If the down pair fails, the job stops, pages the on-call engineer, and leaves the previous image
   running on the new schema (safe because of step 3); the runbook covers manual recovery.
5. The rollback is complete only when `/readyz` and one C5 read are green.

## Secrets in CI/CD

- Bootstrap configuration is Docker secrets, not Vault: `account_database_url`,
  `account_supabase_service_role`, and `account_migrate_database_url` are created at
  `docker stack deploy` from the operator's secret store and mounted as `*_FILE` into the services.
  Vault lives inside the database, so it cannot hold the database URL.
- The release job authenticates to Swarm with its manager credential; it never reads Vault.
- Per-user exchange credentials are written to Vault at runtime, by the service, after it connects
  with the Docker-secret DSN; nothing is written to the repository or logs.
- Images are digest-pinned; registry credentials are short-lived and scoped to the image registry.
- No credential or token appears in build logs or artifacts.

## Feature flags

- No deployment-gating flag is used for this unit; account and credential changes are additive
  behind the C1 routes. A behaviour change that needs a flag gets an owner and expiry before it
  ships.
