# Performance Design — u4-account

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`,
`scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`,
`tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

## Read path

- C5 holdings: one indexed query by `accountId` over the latest `SyncRun`/`HoldingSnapshot`, plus
  the account row. No provider call on the read path.
- Account list: one indexed query by `userId`.
- Manual re-sync: returns 202 with the active run id within 200 ms; the first sync starts
  immediately on add.
- Support metadata read: the C3 grant check is cached for the request; if not cached the read returns
  a pending status rather than blocking.

## Sync path

- One run per account; the run reads the venue/RPC, resolves tokens (C4, batched), and writes the
  snapshot in one transaction.
- Provider timeouts and retries are bounded (1 s connect / 5 s read / 30 s total, 3 attempts); the
  full budget is in the NFR requirements.
- The worker processes accounts with bounded concurrency (4 at launch) and backoff per venue.

## Import path

- CSV rows stream through a parser with a per-format column map; memory is bounded by the batch
  size (500 rows), not the file size. Budget: 10,000 rows under 60 seconds, measured as
  parse + dedupe + insert.
- `ImportedTransaction` writes are batched; `dedupeKey` uniqueness is enforced by an index.

## Caching

- No cross-request cache of holdings, credentials, or provider responses.
- Chain metadata and the C4 supported-chains read may be cached for 5 minutes (public data only).

## Frontend interaction

- The SPA polls account status and last-synced time; a long sync shows in-progress and then the
  result. The ports are the C1 DTOs.

NFR references: NFR2.1, NFR2.2, NFR2.3, NFR4.1, NFR6.1, NFR16.1, NFR17.1, NFR17.2, NFR17.3,
NFR17.4, NFR17.5.
