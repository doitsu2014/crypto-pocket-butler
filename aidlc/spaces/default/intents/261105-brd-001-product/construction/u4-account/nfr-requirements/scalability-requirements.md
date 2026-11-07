# Scalability Requirements — u4-account

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`,
`inception/requirements-analysis/requirements.md`, and `inception/contract-design/contract-summary.md`.

## Targets

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR20.1 | Account scales horizontally, stateless | 3 replicas behind the gateway | NFR20 |
| NFR20.2 | Holdings reads scale with portfolio reads | indexed by account; snapshot rows bounded per run | NFR20 |
| NFR20.3 | Sync workers scale without duplicate runs | single-flight lease; extra workers poll | NFR20 |
| NFR20.4 | Provider rate limits protect the venues and RPCs | per-venue concurrency and backoff; launch capacity reviewed | NFR20 |

## Load projection

| Figure | Launch bound |
|---|---|
| Users | 10,000 |
| Accounts | ≤ 5 per user (50,000) |
| Peak concurrent syncs | 4 workers |
| C5 holdings read rate | 50 rps |
| Postgres pool | 12 per replica (36), 8 worker, headroom |

## Data growth and retention

- At launch: 10,000 users × ≤5 accounts × one daily sync ≈ 50,000 snapshots/day.
- `SyncRun` and `HoldingSnapshot` are retained 90 days and older rows are deleted by the worker;
  `ImportRowError` is retained with its batch for 90 days. The 90-day window caps the snapshot
  table at roughly 4.5 million rows.
- Deletion runs on the same scheduled sweep as the lease expiry.

## Scaling trigger

- C5 read p95 > 150 ms for 10 minutes, or the Postgres pool above 18 of 20 for 10 minutes, adds a
  replica; the behavioural ceiling is 4 replicas within the 60-connection budget.

## Concurrency

- One live `SyncRun` per account; a lease-expired run is failed and re-claimable.
- Provider calls are bounded per venue; a rate limit backs off and reports `rate_limit`.
- Imports stream rows; one import per account at a time.

NFR references: NFR20.1–NFR20.4.
