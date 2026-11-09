# Functional Entities — u11-job-orchestration

Sources: `inception/units-generation/unit-of-work.md` (U11), `inception/requirements-analysis/requirements.md`
(FR10.1–FR10.3), and `inception/domain-design/components.md`; story US9.1 (AC9.1.1–AC9.1.3).

```yaml
unit: u11-job-orchestration
source_of_truth: inception/domain-design/components.md (Job Orchestration), amended here
entities: [JobDefinition, JobRun, OrchestratorLease, Heartbeat, BackfillRequest]
change_from_inception: >
  JobSchedule is merged into JobDefinition (a job owns one schedule); JobRun's schedule_id/error
  become job/detail with attempt, actor, trigger and next_attempt_at added; one row now exists per
  attempt, not per window. The tick source changes from Supabase Cron to an in-process ticker
  guarded by OrchestratorLease, because no unit may use a message queue and the orchestrator must
  own its own liveness signal.
```

## JobDefinition

A job the orchestrator runs.

| Field | Type | Rules |
|---|---|---|
| `name` | text | unique; one of the allow-listed job names (BR6.1) |
| `targetService` | text | the registered service whose run endpoint is called |
| `schedule` | text | `every:<minutes>` (1–60), `hourly:<minute>` (0–59), or `daily:<HH:MM>` UTC (BR1.1) |
| `timeoutSeconds` | int | 1–3,600 |
| `maxAttempts` | int | 1–5 (BR2.1) |
| `enabled` | bool | a disabled job is never scheduled |
| `lastScheduledFor` | timestamptz? | the newest window already scheduled |

## JobRun

One attempt at one scheduled window. A retry is a new row, never a mutation of a terminal one
(BR5.1).

| Field | Type | Rules |
|---|---|---|
| `id` | uuid | |
| `job` | text | the job name |
| `scheduledFor` | timestamptz | the window, canonical whole-second UTC; unique with `job` and `attempt` (BR1.2) |
| `attempt` | int | 1..=maxAttempts |
| `status` | text | `pending`, `running`, `succeeded`, `failed`, `skipped`, `abandoned` |
| `trigger` | text | `scheduled`, `missed`, `backfill`, `manual` (BR7.2) |
| `actorId` | uuid? | the requesting support/admin user for `backfill` and `manual`; absent for system triggers |
| `reason` | text? | the backfill/manual reason (≤200 chars, never a secret) |
| `owner` | text? | the instance holding the run lease while `running` |
| `leaseUntil` | timestamptz? | the run lease expiry (BR1.3) |
| `nextAttemptAt` | timestamptz? | when a `failed` row becomes retryable (BR2.1) |
| `startedAt` | timestamptz? | when the call began |
| `finishedAt` | timestamptz? | when the row reached a terminal state |
| `responseStatus` | int? | the target's HTTP status |
| `detail` | text? | a short non-secret reason (≤500 chars) |

## OrchestratorLease

One row, `name = 'job-orchestrator'`, `owner`, `leaseUntil` (TTL 30 s, renewed every 10 s). A tick
runs only while its instance holds it (BR1.3).

## Heartbeat

One row, `updatedAt` written by the tick loop at each tick's start and end. Its age is exported as
`job_orchestration_heartbeat_age_seconds`; an absent or stale metric is the dead-man condition
(BR4.1–BR4.2).

## BackfillRequest

An operator's request to re-run windows: `job`, `from`, `to` (range width ≤ 7 days, BR3.2),
`requestedBy`, `requestedAt`, `reason` (1–200 chars). Each accepted window becomes a `pending`
JobRun with trigger `backfill`.

## Window identity

The canonical `scheduledFor`: the largest boundary of the job's schedule at or before now, as
whole-second UTC RFC 3339. `every:N` is anchored to the Unix epoch (`floor(epoch_seconds / (N*60))`),
`hourly:M` to the hour with minute `M`, `daily:HH:MM` to that UTC time. The idempotency key is
`job:{job}:{scheduledFor}` with that exact text.
