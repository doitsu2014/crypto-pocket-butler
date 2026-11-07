# Logical Components — u5-market-data

Inputs: `nfr-requirements/*`, `functional-design/functional-spec.md`, and C4/C6/C10/C11.

| Logical component | Responsibility | Scaling | Failure domain |
|---|---|---|---|
| Market Data API | C6 prices and FX, the C11 run endpoint (runs execute inline in the receiving replica under the per-job lease), health and metrics | 2 replicas, ceiling 4 | One task; gateway routes around |
| Price selector | Priority selection, freshness classification, degraded mode | In-process | A bad request fails that response |
| Refresh worker | Per-job lease sweep and 90-day retention; C11 runs themselves execute in the API replica that receives the call | 1 replica for maintenance | A crashed worker delays the lease sweep and retention, not an in-flight API run |
| `market_data` schema | Observations, FX, runs, tiers, service tokens, rate buckets | Supabase managed | Service-owned schema |
| C4 client | Source list, canonical assets and rank | Per request/run | Degraded read or failed run |
| Vault function | Provider key read via `SECURITY DEFINER` | Per source call | A failed read fails the source call |
| Audit client | C10 failed-run events | Per failure | Logged and retried |
| Job Orchestration | C11 caller | — | 409/202 handling |

## Boundaries

- No portfolio, holding, or valuation data is stored here.
- The source registry and canonical assets are C4 data; Market Data keeps no copy.
- No message queue; the two jobs are claimed from `refresh_runs`.
- C11 runs execute inline in the API replica that receives the call, fenced by the per-job lease;
  the worker owns the lease sweep and retention.

## Blast radius

- A failed API task affects its replica only; the other replica keeps serving.
- A failed source affects the assets that source covers; fallback or stale applies.
- A compromised service token exposes only the C6 reads its allow-list grants.
- An unreachable C4 degrades reads to stale and fails refresh runs.

NFR references: NFR20.1, NFR1.1, NFR9.2.
