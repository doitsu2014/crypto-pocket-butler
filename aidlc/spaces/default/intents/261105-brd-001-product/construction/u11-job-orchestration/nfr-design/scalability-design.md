# Scalability Design — u11-job-orchestration

Sources: `nfr-requirements/scalability-requirements.md`.

- **Deployment shape.** One `job-orchestration` image, one Swarm service, 2 replicas. The
  OrchestratorLease (`name`, `owner`, `leaseUntil`, TTL 30 s, renewed every 10 s) elects the
  scheduler; the other replica serves reads and answers probes (NFR20.1).
- **Failover.** A stopped scheduler's lease expires within 30 s; the survivor claims it on its
  next tick (≤ 60 s), so work resumes ≤ 90 s (NFR1.10, NFR20.1).
- **Storage.** Run rows carry `(job, scheduled_for desc)`, `(status, next_attempt_at)`, and
  `(job, scheduled_for, attempt)` unique. 14 jobs × 1,440 windows/day ≈ 20,300 rows/day;
  400 days ≈ 8.1M rows within the Supabase plan (NFR20.2–NFR20.3).
- **Retention.** Run rows are operational records (not financial records, not audit events): the
  400-day window covers a full annual cycle of drift debugging, is cheaper than keeping them
  forever, and is longer than the six-month audit-log class NFR14 names for audit records.
- **Pruning.** The on-call runbook's nightly operator job (02:00 UTC on a Swarm manager) runs
  `job-orchestration-migrate --prune-runs` with the migrator DSN (the runtime role has no DELETE)
  and stores the epoch seconds of the successful prune in `job_orchestration_prune_state`. The
  service reads that row and exports `job_orchestration_last_prune_timestamp`; the alert uses
  `absent(...) or time() - value > 129600` with a 24 h deployment grace, so a never-pruned
  deployment still alerts (NFR20.3).
- **Backfill bound.** Width ≤ 7 days; at `every:1` this is 10,080 rows inserted in one statement
  (NFR20.4).
- **Pooling.** One process, one pool of 10 connections shared by the operator routes, the tick and
  the executor; the role's Supabase connection limit (60) leaves room for 2 replicas plus the
  migrate job (NFR20.5).
