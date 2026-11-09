# Scalability Design — u10-analytics

Inputs: `nfr-requirements/scalability-requirements.md`.

- API: 2 stateless replicas, ceiling 4; pool 10 each.
- Worker: 1 replica for aggregation and retention; pool 6.
- Volume: 1 M events/month (≈0.4/s average), peak 50/s for 1 minute.
- Storage: ≈6 M events + ≈1 M ConsentCheck rows (90-day) + ≤100 k aggregates ≤ 7.5 M rows.
- Aggregation: one nightly run, ≤200 k events, one job at a time with a lease.
- Retention: batched deletes of 10 k rows; events > 6 months, ConsentCheck > 90 days.
- Erase runs synchronously in the API request (no queue): one indexed delete per retained salt
  version, ≤ 30 s for 100 k events.
- Aggregation scans the closed day for daily metrics plus a 30-day lookback for day-7/day-30
  retention; both use `(event_name, occurred_at)` and are bounded by the 200 k-event cap.
- Indexes: `(event_name, occurred_at)`, `(user_ref)`, `(metric, period, period_start)`, `(key)`.

NFR references: NFR20.1–NFR20.6.
