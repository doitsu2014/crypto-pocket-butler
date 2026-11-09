# Scalability Requirements — u11-job-orchestration

| ID | Requirement | Measure |
|---|---|---|
| NFR20.1 | One `job-orchestration` Swarm service runs 2 replicas of the same image; the OrchestratorLease (TTL 30 s) elects one scheduler, and the other serves reads only. Failover after a stop is ≤ 90 s (lease TTL 30 s + one tick), matching NFR1.10 | Stack config; lease test |
| NFR20.2 | The due query scales to 14 jobs × 1,440 windows/day with `(job, scheduled_for desc)` | Query plan check |
| NFR20.3 | Run rows are retained 400 days (≈ 8.1M rows) and pruned by the platform's nightly operator job at 02:00 UTC: the on-call runbook's scheduled task runs `job-orchestration-migrate --prune-runs` with the migrator DSN. The job writes `job_orchestration_last_prune_timestamp`; an alert fires when it is older than 36 h | Runbook + metric |
| NFR20.4 | Backfill is bounded to a 7-day width per request | Validation test |
| NFR20.5 | One process holds one pool of 10 connections shared by the operator routes, tick and executor (2 replicas = 20 against the role's 60 limit) | Configuration constant |
