# Performance Design — u10-analytics

Inputs: `nfr-requirements/performance-requirements.md`, `functional-design/functional-spec.md`.

## Ingest path

- One C3 `getConsent` call (80 ms timeout, one attempt) for analytics-class events, then one
  `INSERT` into `analytics.events` and one `INSERT` into `analytics.consent_checks`.
- Index: `(event_name, occurred_at)` for aggregation; `(user_ref)` for erasure.
- No synchronous aggregation work on the ingest path.

## Flag path

- One indexed lookup by `key`, one SHA-256 of the stable userRef, in-memory bucket.
- Flags cached in process for 30 seconds (public configuration, not user data). An admin change
  can take up to 30 seconds per replica and replicas may briefly disagree; this is accepted.

## Aggregation

- One SQL pass over the closed period with `GROUP BY` per metric; upserts into
  `metric_aggregates` keyed by `(metric, period, cohort, period_start)`.
- Suppression is computed from `COUNT(DISTINCT user_ref)` in the same pass.
- Nightly run target ≤ 2 minutes for ≤ 200 k events.

## Budgets

| Operation | Budget | Design element |
|---|---|---|
| Ingest | p95 ≤ 120 ms | C3 80 ms timeout + single insert |
| Flag | p95 ≤ 30 ms | Index + 30 s cache |
| Aggregate read | p95 ≤ 150 ms | Index on `(metric, period, period_start)` |
| Nightly aggregation | ≤ 2 min | One grouped pass, batched upsert |
| Erase | ≤ 30 s | Indexed delete by userRef |

NFR references: NFR2.1–NFR2.5.
