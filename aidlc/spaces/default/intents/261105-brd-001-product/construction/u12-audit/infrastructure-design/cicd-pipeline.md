# Audit — CI/CD Pipeline

GitHub Actions. Production is the only environment, so there is no staging promote step: the
pipeline's gates are the test run, the security scan, and the post-deploy smoke check.

## Stages

| # | Stage | Command | Gate |
|---|---|---|---|
| 1 | Format and lint | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` | fail fast, < 60 s |
| 2 | Unit and integration tests | `cargo test -p audit-domain -p audit-application -p audit-http`, then the adapter suite against a disposable Postgres schema | any failure blocks |
| 3 | Coverage | `cargo llvm-cov` per crate, gate at 80% line coverage (the team's floor) | blocks below the floor |
| 4 | Boundary check | `scripts/check-boundaries.sh` — created in the first code-generation step of this unit — which reads `cargo metadata` and fails if any crate depends outward | blocks |
| 5 | Build images | two multi-stage Dockerfiles: `audit-api` and `audit-retention`, both non-root, read-only root filesystem, digests recorded | blocks on build failure |
| 6 | Dependency and image scan | `cargo audit` plus Trivy on both images | blocks on high or critical (NFR12.1) |
| 7 | Migration check | apply up, apply down, re-apply up against a disposable schema | blocks; a down migration that fails is a release blocker |
| 8 | Push images | both images to the registry, digest-pinned | — |
| 9 | Apply migrations to production | `sqlx migrate run` with the `audit_migrator` credential from the `audit_migrator_dsn` Docker secret; this is the step that actually changes the schema, because `docker stack deploy` runs no SQL | blocks; a failed migration stops the release before any task is replaced |
| 10 | Deploy | `docker stack deploy` of `infra/swarm-stack.yml` | Swarm rolling update, health check gating |
| 11 | Smoke test | `GET /healthz` and `GET /readyz` on the new tasks | failure triggers the rollback below |
| 12 | Verify | append one allow-listed system event (`system.smoke.check`) and read it back | failure triggers the rollback below |

## Rollback

Rollback is a pipeline action, not a Swarm one: Swarm's `failure_action: rollback` restarts the
previous **task definition**, but it runs no SQL, so it cannot reverse a migration on its own.
The pipeline therefore does the ordered rollback explicitly:

1. Run the matching sqlx down migration with the `audit_migrator` credential. That down migration
   refuses to drop a partition still inside the retention window, so a botched release can never
   delete six months of trail as a side effect of rolling back.
2. Redeploy the previous digest with `docker stack deploy`.
3. Re-run the smoke test against the restored service.

If the down migration refuses, the pipeline stops and an operator decides — it never forces the
drop and never leaves code and schema out of step silently.

## Secrets

- Vault holds the four credentials (`audit_app`, `audit_readonly`, `audit_retention`,
  `audit_migrator`). The pipeline reads only `audit_migrator` for the migration step, and nothing
  else: the retention credential is never given to CI.
- A stack file cannot reference a Vault path, so the pipeline creates Docker secrets from Vault
  values before deploying, and the stack file references the Docker secrets — the pattern the
  walking skeleton already uses with `registry_database_url`.
- No secret is echoed. The smoke append uses the allow-listed `system.smoke.check` action with no
  actor id, which NFR10.2 permits, and it is written to the trail like any other event: a synthetic
  event that vanished after the test would be a worse audit trail than one that records the check.

## Pipeline-specific notes for this service

- The migration step runs against a disposable schema, not production, and the same migration set
  is applied to production by the deploy step.
- The image scan fails the build on a high or critical finding; there is no "warn only" path, per
  the team's quality gate rule.
- `cpb/audit-api` and `cpb/audit-retention` are two binaries in the same Cargo workspace
  (`services/audit/src/bin/api.rs` and `.../bin/retention.rs`) with two Dockerfiles, built, scanned,
  and pushed as two images by the same job. They differ in their entrypoint and the credential they
  mount, not in their codebase.

## Sources

- `construction/u12-audit/infrastructure-design/infrastructure-specification.md` — deployment facets
- `construction/u12-audit/nfr-design/reliability-design.md` — rollback and recovery
- `construction/u12-audit/nfr-design/security-design.md` — credential separation
- `construction/u12-audit/functional-design/functional-spec.md` — WF1, WF6
- `inception/requirements-analysis/requirements.md` — NFR12, NFR18
