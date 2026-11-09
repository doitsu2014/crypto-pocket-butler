# Performance Requirements — u5-market-data

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`,
`inception/requirements-analysis/requirements.md` (NFR5), and `inception/contract-design/contract-summary.md` (C6).

| ID | Requirement | Target | Measurement |
|---|---|---|---|
| NFR2.1 | `getPrices` for up to 1,000 asset ids, including the C4 source-list read | p95 ≤ 250 ms, p99 ≤ 600 ms | Server histogram over 30 days |
| NFR2.2 | `getFxRate` | p95 ≤ 100 ms | Server histogram over 30 days |
| NFR2.3 | Refresh runs | top100 job ≤ 3 minutes; tail job (≤ 2,000 assets) ≤ 10 minutes; the top100 job starts every 5 minutes | Run duration by job |
| NFR2.4 | Newest accepted observation for a 1,000-asset batch | one query per source using `DISTINCT ON (asset_id) ... ORDER BY observed_at DESC`; total p95 ≤ 100 ms for the batch | Query histogram |
| NFR2.5 | Per-source call | 1 s connect / 5 s read / 30 s total, 3 attempts | Adapter configuration |
| NFR2.6 | FX freshness | 99% of requested pairs within 1 hour | `market_data_fx_age_seconds` |
| NFR2.7 | Price freshness | Top-100 ≤ 5 min; tail ≤ 15 min from source `observedAt` | `market_data_price_age_seconds` |

The read path never calls a price provider: `getPrices` and `getFxRate` read stored observations
only. They do read the enabled source list from C4 on each request (BR1.4 forbids a cached copy),
which is why NFR2.1's budget includes that call.

## Notes

- Requests above 1,000 asset ids are refused with 422 rather than served slowly; C6 declares the
  bound and the 422 response.
- The freshness targets are hard bounds, not averages: a quote older than its tier bound is served
  `stale`.
- Two jobs exist: `top100` (5-minute cadence, ≤ 3-minute run) and `tail` (15-minute cadence,
  ≤ 10-minute run), each with its own lease. The top100 job's 3-minute budget keeps the 5-minute
  freshness bound reachable.
