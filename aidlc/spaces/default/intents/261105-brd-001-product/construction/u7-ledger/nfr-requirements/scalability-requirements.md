# Scalability Requirements — u7-ledger

| ID | Requirement | Target |
|---|---|---|
| NFR20.1 | API scaling | stateless, 2 replicas, ceiling 4 |
| NFR20.2 | Transaction volume | ≤ 1 M transactions/month across users; ≤ 50k per user at launch |
| NFR20.3 | Cost-basis lots | ≤ 500k open lots; FIFO query indexed by (user, asset, acquired_at) |
| NFR20.4 | Reconcile | one nightly run per partition; ≤ 10 minutes for 50k transactions |
| NFR20.5 | Connection pool | 12 per API replica, 8 for the worker |
| NFR20.6 | Retention | transactions and corrections kept two years as financial records, then anonymised (user id replaced) and retained; P&L snapshots 2 years |

## Notes

- At 1 M transactions/month with 2-year snapshots the expected row count is ≤ 24 M; indexes are
  partitioned by user hash if needed.
