# CI/CD Pipeline — u4-preferences-consent

**Upstream:** `infrastructure-specification.md`, `monitoring-design.md`, `reliability-design.md`, `team.md` (Deployment), `project.md` (canonical commands).

## The honest starting point

No CI platform exists in the repository, so these stages run when a person runs them. Two gates
are red on this tree today and the pipeline says so rather than pretending otherwise: stage 3
(boundaries) fails on the pre-existing `reporting-domain` → `serde` edge, and stage 6 (coverage)
reports the identity crates below 80% overall.

## Stages

| # | Stage | Command | Gate |
|---|---|---|---|
| 1 | Format | `cargo fmt --all -- --check` | Blocks |
| 2 | Lint | `cargo clippy --workspace --all-targets -- -D warnings` | Blocks |
| 3 | Boundaries | `bash scripts/check-boundaries.sh` | Blocks — **currently red** on `reporting-domain` → `serde`, a pre-existing edge |
| 4 | Tests | `cargo test --workspace` | Blocks |
| 5 | Adapter and migration tests | disposable Postgres: `TEST_DATABASE_URL=… cargo test --workspace -- --ignored` | Blocks — **this unit's migration 0005 is exercised here**, including its down |
| 6 | Coverage | `cargo llvm-cov --workspace` | Blocks below 80% per crate for the crates this unit changes; reports the rest |
| 7 | Frontends | `yarn` test/lint/build/format:check in both packages | Blocks |
| 8 | Image build | Identity image tagged with the commit SHA | Blocks |
| 9 | Vulnerability scan | `cargo audit`, Trivy | Blocks: high/critical (BRD-012 NFR-SEC-14; **not yet configured**) |
| 10 | Migrate | `identity-migrate` runs `0005` before the service update | Blocks |
| 11 | Deploy | Swarm rolling update, `parallelism: 1`, `failure_action: rollback`, `monitor: 30s`, health check on `/healthz` | Blocks |
| 12 | Rollback | Stack rollback, then the matching `0005_preferences_consent.down.sql` — **the automatic down step is not built**: no release job exists and nothing applies a `.down.sql` today. Owner: the release-pipeline work, alongside stage 9's scanning. This unit's down *is* real and must be tested (see below); wiring it into a job is the missing piece | Blocks once built |
| 13 | Smoke | `scripts/smoke-identity.sh`, extended with one consent-state call | Blocks |

## Migration safety — the part that matters here

The rollback path is the risk, and so is the rollout overlap. `0005` alters a live table with real
rows while three `identity-api` replicas run `start-first`, so old and new code serve together.

**Correction to the obvious plan:** plain `ALTER TABLE` and `CREATE INDEX` *are* transactional in
PostgreSQL; only the `CONCURRENTLY` variants cannot run inside a transaction. What matters is the
**lock and the rewrite**, not the transaction split:

| Step | Lock / cost |
|---|---|
| Add `kind`, `action`, `occurred_at`, `policy_version` **as nullable** | `ACCESS EXCLUSIVE`, metadata only, sub-second |
| Add `seq` as a **nullable bigint**, backfill in batches of 10 000, then `SET NOT NULL` via a validated `CHECK` (or `ALTER COLUMN … SET NOT NULL` after the backfill) | No full-table rewrite: a `GENERATED … AS IDENTITY` or `bigserial` default is volatile and *would* rewrite the table under `ACCESS EXCLUSIVE`, which is why it is added nullable and backfilled |
| Backfill the four new columns from the legacy ones | Row updates; batched so no single statement holds the lock long |
| Drop the old CHECKs and `consent_records_latest_idx`, add the new CHECKs and two indexes | `CREATE INDEX` (not `CONCURRENTLY`) inside the migration transaction; the old index stays until the new one exists in the same transaction, so no read loses its index |
| `REVOKE`, role creation, trigger | Metadata only |

**Expand now, contract later.** The new columns stay nullable through the release, so an old
replica writing a legacy row during the overlap still succeeds; a follow-up migration in the next
release tightens them once every replica runs the new code. This is stated because the alternative
— `NOT NULL` immediately — breaks the overlap the stack's `start-first` order guarantees.

A step that fails mid-migration rolls its transaction back; the migration runs as one transaction
for the metadata steps and per-batch for the backfill, so a failure leaves the table at its previous
version and the release job's rollback path applies `0005.down.sql`, which is a no-op at that point.

## Rollback — the down migration

1. **Up** — the steps above.
2. **Down** — create `consent_records_legacy_archive` **with `IF NOT EXISTS`** (so a second rollback
   after a redeploy does not fail on the table already being there), copy `risk`/`age` events into
   it, restore the old columns, collapse each `(user, kind)` to its newest event with the inverse
   mapping, and drop the new columns. The archive is a **shared resource** with an owner (u4), is
   erased by the erasure job alongside the account, and carries no retention beyond the account's.

Stage 5 runs the up, seeds legacy rows, and runs the down twice against a disposable database, so a
repeated rollback is proven before a release rather than discovered during one.

## Secrets

No new secret. The drainer uses the Identity service's existing C9 token, materialised from
Vault through the existing `*_FILE` path.

## What must exist before this unit can ship

1. The C6 amendment (consent-state endpoint, statuses, `account-api` consumer).
2. The C9 amendment (`Idempotency-Key` honoured; outbox-completion wording).
3. The Prometheus service and scrape job.
4. `cargo audit` and Trivy wired into whichever pipeline the team chooses.
