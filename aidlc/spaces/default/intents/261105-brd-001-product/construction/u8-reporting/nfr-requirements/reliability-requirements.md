# Reliability Requirements — u8-reporting

| ID | Requirement | Target |
|---|---|---|
| NFR1.1 | Brief availability | 99.9% of reads answered over 30 days |
| NFR1.2 | Explicit failure | a source failure marks the brief failed with a reason; a degraded source produces a generated partial/unavailable brief |
| NFR1.3 | Exactly one brief | unique (user, period, periodStart); a failed or hour-old pending brief is retried in place |
| NFR1.4 | Run | leased 1,800 s renewed every 60 s, one run, one retry after 10 minutes |
| NFR1.5 | Email | Reporting always submits a C9 `brief` alert on the `email` channel with key `brief:{briefId}:email`; submission failure is recorded and retried; no duplicate alert |
| NFR1.6 | Backup | Supabase regime, RPO ≤24 h, RTO ≤4 h |
| NFR1.7 | Health | `/healthz` liveness, `/readyz` Postgres only |
