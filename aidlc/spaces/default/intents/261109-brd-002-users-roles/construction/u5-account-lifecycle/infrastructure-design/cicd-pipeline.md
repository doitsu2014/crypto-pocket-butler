# CI/CD Pipeline — u5-account-lifecycle

**Upstream:** `infrastructure-specification.md`, `monitoring-design.md`, `reliability-design.md`, `team.md` (Deployment), `project.md` (canonical commands).

## The honest starting point

No CI platform exists; two gates are red on this tree for pre-existing reasons (the
`reporting-domain` boundary edge, and coverage below the floor). These stages are what a pipeline
*would* run.

## Stages

| # | Stage | Command | Gate |
|---|---|---|---|
| 1 | Format | `cargo fmt --all -- --check` | Blocks |
| 2 | Lint | `cargo clippy --workspace --all-targets -- -D warnings` | Blocks |
| 3 | Boundaries | `bash scripts/check-boundaries.sh` | Blocks — pre-existing red on `reporting-domain` |
| 4 | Tests | `cargo test --workspace` | Blocks |
| 5 | Adapter + migration tests | disposable Postgres: `cargo test --workspace -- --ignored` | Blocks — **`0006`'s up, down, and a second down are exercised here** |
| 6 | Coverage | `cargo llvm-cov --workspace` | Blocks below 80% for the crates this unit changes; owner: integration/release, before merge |
| 7 | Frontends | `yarn` test/lint/build/format:check in both packages | Blocks |
| 8 | Image build | Identity API and worker images, tagged with the SHA | Blocks |
| 9 | Vulnerability scan | `cargo audit`, Trivy | Blocks: high/critical (**not yet configured**) |
| 10 | Migrate | `identity-migrate` runs `0006` before the service update | Blocks |
| 11 | Deploy | Swarm rolling update, health check on `/healthz` | Blocks |
| 12 | Rollback | Stack rollback, then `0006_account_lifecycle.down.sql` — **the automatic down step is not built**; owner: release-pipeline work. The down refuses while any open deletion request exists, so a rollback cannot strand a deletion | Blocks once built |
| 13 | Smoke | `scripts/smoke-identity.sh`, extended with one status read and one denial check | Blocks |

## Migration safety

`0006` alters a live `users` row shape and an existing `deletion_requests` table, so it follows
`0005`'s pattern: nullable columns, batched backfill, `CONCURRENTLY` indexes, `NOT VALID` +
`VALIDATE`, executed statement-wise by the runner.

**Legacy data, decided rather than assumed:**

- `recoverable_until` is `NOT NULL` with no default: the migration backfills existing rows with
  `COALESCE(completed_at, requested_at) + interval '30 days'` — a closed row keeps its outcome date,
  an open one gets its window from the request — and only then sets `NOT NULL`.
- Before `CREATE UNIQUE INDEX CONCURRENTLY` on the open-request predicate, a **pre-flight duplicate
  check** runs: any `user_id` with more than one open row is reported and the migration **aborts**
  rather than building an index that cannot be validated. The operator resolves the duplicates with a
  statement the migration header gives, then re-runs.
- A concurrent build that fails leaves an **invalid** index (`indisvalid = false`); the migration's
  first step drops any invalid index with that name before building again, so a re-run recovers
  instead of silently enforcing nothing.

**The down is the dangerous direction.** It drops `status_transitions` and `users.erased_subject_hash`,
and if any account has been erased, that destroys the tombstone the previous build needs: the old
code looks the subject up through `user_personal` — deleted by erasure — finds nothing, and
**provisions a brand-new account for a token BR5.8/AC4.5.1 requires be denied.** The down therefore
**refuses while any row has `erased_at` set or any `users.status = 'deleted'` exists**, and the
migration header states that consequence in exactly those terms. A rollback past erasure is a human
decision with a documented procedure, not a silent side effect.

`status_transitions` is **insert-only**: `GRANT SELECT, INSERT` to `identity_app` and the same
`BEFORE UPDATE OR DELETE` trigger pattern `0005` uses.

Stage 5 runs the up, seeds a requested/cancelled/erased mix, **closes the request before running the
down**, runs the down, and runs it a second time (every down statement is `IF EXISTS`). A separate
test seeds an erased subject and asserts its token is **denied, not provisioned**.

## Secrets

None new. The retention DSN is u4's `identity_retention_database_url`; the Auth admin credential and
the C9 token are the service's existing ones.

## What must exist before this unit can ship

1. u2's read-versus-write predicate and `account:connect` operation.
2. u1's tombstone lookup change (`erased_subject_hash` consulted; `SubjectErased` denied).
3. The Prometheus service and the two scrape jobs.
4. C7's 409/410 responses and `recoveryEndsAt`.
