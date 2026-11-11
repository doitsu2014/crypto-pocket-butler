# CI/CD Pipeline — u3-support-access

**Upstream:** `nfr-design/reliability-design.md` (rollback procedure), `nfr-design/security-design.md`
(secrets), `infrastructure-specification.md`, `memory/project.md` (canonical build/lint/test list),
`memory/team.md` (`## Testing Posture`, `## Deployment`), `.claude/rules/development.md`.

**There is no CI platform in the repository.** No `.github/`, GitLab, Jenkins or CircleCI
configuration exists, and the team deliberately left the platform open (`memory/team.md`). This
pipeline is therefore written as the ordered set of gates a person runs today, with each stage
naming the exact command. Where it lands, it runs these commands unchanged.

## Stages

| # | Stage | Command | Fails the change when |
|---|---|---|---|
| 1 | Format | `cargo fmt --all -- --check` | Any Rust file is unformatted |
| 2 | Lint | `cargo clippy --workspace --all-targets -- -D warnings` | Any warning, including in the new support modules |
| 3 | Boundaries | `bash scripts/check-boundaries.sh` | Any outward crate edge — notably `infrastructure` importing `http` for the metrics port |
| 4 | Unit and HTTP tests | `cargo test --workspace` | Any failure; includes the gateway's 429 acceptance tests |
| 5 | Adapter and migration tests | `TEST_DATABASE_URL=<disposable db> cargo test --workspace -- --ignored` | Any failure. The gate provisions a disposable Postgres per run and runs the `#[ignore]` suites against it |
| 6 | Coverage | `cargo llvm-cov --workspace` | Any crate below 80% line coverage |
| 7 | Frontend (unchanged, still run) | `yarn test`, `yarn lint`, `yarn format:check`, `yarn build` in `web/` and `support-console/` | Any failure — the unit changes no frontend file, but the gates are not skipped |
| 8 | Migration pair check | Both `0007_support_access.up.sql` and `.down.sql` present, both apply and reverse on the disposable database | A schema change without a matching down migration (forbidden rule) |
| 9 | Image build | The existing `Dockerfile` / `Dockerfile.migrate` per service | The image fails to build |
| 10 | Dependency and image scan | `cargo audit`; Trivy on each image | Any `cargo audit` advisory rated medium (CVSS ≥ 4.0) or above; any Trivy `HIGH` or `CRITICAL` finding with a fix available. An accepted finding is an allowlist entry with an owner and an expiry date, reviewed by devsecops — never a blanket ignore |
| 11 | Migrate, then deploy | The migrator as a one-shot `docker service create` whose task is awaited, then `docker stack deploy -c infra/swarm-stack.yml` | The migration task exits non-zero (the release stops before any API replica changes) or the update's health check fails |

Stage 5 is a blocking gate, not a developer opt-in (`memory/team.md`); stage 10 is required by
BRD-012 but names tooling the repository does not configure yet — it is recorded as a requirement
of the pipeline, not as a step that exists, and **its severity levels are pinned in this document
precisely because no pipeline image exists to pin them**. When the platform is chosen, the
scanner versions are pinned by that image and the levels above do not move.

## The migration gate

A release that ships `0007_support_access` must run the migration to completion **before**
`identity-api` is updated. `docker stack deploy` cannot express that: it creates and updates every
service in the stack in one operation and does not wait for a one-shot task to exit, so a failed
migrate task inside the stack would leave the API already updated. The order is therefore enforced
by the release job running the migration as its own step:

1. `docker service create --name identity-migrate-<tag> --network crypto-pocket-butler_internal
   --secret identity_migrate_database_url --env DATABASE_URL_FILE=/run/secrets/identity_migrate_database_url
   --restart-condition none <registry>/identity-migrate:<tag>`
2. Poll `docker service ps identity-migrate-<tag> --filter desired-state=shutdown
   --format '{{.CurrentState}}'` until it reads `Complete` or `Failed`, with a 10-minute timeout.
3. `docker service rm identity-migrate-<tag>`.
4. On `Failed`, **stop**: no `docker stack deploy` runs, the previous `identity-api` replicas keep
   serving, and the rollback procedure's step 3 applies the down pair.
5. Only after `Complete`, `docker stack deploy -c infra/swarm-stack.yml crypto-pocket-butler`.

Who orders the migrate step against the service update is recorded as open in `memory/team.md`; the
mechanism above is what this unit needs to make its own migration safe, and the release job is the
place it belongs. The `identity-migrate` service stays in the stack file for the deploy that has
no release job, where its one-shot task is a no-op on an already-migrated schema — it is not the
ordering mechanism.

## Deployment strategy

Docker Swarm rolling update, unchanged from `infra/swarm-stack.yml`: `parallelism: 1`,
`order: start-first` for `identity-api` and `identity-worker`, `failure_action: rollback`,
`monitor: 30s`, and the `/healthz` health check as the gate. No blue-green and no canary path is
introduced.

`/healthz` reports only that the process is up, so an Account or Portfolio outage cannot stall the
update; `/readyz` reports the database and the audit store, and the two metadata sources are
reported there without failing readiness. The support outbox is drained by the existing drainer
inside `identity-api` — 3 replicas, all draining under `FOR UPDATE SKIP LOCKED`, not by
`identity-worker` — and its heartbeat is in neither endpoint: a lagging drainer is an alert
(`SupportOutboxStalled`), not a readiness failure, so a drainer stall never pulls healthy replicas
out of service.

## Rollback procedure

1. **Stop the release.** Swarm's `failure_action: rollback` reverts `identity-api` on a failed
   health check; a manual rollback is the same stack file with the previous `${TAG}`.
2. **Export undelivered audit intents, then empty the table.** Both statements run as
   `identity_migrator`, the role the release job already holds, and both are database operations —
   they work whether or not the audit store is reachable, which matters because an audit outage is
   one of the things that forces a rollback.
   ```sh
   psql "$IDENTITY_MIGRATE_DATABASE_URL" -c "\copy (SELECT * FROM identity.support_audit_outbox \
     WHERE state IN ('pending','dead')) TO 'outbox-<record>-<date>.csv' WITH (FORMAT csv, HEADER true)"
   psql "$IDENTITY_MIGRATE_DATABASE_URL" -Atc "SELECT count(*) FROM identity.support_audit_outbox \
     WHERE state IN ('pending','dead')"
   ```
   The count must equal the exported file's rows minus its header line; if it does not, stop — the
   export did not capture what it removed. Then delete exactly the exported set:
   ```sh
   psql "$IDENTITY_MIGRATE_DATABASE_URL" -c "DELETE FROM identity.support_audit_outbox \
     WHERE state IN ('pending','dead')"
   ```
   The export file **is** the record of the intents that will never be delivered; it holds audit
   payloads, so it is stored where audit records are and never in a build log.
3. **Reverse the schema.** Run `0007_support_access.down.sql`. It restores the pre-0007 approval
   shape as `NOT VALID` — historical grant rows stay exactly as recorded — and its guard raises if
   any `pending` or `dead` row is still present, so step 2 cannot be skipped silently. Because step
   2 emptied the table, the guard passes.
4. **Confirm.** Read the running image back
   (`docker service inspect --format '{{.Spec.TaskTemplate.ContainerSpec.Image}}' crypto-pocket-butler_identity-api`)
   and check it names the previous tag; `GET /healthz` and `GET /readyz` both return 200 on it.
   `/readyz` is the check that can actually fail — it reports the database and the audit store — so
   a green `/readyz` on the previous tag is the rollback's proof of health.

If step 2 cannot run (no database access at all), the down migration refuses and the release stays
on the new schema with nothing dropped. That is the intended failure: an undelivered audit intent
is never lost to a rollback, and a rollback that cannot prove otherwise does not happen.

Every step is reversible from the previous tag; the only irreversible act (dropping the outbox) is
guarded by the export.

## Secrets in the pipeline

No secret is added by this unit. The identity service token already exists in the service-token
registry and reaches `identity-api` through `identity_service_tokens`; the database URL reaches
`identity-migrate` through `identity_migrate_database_url`. Both are Docker secrets materialised
from Supabase Vault. The pipeline injects no credential into an environment variable, a build log
or a test fixture, and the scanner stage must never print a decoded secret.

## What this unit adds to the pipeline

Two steps only: the migration pair check (stage 8) gains its first file for this unit, and the
alert file `infra/prometheus/support-rules.yml` ships with the stack. Everything else is the
existing gate list applied unchanged.