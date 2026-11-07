# Scalability Requirements — u5-market-data

Inputs: `functional-design/functional-spec.md`, `inception/requirements-analysis/requirements.md` (NFR5, launch bound), and C6.

| ID | Requirement | Target |
|---|---|---|
| NFR20.1 | API scaling | Stateless, 2 replicas at launch, scale to 4 on p95 > 250 ms for 10 minutes |
| NFR20.2 | Refresh scaling | One run per job; bounded concurrency of 4 source calls and 4 assets in flight per job |
| NFR20.3 | Provider rate limits | Per-source `rateLimitPerMinute` from C4 is never exceeded across both jobs and all replicas, enforced by a shared token-bucket row in Postgres; a 429 waits or defers the asset |
| NFR20.4 | Data growth | ≤ 2,000 canonical assets; observations pruned to 90 days except the newest per asset and source. Market data is public data, not a financial record, so the two-year financial-retention rule does not apply. Steady state ≈ 27 M observation rows at a 5-minute top-100 and 10-minute tail cadence, before FX readings |
| NFR20.5 | Connection pool | API 12 connections per replica, worker 8 |
| NFR20.6 | Batch size | Price requests ≤ 1,000 asset ids; larger batches are refused with 422 |

## Notes

- The read path serves from Postgres only; a new API replica adds read capacity without
  coordination.
- The per-job single-flight lease bounds provider load by design; two jobs can run concurrently
  but each honours the per-source rate limit.
