# Scalability Requirements — u8-reporting

| ID | Requirement | Target |
|---|---|---|
| NFR20.1 | API scaling | 2 replicas, ceiling 4 |
| NFR20.2 | Volume | 10k daily + 10k weekly ≈ 4.2M briefs/year at the launch bound |
| NFR20.3 | Storage | briefs retained two years ≈ 8.4M rows (bounded ≤ 9M), sections JSONB ≈ 2 KB each |
| NFR20.4 | Run | 200 users per batch, 8 batches × 4 in flight = 32 concurrent, one leased run |
| NFR20.5 | Pool | 10 per API replica, 8 per worker |
| NFR20.6 | Retention sweep | briefs > 2 years pruned nightly |
