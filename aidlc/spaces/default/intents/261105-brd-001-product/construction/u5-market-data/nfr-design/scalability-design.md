# Scalability Design — u5-market-data

Inputs: `nfr-requirements/scalability-requirements.md` and `functional-design/functional-spec.md`.

## Scaling

- Stateless API: 2 replicas at launch, ceiling 4; scale on p95 > 250 ms for 10 minutes.
- Two worker jobs (top100, tail) run as separate claimed runs with per-job leases; a new worker
  replica competes on the same lease and does not double a job.
- Postgres pool: 12 per API replica, 8 per worker.

## Load distribution

| Figure | Launch bound |
|---|---|
| Canonical assets | 2,000 (top 100 + tail) |
| `getPrices` | 50 rps at 100 assets average |
| top100 job | Every 5 min, ≤ 3 min |
| tail job | Every 10 min, ≤ 10 min |
| Source concurrency | 4 per job |
| Rows | ≈ 27 M observations plus FX readings at a 5/10-minute cadence with 90-day pruning |

## Provider rate limits

- A shared token bucket per source lives in `market_data.source_rate_bucket`, updated with
  `UPDATE ... RETURNING` under a row lock, so both jobs and all replicas share one budget.
- A 429 records `rate_limit` and defers the asset to the next run.

## Data growth

- Observations and runs are pruned at 90 days, except the newest observation per asset and source.
- The FX table stores one reading per pair per refresh; pruning keeps the newest per pair.

NFR references: NFR20.1–NFR20.6.
