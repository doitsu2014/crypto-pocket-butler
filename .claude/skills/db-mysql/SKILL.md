---
name: db-mysql
description: >
  Use when designing, implementing, reviewing, or testing MySQL or MariaDB
  integration: schema and migrations, drivers/ORMs, connection pooling,
  InnoDB transactions and isolation, indexing and query plans, online DDL,
  and secure configuration. Applies to any stack talking to MySQL.
license: MIT-0
compatibility: Any language or framework with a MySQL driver.
metadata:
  category: database
  engine: mysql
  aliases: mariadb, innodb
---

# MySQL / MariaDB Integration

Apply this skill whenever a stage touches a MySQL or MariaDB database: schema
design, migrations, queries, transactions, performance, or tests.

## When this skill applies

- Inception: choosing MySQL/MariaDB, designing the schema, or defining the
  contract.
- Construction: writing queries, repositories, migrations, or seed data.
- Construction: containerised test databases, CI migration runs, fixtures.
- Operation: index and query-plan review, slow-query triage, replication
  health.

## Configuration and secrets

- Connect with credentials from environment/secret manager only.
- Require TLS for remote connections (`ssl-mode=REQUIRED` /
  `VERIFY_IDENTITY` on managed services).
- Set an explicit `connectionAttributes`/`application_name`-style tag where the
  driver supports it, so sessions are attributable.
- Use a least-privilege account per service; avoid `ALL PRIVILEGES` and
  `GRANT OPTION`.
- Set `wait_timeout`, `interactive_timeout`, `max_execution_time`, and
  `lock_wait_timeout` explicitly.

## Drivers and ORMs (pick one per codebase and record it)

| Ecosystem | Driver | Common ORM/query layer |
| --- | --- | --- |
| Node/TypeScript | `mysql2` | Prisma, Drizzle, TypeORM, Kysely |
| Python | `PyMySQL` / `mysqlclient` | SQLAlchemy, Django ORM, Peewee |
| Java/Kotlin | JDBC (`com.mysql.cj.jdbc`) | Hibernate/JPA, jOOQ, MyBatis |
| Go | `go-sql-driver/mysql` | sqlc, GORM, Ent |
| .NET | `MySqlConnector` | EF Core, Dapper |

Record the driver and version in `construction/nfr-requirements`, and the
pooling strategy in `construction/nfr-design`.

## Schema and data modelling

- Use `InnoDB` for every table (transactions, foreign keys, row locks).
- Use `utf8mb4` with a modern collation (`utf8mb4_0900_ai_ci` on MySQL 8+,
  `utf8mb4_unicode_ci` on MariaDB/older). Never `utf8` (3-byte) if you accept
  user text.
- Prefer `BIGINT UNSIGNED AUTO_INCREMENT` primary keys, or `BINARY(16)` for
  UUIDs. `CHAR(36)` UUIDs waste space and index locality; if you need random
  IDs, store them as `BINARY(16)`.
- Use `DATETIME(6)`/`TIMESTAMP(6)` consistently and store UTC. `TIMESTAMP` has
  a 2038 limit and implicit time-zone conversion; `DATETIME` does not.
- MySQL has no native `boolean` or `enum`-with-constraints worth relying on;
  use `TINYINT(1)` and `VARCHAR` + `CHECK` (8.0.16+).
- Use `JSON` (8.0+) for schemaless attributes, not as a substitute for indexed
  columns.
- Add `NOT NULL`, `CHECK`, and foreign keys by default. Choose a naming
  convention and enforce it.
- Account for index key length limits (3072 bytes with `DYNAMIC` row format).

## Migrations

Choose one tool and make every schema change a reviewed, forward-only
migration:

- Prisma Migrate, Drizzle Kit, TypeORM migrations
- Flyway, Liquibase (language-agnostic)
- Alembic (Python), goose/golang-migrate/dbmate (Go), EF Core Migrations (.NET)

Rules:
- Migrations are immutable once merged. Fix forward.
- Separate schema changes from data backfills.
- Use online DDL where possible (`ALGORITHM=INPLACE, LOCK=NONE`); verify with
  `SHOW WARNINGS`. Some changes are `COPY`-only and will lock the table — for
  large tables use `gh-ost` or `pt-online-schema-change`.
- Adding a nullable column is cheap; adding an index, changing a type, or
  changing the primary key is not.
- For destructive changes: expand → migrate → contract across releases.

## Query patterns and performance

- Index for the access pattern; column order in a composite index matters.
- Remember the leftmost-prefix rule: an index on `(a, b)` serves `a` and
  `a, b` but not `b` alone.
- Read plans with `EXPLAIN` and, on 8.0.18+, `EXPLAIN ANALYZE`. Watch for
  `Using filesort`, `Using temporary`, full scans on large tables, and bad row
  estimates.
- Enable and inspect the slow query log and `performance_schema`.
- Avoid `SELECT *` in hot paths; avoid functions on indexed columns in
  `WHERE`.
- Watch for N+1 queries from ORMs.
- Use keyset pagination instead of large `OFFSET`.
- Covering indexes and `ORDER BY` alignment eliminate sorts.

## Transactions and concurrency

- Default isolation is `REPEATABLE READ` (InnoDB), which differs from
  PostgreSQL. Understand gap locks and next-key locks.
- Deadlocks are normal under load: catch error `1213`, retry the transaction,
  and log the frequency.
- Keep transactions short; never hold one across a network or user wait.
- Use `SELECT ... FOR UPDATE` / `FOR SHARE` deliberately, with the matching
  index or you lock more than intended.
- Use `GET_LOCK`/`RELEASE_LOCK` for application-level mutexes.

## Connection pooling

- Pool per process; size from the server's `max_connections`, not request
  concurrency.
- Set `connectionLimit`, `maxIdle`, `idleTimeout`, and a connect timeout.
- Account for proxy poolers (ProxySQL) and their own limits.
- Always set `max_execution_time` so runaway queries fail.

## Security

- Parameterise every query. Never string-concatenate SQL.
- Avoid `LOAD DATA LOCAL INFILE` unless deliberately configured.
- Encrypt sensitive columns at the application layer where possible.
- Grant only required privileges; audit accounts with `SUPER`/`FILE`.
- Never log credentials or PII-bearing rows.

## Testing

- Use Testcontainers (`mysql`/`mariadb`) or an ephemeral service.
- Run migrations in test setup so tests exercise the real schema.
- Reset state between tests; avoid order-dependent fixtures.
- Assert on constraints and error codes, not just happy paths.

## Replication and operations

- Track replica lag (`Seconds_Behind_Source`/`Seconds_Behind_Master`), and know
  that it is a coarse signal.
- Choose binlog format deliberately (`ROW` is the safe default) and GTID where
  supported.
- Back up with a consistent method (`mysqldump --single-transaction`, Percona
  XtraBackup, or managed snapshots) and test restores.
- Prefer reversible migrations; for destructive changes use expand/contract.

## Common pitfalls

- Assuming PostgreSQL semantics: `REPEATABLE READ` in InnoDB uses gap locks and
  behaves differently.
- Forgetting `utf8mb4`, then losing emoji and non-BMP text.
- `ALTER TABLE` taking a metadata lock on a busy table.
- Implicit type conversion in `WHERE` defeating an index.
- Storing UUIDs as `CHAR(36)` and suffering random-insert index fragmentation.

## How this skill plugs into my-aidlc

- Construction: record the engine/version (MySQL vs MariaDB), driver/ORM, and
  migration tool as tech-stack decisions in `construction/nfr-requirements`;
  record the pooling strategy in `construction/nfr-design`.
- Construction: every schema change is a migration under
  `construction/code-generation`; note the tool and commands in
  `code-generation-plan`.
- Construction: integration tests use Testcontainers; CI runs migrations
  before `construction/build-and-test`.
- Operation: use slow-query and `EXPLAIN` evidence in
  `operation/performance-validation`.
