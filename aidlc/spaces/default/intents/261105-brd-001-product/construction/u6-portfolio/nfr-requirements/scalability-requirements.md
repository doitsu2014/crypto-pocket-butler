# Scalability Requirements — u6-portfolio

| ID | Requirement | Target |
|---|---|---|
| NFR20.1 | API scaling | 2 replicas, ceiling 4 |
| NFR20.2 | Volume | 10k users, ≤ 50k accounts, ≤ 20k portfolios, ≤ 5 targets/portfolio |
| NFR20.3 | Snapshots | one per portfolio per hour ⇒ ≤ 480k/day; two-year retention; at 20k portfolios this is ≈ 350M rows with frozen inputs, so inputs are stored once per snapshot in a compressed JSONB column and pruned at two years |
| NFR20.4 | Valuation run | batched 500 portfolios, one leased run |
| NFR20.5 | Pool | 12 per API replica, 8 per worker |
| NFR20.6 | Retention | snapshots two years, then pruned |
