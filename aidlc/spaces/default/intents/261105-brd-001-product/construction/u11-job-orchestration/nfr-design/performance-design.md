# Performance Design — u11-job-orchestration

Sources: `nfr-requirements/performance-requirements.md`, `functional-design/functional-spec.md` WF1–WF2.

- **Tick budget.** The ticker is a `tokio::time::interval` at `JOB_ORCHESTRATION_TICK_SECONDS`
  (60 s default, allowed 30–90 s so the heartbeat age stays under 90 s; NFR2.1, NFR2.5). A tick
  performs only database reads/writes plus enqueueing; it never awaits a target call. The planning
  phase runs under a 30 s `tokio::time::timeout`; on expiry the tick logs a warning, exports the
  duration, and abandons the unfinished planning (the next tick resumes it).
- **Executor.** A bounded executor (32 in-flight, `tokio::Semaphore`) drains `pending` rows, at
  most 200 per tick. A row becomes `running` only after a permit is acquired, so a large backfill
  never claims rows it cannot start; a pending row waiting for a permit is untouched by the
  reaper. Each call uses a per-job `reqwest` timeout of `timeoutSeconds`, connect 1 s.
- **Lease margin.** A run lease is `timeoutSeconds + 30`; the reaper treats a row whose lease
  expired as failed, so a hung call cannot hold a window (NFR2.3).
- **Reads.** The two operator list queries are keyset-paginated and bounded to 500 rows; the
  `(job, scheduled_for desc)` index serves them and the due query (NFR2.4, NFR2.6).
- **Backfill.** Accepted windows are inserted in one `INSERT ... SELECT generate_series` statement
  and the response carries counts plus the covered-window count, not a full list (NFR2.7).
- **Heartbeat.** Only the lease owner writes the heartbeat, at tick start and end; a standby
  replica exports the shared row's age and never writes it (NFR2.5).
