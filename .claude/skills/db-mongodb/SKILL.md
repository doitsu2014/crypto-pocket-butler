---
name: db-mongodb
description: >
  Use when designing, implementing, reviewing, or testing MongoDB integration:
  document schema design, embed vs reference, indexes and aggregation
  pipelines, drivers/ODMs, transactions, sharding, validation, and secure
  configuration. Applies to any stack talking to MongoDB.
license: MIT-0
compatibility: Any language or framework with a MongoDB driver.
metadata:
  category: database
  engine: mongodb
  aliases: mongo, document-db, atlas
---

# MongoDB Integration

Apply this skill whenever a stage touches MongoDB: document modelling, indexes,
queries and aggregation, transactions, performance, or tests.

## When this skill applies

- Inception: choosing MongoDB, modelling documents, or defining the contract.
- Construction: writing queries, aggregation pipelines, repositories,
  migrations.
- Construction: containerised/replica-set tests, CI, fixtures.
- Operation: index and aggregation review, slow-query triage, sharding.

## Configuration and secrets

- Connect with a single URI:
  `mongodb://user:pass@host:27017/db?retryWrites=true&w=majority` or
  `mongodb+srv://...` for Atlas.
- Read credentials from environment/secret manager only. Never commit them.
- Always set `retryWrites=true`; set `w=majority` for durability-sensitive
  workloads.
- Use `authSource` explicitly when the user lives in the `admin` database.
- Require TLS (`tls=true`) for any remote or managed deployment.
- Use a least-privilege role per service; avoid `root`, `readWriteAnyDatabase`,
  and `dbAdminAnyDatabase` for applications.
- Set `serverSelectionTimeoutMS`, `connectTimeoutMS`, `socketTimeoutMS`, and a
  bounded `maxPoolSize`.

## Drivers and ODMs (pick one per codebase and record it)

| Ecosystem | Driver | Common ODM/query layer |
| --- | --- | --- |
| Node/TypeScript | `mongodb` | Mongoose, Prisma (Mongo connector) |
| Python | `pymongo` / `motor` (async) | Beanie, ODMantic, MongoEngine |
| Java/Kotlin | `mongodb-driver-sync` | Spring Data MongoDB |
| Go | `mongo-go-driver` | — (native) |
| .NET | `MongoDB.Driver` | — (native) |

Record the driver/ODM and version in `construction/nfr-requirements`, and the
pooling strategy in `construction/nfr-design`.

## Document schema design

- Model around how you **read**, not how you normalise. Embed data that is
  read together and updated together; reference data that grows unbounded or is
  shared.
- Respect the **16 MB** document limit. Never let an unbounded array grow inside
  a document; use a separate collection or the bucket pattern.
- The bucket pattern (time-series, one document per time bucket) and the
  subset pattern (hot fields embedded, cold fields referenced) solve common
  scaling problems.
- Aim for **single-document atomicity** to be sufficient for your invariants;
  MongoDB guarantees atomic writes per document.
- Denormalise counts/summaries deliberately and define how they stay
  consistent.
- Enforce shape with `$jsonSchema` collection validation in production.
- Use consistent field naming (`camelCase` or `snake_case`); define `_id` type
  deliberately (ObjectId is default; custom string keys are fine if indexed).

## Indexes

- Index every query shape you rely on; unindexed queries are collection scans.
- Follow the **ESR rule** for compound indexes: Equality fields, then Sort
  fields, then Range fields.
- Index order matters and the leftmost-prefix rule applies.
- Use:
  - `partial` indexes for hot subsets,
  - `TTL` indexes for expiring data,
  - `sparse`/`unique` for optional uniqueness,
  - `text` indexes for search (or Atlas Search for real full-text),
  - multikey indexes for array fields (one index entry per array element).
- Verify with `explain("executionStats")`: look for `IXSCAN` vs `COLLSCAN`,
  `totalKeysExamined` vs `nReturned`, and `docsExamined`.
- Drop unused indexes; each one costs writes and space. Check
  `$indexStats`.

## Query and aggregation patterns

- Filter and sort in the database; never fetch everything and filter in the
  application.
- Put `$match` and `$sort` early in an aggregation pipeline so an index can be
  used; `$project` only needed fields.
- Avoid `$where`, `$expr` on unindexed fields, and JavaScript execution.
- Prevent operator injection: never pass user input directly as a query object
  (`{ $gt: "" }`). Validate and coerce types; use driver sanitisation.
- Use projections to limit returned fields; avoid returning large documents.
- Use `findOneAndUpdate`/`updateOne` with `$set`/`$inc` rather than
  read-modify-write, to avoid lost updates.
- Use `bulkWrite` for batches; use `ordered: false` only when order is truly
  irrelevant.

## Transactions and concurrency

- Multi-document transactions require a **replica set or sharded cluster**
  (Atlas qualifies; a standalone `mongod` does not). Do not design around
  transactions when a standalone is the deployment target.
- Transactions have a 60-second default lifetime; keep them short.
- Prefer single-document atomic updates and idempotent operations; use
  optimistic concurrency via a version field where needed.
- Use `findOneAndUpdate` with a filter that encodes the precondition.

## Connection pooling and topology

- Pool per process; size `maxPoolSize` from the server's connection limits.
- Use `retryWrites`/`retryReads` for transient failures.
- Read preference (`primary`, `primaryPreferred`, `secondary`,
  `nearest`) is a consistency decision — document it.
- Write concern (`w`) is a durability decision — document it.
- For sharded clusters, choose a shard key with high cardinality, low
  frequency, and non-monotonic distribution; understand that shard keys are
  expensive to change.

## Security

- Never build queries by concatenating user input; validate types and use
  driver-level query sanitisation to block operator injection.
- Encrypt in transit (TLS) and at rest (managed encryption or CSFLE for
  field-level).
- Use field-level encryption (CSFLE/Queryable Encryption) for PII where
  required.
- Audit users, roles, and `dbAdmin` grants.
- Never log full connection strings or documents containing PII.

## Testing

- Use Testcontainers (`mongo`) or an ephemeral replica set for integration
  tests. If you test transactions, the harness MUST run as a replica set.
- Seed deterministic data and reset between tests; avoid order dependence.
- Test index usage with `explain`, not just result correctness.
- For schema validation, test that invalid documents are rejected.

## Migrations and evolution

- Document shape changes are migrations too. Common tools:
  `mongock`, `migrate-mongo`, Mongoose migration scripts, or a
  versioned script runner you own.
- Migrations must be idempotent and forward-only; include a backfill strategy
  and a way to resume after partial failure.
- Keep old and new shapes co-existing during a transition; add the reading
  path first, then backfill, then remove the old path.
- Never rely on "schemaless" as an excuse for undocumented evolution.

## Operations

- Monitor: slow queries (`db.currentOp`, profiler, Atlas Performance Advisor),
  replication lag, oplog window, connections, page faults, cache eviction.
- Back up with `mongodump`/`mongorestore` or managed snapshots; test restores.
- Prefer reversible migrations; for destructive changes use expand/contract.

## Common pitfalls

- Unbounded embedded arrays hitting the 16 MB limit.
- `COLLSCAN` on large collections because the compound index order violates ESR.
- Designing around multi-document transactions on a standalone deployment.
- Operator injection from unsanitised user input.
- Treating MongoDB as relational, then fighting `$lookup` for everything.
- Monotonic shard keys (timestamps) creating a single hot shard.

## How this skill plugs into my-aidlc

- Inception: record validation rules in `inception/domain-design`.
- Construction: record the deployment topology (standalone/replica set/sharded)
  in `construction/infrastructure-design`; record the driver/ODM as a
  tech-stack decision in `construction/nfr-requirements`, and the read/write
  concerns in `construction/nfr-design`.
- Construction: document the reading path, backfill, and cleanup order for
  shape changes under `construction/code-generation`; note commands in
  `code-generation-plan`.
- Construction: integration tests use a replica set via Testcontainers; CI
  runs migrations before `construction/build-and-test`.
- Operation: use `explain("executionStats")` and profiler evidence in
  `operation/performance-validation`.
