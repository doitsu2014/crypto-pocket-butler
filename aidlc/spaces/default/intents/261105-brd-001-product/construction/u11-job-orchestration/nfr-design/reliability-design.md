# Reliability Design — u11-job-orchestration

Sources: `nfr-requirements/reliability-requirements.md`, `functional-design/functional-spec.md`.

- **Single scheduler.** The lease row is claimed with
  `UPDATE ... WHERE lease_until <= now() OR owner = $me RETURNING`, so exactly one instance
  schedules (BR1.3). Renewal failure stops scheduling; the executor still finishes in-flight calls
  but an outcome write checks ownership first and is discarded when the lease moved.
- **Reaper.** Each tick marks `running` rows with `lease_until <= now()` as `failed`
  (`detail = "lease expired"`, `next_attempt_at` set), then the normal retry rule applies
  (NFR1.8).
- **Retry planner.** A failed row becomes retryable at `scheduled_for + one schedule window`
  (or +5 min for `every:<10`). The tick selects failed rows whose `next_attempt_at <= now()` and
  `attempt < max_attempts`, and inserts attempt+1 as `pending`. Exhaustion → `abandoned`
  (NFR1.5, BR2.1–BR2.3).
- **Missed windows.** For each job, windows in `(now - 24 h, now)` with no covering row are
  queued oldest-first. Windows outside the lookback increment
  `job_orchestration_windows_unrecovered_total{job}` and the alert names the range (NFR1.6–NFR1.7).
- **Exactly once.** Every call carries `idempotency-key: job:{job}:{scheduledFor}`. The provider
  stores the outcome per key (C11 amendment): a duplicate whose recorded outcome is `succeeded` or
  `skipped` returns it without re-running; a duplicate whose recorded outcome is `failed`
  re-executes the window and replaces the outcome. A retry after a target-reported failure is
  therefore executable, and a succeeded side effect is applied once even when the response was
  lost (NFR1.8).
- **Queueing.** A window is queued-or-covered by a `pending` row, a live `running` lease, a
  `failed` row with a future `nextAttemptAt`, or a terminal `succeeded`/`skipped`/`abandoned` row.
  Inserts use `ON CONFLICT (job, scheduled_for, attempt) DO NOTHING`, so a tick that races another
  never fails and never duplicates a queued attempt.
- **Unrecovered windows.** Each job keeps a watermark (`last_scheduled_for`). A watermark older
  than `now - 24 h` means windows were dropped: the tick increments
  `job_orchestration_windows_unrecovered_total{job}` once per detected gap, logs the exact
  `from`/`to` range, and the alert rule fires on the counter; the operator reads the range from
  the log entry and a runbook backfill (NFR1.7).
- **Readiness.** `/readyz` runs `SELECT 1` and a `_sqlx_migrations` count; no target is polled
  (NFR1.9).
- **Restart.** On start the instance claims the lease when free and runs a tick immediately, then
  every 60 s; a seeded missed window is picked up by that first tick (NFR1.10).
