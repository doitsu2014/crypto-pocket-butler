# Scalability Requirements — u10-analytics

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`, and
`inception/requirements-analysis/requirements.md`.

| ID | Requirement | Target |
|---|---|---|
| NFR20.1 | API scaling | Stateless, 2 replicas, ceiling 4 |
| NFR20.2 | Ingest volume | 1 M events/month average (≈ 0.4/s), peak ≤ 50/s for 1 minute |
| NFR20.3 | Storage | ≈ 1 M events/month plus ≈ 1 M ConsentCheck rows per 90 days; six-month events ≈ 6 M rows; aggregates ≤ 100 k rows; total ≤ 7.5 M rows |
| NFR20.4 | Aggregation | ≤ 200 k events per nightly run; one job at a time |
| NFR20.5 | Connection pool | 10 per API replica, 6 per worker |
| NFR20.6 | Retention sweep | Deletes events older than 6 months and ConsentCheck rows older than 90 days in batches of 10 k rows; ≤ 2 minutes nightly |
