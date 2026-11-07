# Performance Design — u5-market-data

Inputs: `nfr-requirements/performance-requirements.md`, `functional-design/functional-spec.md`, and
C6.

## Read path

- `getPrices`: one query per enabled source using
  `DISTINCT ON (asset_id) ... WHERE asset_id = ANY($1) AND status = 'accepted' ORDER BY asset_id, observed_at DESC`,
  then selection in memory by source priority and age. The C4 source list is read once per request.
- `getFxRate`: one indexed lookup on `(base, quote)` for the newest reading; a cross rate adds two
  lookups.
- No provider call on the read path. The C4 call and Postgres queries are the whole budget
  (NFR2.1, NFR2.2).
- Indexes: `(asset_id, source_name, observed_at DESC)` on observations, `(base, quote, observed_at
  DESC)` on FX, `(job, status, lease_until)` on runs.

## Refresh path

- Two jobs: `price-refresh-top100` (every 5 min, ≤ 3 min) and `price-refresh-tail` (every 10 min,
  ≤ 10 min).
- Each job processes assets with 4 in flight and 4 source calls in flight per source;
  providers use 1 s connect / 2 s read / 5 s total with 2 attempts, assuming p95 source latency of
  300 ms. A run deadline of 180 s (top100) or 600 s (tail) bounds the worst case, and a per-source
  circuit breaker skips a slow provider rather than exhausting the deadline.
- Prices and rates are stored as `rust_decimal` numeric columns; no float type is used.
- One accepted observation is stored per asset per run; the next source is tried only on failure.
- The top100 job owns the tier refresh and the FX refresh; the tail job only prices its slice.

## Budgets

| Operation | Budget | Design element |
|---|---|---|
| `getPrices` 1,000 assets | p95 ≤ 250 ms | One query per source + C4 read |
| `getFxRate` | p95 ≤ 100 ms | Indexed lookup, cross adds two |
| top100 run | ≤ 3 min | 100 assets, concurrency 4 |
| tail run | ≤ 10 min | ≤ 1,900 assets, concurrency 4 |

NFR references: NFR2.1, NFR2.2, NFR2.3, NFR2.4, NFR2.5.
