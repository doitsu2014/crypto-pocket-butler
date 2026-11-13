# Database migrations

Schema changes ship as versioned sqlx up/down pairs. Everything below is either a rule the team
already affirmed or a pattern the existing migrations established and you should follow.

## The pair

```
services/<service>/migrations/0007_support_access.up.sql
services/<service>/migrations/0007_support_access.down.sql
```

- Numbered consecutively **within the service**, four digits, `<snake_case>` name. Two units never
  take the same number — check the directory before choosing one.
- An up migration without its down migration is not shippable. A rollback applies the matching
  `.down.sql` as part of the release job.
- The down migration must succeed from **any** state the up can produce. If the up widened a CHECK,
  the down first maps the newly legal values back to a legal old value (with a comment saying why)
  and only then re-imposes the narrower constraint. `0004_user_registry_extensions.down.sql` does
  exactly this.
- Object names inside migrations are **unqualified**: the runner sets `search_path` to the
  service's schema, and adapter tests point the same files at a disposable schema. A qualified name
  breaks that.

## The header comment

Each file opens by saying which rule each change serves and why it is shaped that way — BR ids,
constraint names, and the reason a relaxation is the narrowest one possible:

```sql
-- 0007 — Support access: the audit outbox and the live-pair index.
--
-- Three changes, each serving one rule:
--   1. `support_grants_approval_shape` admits exactly one more row — …
```

That header is how a reviewer checks intent without reverse-engineering the DDL. Write it before
the SQL.

## DDL on a live table

`0005_preferences_consent` is the reference for alterations that must not lock a live table. Its up
migration is applied **statement by statement outside a transaction** by
`infrastructure/src/migrations.rs`: backfills commit in bounded batches, indexes are built
`CONCURRENTLY`, and CHECKs are added `NOT VALID` and then validated. PostgreSQL refuses
`CREATE INDEX CONCURRENTLY` inside a multi-statement implicit transaction, which is why the runner
executes each top-level statement on its own. Ordinary migrations stay transactional.

## Seeding rows the application depends on

Seed scripts live in `infra/sql/`. A row that carries authority — a service token, a role grant, a
catalogue entry the code looks up — must be **re-appliable**: `ON CONFLICT … DO UPDATE`, never
`DO NOTHING`. `account-service-tokens.sql` and `ledger-service-tokens.sql` are the pattern. A
`DO NOTHING` seed silently keeps a stale row (for example a token row whose `operations` list was
never widened), and the application then refuses traffic it should accept. Several older seed files
still use `DO NOTHING`; treat that as a known open defect, not as precedent.

## Running them

- The runner is `<service>/infrastructure/src/migrations.rs`; the binary is
  `<service>/src/bin/migrate.rs`. It reads the DSN from `DATABASE_URL_FILE` (preferred, a Docker
  secret path) or `DATABASE_URL`, opens a small pool, applies the migrations and exits.
- In Swarm the binary runs as a one-shot `<service>-migrate` service with `restart_policy: none`,
  before the API and worker update. In `deployment/localhost` the same migrations are applied by the
  `provision` job.

## Testing them

- Migration and adapter suites are `#[ignore]`d and run against a **disposable** database:
  `TEST_DATABASE_URL` pointing at a throwaway `supabase/postgres` container. This is a blocking
  gate, not an opt-in — every gate run provisions one.
- A migration is proven by applying it and reversing it in the same test, then asserting the schema
  that matters (the constraint that now admits the new shape, the index that now exists).
- A test that only checks "the migration applied" proves nothing; assert the behaviour the
  migration was written to enable.