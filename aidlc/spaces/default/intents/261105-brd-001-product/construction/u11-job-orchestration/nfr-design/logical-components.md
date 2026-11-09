# Logical Components — u11-job-orchestration

| Component | Responsibility |
|---|---|
| `schedule` (domain) | the schedule grammar and canonical window computation (BR1.1, BR1.4) |
| `job` (domain) | the job catalogue, trigger and status types, the retry planner and state transitions (BR2, BR5) |
| `run` (domain) | the JobRun record and its terminal-state rules (BR5.1) |
| `scheduler` (application) | tick planning: due windows, missed windows, pending drain, retries (WF1–WF2) |
| `executor` (application) | bounded outbound calls, response classification, outcome recording under the ownership check (WF1 step 6, WF2) |
| `reaper` (application) | expired-lease sweep (BR2.2) |
| `operator` (application) | job list, run pages, manual run and backfill queueing (WF3) |
| `lease` / `heartbeat` (infrastructure) | Postgres lease and heartbeat rows |
| `run repository` (infrastructure) | sqlx access to `job_runs`, `job_definitions` seeds, `backfill_requests` |
| `target client` (infrastructure) | reqwest call with timeout, idempotency key and trace propagation |
| `http` | operator routes, JWKS middleware, DTOs, RFC 7807 problems, metrics |

Dependencies point inward only: `http`/`infrastructure` → `application` → `domain`.
