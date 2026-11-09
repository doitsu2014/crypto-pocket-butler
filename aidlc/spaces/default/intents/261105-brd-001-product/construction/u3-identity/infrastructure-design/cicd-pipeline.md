# CI/CD Pipeline — u3-identity

Inputs: `nfr-design/performance-design.md`, `security-design.md`, `scalability-design.md`,
`reliability-design.md`, `observability-design.md`, `logical-components.md`,
`inception/domain-design/components.md`, `functional-design/functional-spec.md`, and
`inception/contract-design/contract-summary.md`.

The project's CI runs the same commands developers run locally; production is the only
environment, and a release rolls out as a health-checked Swarm rolling update.

## Stages

| Stage | Command | Gate |
|---|---|---|
| 1. Format | `cargo fmt --all -- --check` | Must pass |
| 2. Lint | `cargo clippy --workspace --all-targets -- -D warnings` | Must pass |
| 3. Boundaries | `scripts/check-boundaries.sh` | Must pass (15 packages) |
| 4. Test | `cargo test --workspace` (includes `cargo test --manifest-path services/identity/Cargo.toml`) | Must pass; the pipeline provisions a disposable Postgres service container (or an ephemeral Supabase branch) and fails the stage if it cannot, so the adapter tests always run |
| 5. Web | `yarn --cwd web test && yarn --cwd web lint && yarn --cwd web build` | Must pass |
| 6. Image build | Multi-stage Dockerfile, non-root, read-only root filesystem, digest-pinned bases | Must pass |
| 7. Scan | Dependency and container-image vulnerability scan | Fails on a known critical (NFR12.1) |
| 8. Push | Push the digest-pinned image to the registry | Must pass |
| 9. Migrate | Run the sqlx up migration as a Swarm one-shot job with the dedicated DDL role (never the service role) | Must pass; every migration is backward compatible for one release (expand/contract) |
| 10. Deploy | `docker stack deploy` with `failure_action: rollback` and the `/readyz` health check | Must pass |
| 11. Smoke | Call `/healthz` and `/readyz` through the gateway and resolve the dedicated smoke-test principal (a pre-created account whose credential lives in the CI secret store; its User row and audit events are tagged `smoke-test` and retained for audit, not deleted) | Must pass, else roll back |

## Deployment strategy

- Rolling update with Swarm defaults; each task must pass `/readyz` before the next is replaced.
- A failed health check triggers the stack's `rollback` action.
- Rolling back a release also applies the matching sqlx down migration (team rule).

## Rollback procedure

1. `docker service update --rollback identity-api` and `identity-worker` (or redeploy the previous
   digest-pinned image), so the previous image serves again.
2. Only after the previous image is serving, apply the matching sqlx down migration for any
   migration in the release; the migration is contract-compatible, so no row the new version wrote
   is discarded. If a down migration would discard data, it is not run automatically; the runbook
   records the manual data-handling step.
3. Confirm `/readyz` on every task and a successful principal resolution.
4. Record the rollback in the runbook and the audit trail.

## Environment promotion

- No staging tier: the pipeline builds, scans, migrates, deploys, and smoke-checks production.
- The approval to deploy is a human gate in the pipeline configuration (release approval); the
  workflow's own production deploy stages belong to the Operation phase.

## Secrets in CI/CD

- Credentials come from Supabase Vault or the CI secret store; never from env files or the repo.
- The Supabase service role, the database DSN, and the Vault token are injected as Swarm secrets.
- The Docker registry token is short-lived and least-privilege.
- No secret is echoed in logs; the scan and build stages redact environment values.

## Test automation

- Unit and integration tests run in stage 4; the Postgres adapter tests use a disposable database
  when the pipeline provides one, and are reported as skipped otherwise.
- The release is blocked if the boundary or coverage gate fails; the 80% per-crate floor is
  measured on the assembled workspace.

NFR references: NFR12.1 (dependency and image scan, stage 7), NFR14.2 (retention on rollback),
NFR17.1 (structured pipeline logs).
