---
name: db-mssql
description: >
  Use when designing, implementing, reviewing, or testing Microsoft SQL Server
  or Azure SQL integration: T-SQL schema and migrations, drivers and ORMs,
  connection pooling, isolation and locking, indexing and execution plans,
  and secure configuration. Applies to any stack talking to SQL Server.
license: MIT-0
compatibility: Any language or framework with a SQL Server driver.
metadata:
  category: database
  engine: mssql
  aliases: sqlserver, t-sql, azure-sql
---

# Microsoft SQL Server Integration

Apply this skill whenever a stage touches SQL Server or Azure SQL: schema
design, migrations, queries, transactions, performance, or tests.

## When this skill applies

- Inception: choosing SQL Server/Azure SQL, designing the schema, or the
  contract.
- Construction: writing T-SQL, repositories, migrations, stored procedures.
- Construction: containerised tests, CI migration runs, fixtures.
- Operation: execution-plan and index review, blocking/deadlock triage.

## Configuration and secrets

- Connection string shape:
  `Server=host,1433;Database=db;User Id=...;Password=...;Encrypt=True;TrustServerCertificate=False;`
- Use `Encrypt=True` and validate the certificate; only set
  `TrustServerCertificate=True` in local development.
- Read credentials from environment/secret manager only.
- Prefer integrated/managed identity (Entra ID) over SQL logins where
  available on Azure SQL.
- Set `Application Name` per service for attributability.
- Create a least-privilege login/user; grant only required permissions. Avoid
  `db_owner` and `sysadmin` for applications.

## Drivers and ORMs (pick one per codebase and record it)

| Ecosystem | Driver | Common ORM/query layer |
| --- | --- | --- |
| Node/TypeScript | `mssql` (tedious) | Prisma, TypeORM, Kysely, Drizzle |
| Python | `pyodbc` / `pymssql` | SQLAlchemy, Django ORM |
| Java/Kotlin | JDBC (`mssql-jdbc`) | Hibernate/JPA, jOOQ, MyBatis |
| Go | `microsoft/go-mssqldb` | sqlc, GORM, Ent |
| .NET | `Microsoft.Data.SqlClient` | EF Core, Dapper |

Record the driver and version in `construction/nfr-requirements`, and the
pooling strategy in `construction/nfr-design`.

## Schema and data modelling

- Use `schema.Table` naming; keep application tables out of `dbo` only if you
  have a deliberate convention (many teams use `dbo` and are fine).
- Prefer `BIGINT IDENTITY` or `SEQUENCE`, or `UNIQUEIDENTIFIER` with
  `NEWSEQUENTIALID()` when client-generated IDs are required. Note that random
  `NEWID()` causes index fragmentation.
- Use `DATETIME2` (not `DATETIME`) for instants; store UTC and use
  `datetimeoffset` only when the zone must be preserved.
- Use `NVARCHAR`/`NCHAR` for text you accept from users; `VARCHAR` for
  constrained identifiers.
- Use `BIT` for booleans, `DECIMAL` for money (never `FLOAT`).
- Use `ROWVERSION` for optimistic concurrency where useful.
- Add `NOT NULL`, `CHECK`, and foreign keys by default. Choose and enforce a
  naming convention.
- Prefer `sysname`/`nvarchar(128)` for identifiers and be mindful of the
  900-byte index key limit on clustered keys.

## Migrations

Choose one tool and make every schema change a reviewed, forward-only
migration:

- EF Core Migrations (.NET), Fluent Migrator
- Flyway, Liquibase (language-agnostic)
- DbUp, RoundhousE
- DACPAC / `sqlpackage` (state-based; understand the generated diff)
- Prisma Migrate / Umzug / Knex (Node), Alembic (Python)

Rules:
- Migrations are immutable once merged. Fix forward.
- Separate schema changes from data backfills.
- Set `SET LOCK_TIMEOUT` and consider `ONLINE = ON` for index operations
  (Enterprise/Azure SQL; not available on Standard).
- Adding a nullable column is metadata-only; adding a `NOT NULL` column with a
  default on a large table can rewrite it — plan it.
- Avoid `MERGE` for upserts where concurrency matters; prefer explicit
  `INSERT`/`UPDATE` with the right isolation, or `MERGE` with `HOLDLOCK`.
- For destructive changes: expand → migrate → contract across releases.

## Query patterns and performance

- Index for the access pattern; SARGable predicates only (no functions or
  implicit conversions on indexed columns).
- Prefer covering indexes (`INCLUDE` columns) for hot reads.
- Read plans with the actual execution plan, `SET STATISTICS IO, TIME ON`, and
  `sys.dm_exec_query_stats` / `sys.dm_exec_sql_text`.
- Beware **parameter sniffing**: a cached plan built for atypical parameters can
  wreck a workload. Fix with `OPTION (RECOMPILE)`, `OPTIMIZE FOR`, or plan
  guides — deliberately.
- Watch for key lookups, scans on large tables, spills to tempdb, and
  implicit conversions.
- Enable Query Store to capture plan regressions over time.
- Watch tempdb contention (PFS/GAM/SGAM) and use `OPTIMIZE_FOR_SEQUENTIAL_KEY`
  where appropriate.
- Prefer keyset pagination with `OFFSET ... FETCH` only for bounded pages.

## Transactions and concurrency

- Default isolation is `READ COMMITTED`. Many production systems enable
  **RCSI** (`READ_COMMITTED_SNAPSHOT ON`) to avoid reader/writer blocking —
  know which mode you are in.
- `SNAPSHOT` isolation needs `ALLOW_SNAPSHOT_ISOLATION ON`.
- Keep transactions short; never hold one across a network or user wait.
- Use `UPDLOCK`/`ROWLOCK` hints deliberately; use `sp_getapplock` for
  application-level mutexes.
- Capture deadlock graphs (`system_health` Extended Events) and retry on
  deadlock (error 1205).

## Connection pooling

- Pool per process. .NET and drivers pool by default; tune
  `Max Pool Size`, `Min Pool Size`, and `Connect Timeout`.
- Size against the server's connection limits and other consumers.
- Enable a modest retry policy for transient faults (Azure SQL throttling,
  failovers) with exponential backoff.

## Security

- Parameterise every query. Never concatenate SQL or use `EXEC(@sql)` with
  user input; use `sp_executesql` with parameters for dynamic SQL.
- Prefer Windows/Entra integrated auth; rotate SQL logins if used.
- Use Always Encrypted for the most sensitive columns when appropriate.
- Enable Transparent Data Encryption / TDE where available.
- Audit `sysadmin`, `db_owner`, and `EXECUTE AS` usage.
- Never log credentials or PII-bearing result sets.

## Testing

- Use Testcontainers (`mcr.microsoft.com/mssql/server`) or LocalDB for local
  runs; use Azure SQL in CI only when you must test Azure-specific behavior.
- Run migrations in test setup so tests exercise the real schema.
- Reset state between tests (transaction rollback or `DELETE`/`TRUNCATE`);
  avoid order-dependent fixtures.
- SQL Server containers need a strong SA password and often ~2 GB RAM; account
  for startup time.

## Operations

- Track backups (full/differential/log), recovery model, and test restores.
- Monitor `sys.dm_os_wait_stats`, blocking chains, and tempdb usage.
- Prefer reversible migrations; for destructive changes use expand/contract.
- For Azure SQL, understand DTU/vCore throttling and geo-replication.

## Common pitfalls

- Assuming case-sensitive collations; know the database collation.
- Implicit conversions (e.g. `NVARCHAR` column vs `VARCHAR` literal) killing
  index seeks.
- `DATETIME` rounding to 3.33 ms and its 1753 minimum.
- Random `UNIQUEIDENTIFIER` keys fragmenting clustered indexes.
- `MERGE` race conditions without `HOLDLOCK`.

## How this skill plugs into my-aidlc

- Construction: record the edition (SQL Server vs Azure SQL) and hosting
  decisions in `construction/infrastructure-design`.
- Construction: record the compatibility level, driver/ORM, and migration tool
  as tech-stack decisions in `construction/nfr-requirements`, and the pooling
  strategy in `construction/nfr-design`.
- Construction: every schema change is a migration under
  `construction/code-generation`; note the tool and commands in
  `code-generation-plan`.
- Construction: integration tests use Testcontainers or LocalDB; CI runs
  migrations before `construction/build-and-test`.
- Operation: use Query Store and actual execution plans in
  `operation/performance-validation`.
