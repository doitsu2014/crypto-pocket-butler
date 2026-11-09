# Business Rules — u11-job-orchestration

Sources: `requirements.md` (FR10.1–FR10.3, NFR17), `components.md`, and story US9.1
(AC9.1.1–AC9.1.3).

## BR1 — Scheduling, windows and single flight

- BR1.1: a schedule is `every:<minutes>` (1–60), `hourly:<minute>` (0–59), or `daily:<HH:MM>`
  UTC. Any other shape is refused at startup, never guessed.
- BR1.2: at most one JobRun exists per `(job, scheduledFor, attempt)`; a retry inserts attempt+1.
  A window is "queued or covered" when a `pending` row exists, a `running` row holds a live
  lease, a `failed` row has a future `nextAttemptAt`, or any `succeeded`, `skipped` or `abandoned`
  row exists; a queued-or-covered window is never enqueued again (the insert is
  `ON CONFLICT (job, scheduled_for, attempt) DO NOTHING` as a second guard).
- BR1.3: one OrchestratorLease row (TTL 30 s, renewed every 10 s by a background task) grants the
  right to schedule. A write that records a run outcome succeeds only when the writer still owns
  the run's lease; a lost lease makes the outcome non-authoritative and the run reaper retries it.
- BR1.4: the canonical `scheduledFor` and the idempotency key text are defined in entities.md;
  two computations of the same window always produce the same key.

## BR2 — Bounded retries

- BR2.1: a `failed` row carries `nextAttemptAt`, one schedule window later, or 5 minutes for
  `every:` schedules shorter than 10 minutes. The tick drains every `failed` row whose
  `nextAttemptAt` has passed and whose attempts remain.
- BR2.2: a run whose lease expires while `running` is marked `failed` by the reaper; the same
  retry rule applies.
- BR2.3: attempts exhausted → the newest row is `abandoned`. Every failed attempt also exports
  `job_orchestration_job_failures_total`, and the platform alert rule fires on that counter (the
  external alerting stack; C9 addresses users, not operators).

## BR3 — Missed runs and backfill

- BR3.1: a window that passed without a run is scheduled once when it is inside the 24-hour
  lookback, oldest first, so recovery order is deterministic and a long outage never bursts.
- BR3.2: a backfill accepts `from`..`to` whose width is at most 7 days; windows already covered
  are reported as covered, never re-run.
- BR3.3: a backfill records the requesting user, the instant, and the reason; none is a credential.

## BR4 — Heartbeat and dead-man check

- BR4.1: only the instance holding OrchestratorLease writes the heartbeat, at tick start and end;
  a standby replica exports the shared row's age without writing it. A wedged scheduler therefore
  stops the heartbeat while its standby still reports the same stale age.
- BR4.2: the rule is `absent(job_orchestration_heartbeat_age_seconds) or
  job_orchestration_heartbeat_age_seconds > 180`, evaluated every 30 s over a 15 s scrape with
  `for: 90s`, so a stopped or wedged orchestrator pages within five minutes (AC9.1.2).

## BR5 — Run records

- BR5.1: terminal statuses are `succeeded`, `skipped`, and `abandoned`; a `failed` row is
  terminal for that attempt but points to its retry through `nextAttemptAt`. No terminal row is
  ever mutated; a retry is a new row (BR1.2).
- BR5.2: `detail`, `reason`, and audit context carry identifiers and statuses only: no token, no
  credential, no response body.

## BR6 — Target allow-list and call shape

- BR6.1: only the jobs in the catalogue below are defined; each names its owning provider. The
  catalogue is fixed at startup (no admin job-creation API).
- BR6.2: every call is `POST {service_url}/v1/internal/run/{jobName}` with body `{"owner": "..."}`
  and header `idempotency-key: job:{job}:{scheduledFor}`; the orchestrator's service token
  authorises it at the target, and the target must apply one window once for that key (AC9.1.3).

## BR7 — Who may act, and how runs are triggered

- BR7.1: the service validates the forwarded Supabase JWT against JWKS itself and enforces the
  `admin`/`support` role on every operator path; the gateway adds rate limits.
- BR7.2: every run records `trigger` (scheduled, missed, backfill, manual), and backfill/manual
  runs record the actor and the reason. A manual run of a window already covered is refused 409
  with the covering run's id.

## Catalogue

| Job | Provider |
|---|---|
| `account-sync` | Account (per-account lease inside the provider) |
| `market-data-refresh` | Market Data |
| `market-data-fx` | Market Data (the FR10.1 FX job) |
| `portfolio-valuation` | Portfolio |
| `portfolio-snapshot` | Portfolio |
| `ledger-reconcile` | Ledger |
| `ledger-ingest` | Ledger |
| `analytics-aggregate` | Analytics |
| `analytics-flag-expiry` | Analytics |
| `notification-delivery` | Notification |
| `notification-digest` | Notification |
| `brief-generation` | Reporting |
| `audit-privacy-sweep` | Identity |
| `audit-retention` | Audit |
