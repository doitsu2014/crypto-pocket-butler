# Scalability Requirements — u9-notification

| ID | Requirement | Target |
|---|---|---|
| NFR20.1 | API scaling | 2 replicas, ceiling 4 |
| NFR20.2 | Volume | ≤ 500k alerts/month; ≤ 100k preferences |
| NFR20.3 | Storage | alerts and attempts retained 12 months; ≤ 30 M rows |
| NFR20.4 | Delivery | one run at a time, batches of 200, provider rate limit respected |
| NFR20.5 | Pool | 10 per API replica, 8 per worker |
| NFR20.6 | Retention sweep | alerts/attempts > 12 months deleted nightly |
