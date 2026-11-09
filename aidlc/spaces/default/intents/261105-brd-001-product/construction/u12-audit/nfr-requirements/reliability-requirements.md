# Audit — Reliability Requirements

## Availability

| ID | Requirement |
|---|---|
| NFR1.1 | The append path is available 99.9% of the month, published as a target with no service credits (NFR1). |
| NFR1.2 | A replica that fails its health check is replaced by the Swarm; readiness fails while the database is unreachable, so traffic does not reach a replica that cannot write. |
| NFR1.3 | The audit service is deliberately not a single point of failure for user journeys: only security-event appends block their caller. Domain-event appends degrade without stopping the caller. |

## Failure behaviour by event class

**The class is decided by the audit service, not by the caller.** It derives it from the event's
`sensitivity`: `security` is blocking, `routine` is best-effort. A caller that sends an
`eventClass` contradicting the derived value is refused with `422`, so a service cannot label a
sign-in or a credential change best-effort and let it be dropped.

| ID | Requirement |
|---|---|
| NFR1.4 | A **security** event that cannot be stored fails the caller's action. The caller retries with the same `Idempotency-Key`; the first successful insert wins. |
| NFR1.5 | A **domain** event that cannot be stored is retried by the **emitter**, synchronously, at most twice with 100 ms and then 200 ms of jittered backoff, inside the emitter's own request budget. The audit service answers `503` and the emitter stops after those two tries: it logs the loss, increments its own counter, and proceeds with its action. The audit service never retries on its own and holds no queue — a retry there would either block the caller or die with the replica. |
| NFR1.6 | An `Idempotency-Key` is unique within the emitting service, not globally: uniqueness is `(emitting_service, idempotency_key)`. Reused with the same payload it answers `201` with the existing id; reused with a different payload it answers `409`. The key lives as long as its row does — there is no 24-hour expiry, because nothing in this schema is deleted before retention sweeps it, and a key that silently started working again would be a way to duplicate an event. |
| NFR1.7 | A session check that cannot be answered answers **deny**, never allow, and records the denial with its failure reason. |

## Recovery

| ID | Requirement |
|---|---|
| NFR18.1 | RPO ≤ 24 h and RTO ≤ 4 h, inherited from NFR18. The audit schema is inside the same Supabase backup regime as every other schema; the restore drill is recorded in the runbook. |
| NFR18.2 | A restore is verified by checking the newest `recordedAt` and a known event id from an exported sample, not by assuming the backup landed. |
| NFR18.3 | Migrations ship as sqlx up/down pairs; the down migration for a release that added a partition removes it cleanly. |

## Retention safety

| ID | Requirement |
|---|---|
| NFR14.1 | Audit logs are retained six calendar months from `recordedAt`, and the sweep deletes only what is past its window and free of a legal hold. |
| NFR14.2 | A retention run that fails part-way leaves the deleted batches deleted and records `failed` with the reason; the next run recomputes its cutoff rather than resuming. |
| NFR14.3 | Retention is automated by a scheduled job. The **primary** alert is the job-orchestration heartbeat: a missed run alerts within five minutes (FR10.2). The 48-hour "retention stale" alert in the observability requirements is the backstop for the case where the heartbeat itself is lying — it is not a second definition of the same check. |

## Graceful degradation

| ID | Requirement |
|---|---|
| NFR1.8 | When the audit database is slow, appends queue behind a bounded connection pool and fail fast once the wait exceeds the append timeout (2 s), rather than holding a caller's request open. |
| NFR1.9 | Operator queries degrade before appends do: the query path has its own rate limit and its own timeout (10 s), so an expensive query cannot take the append path down. |

## Sources

- `construction/u12-audit/functional-design/functional-spec.md` — WF1 failure behaviour, WF3, WF6, and the integration table
- `construction/u12-audit/functional-design/rules.md` — BR2.1, BR2.2, BR3.2, BR4.1, BR4.2
- `inception/requirements-analysis/requirements.md` — NFR1, NFR14, NFR18
- `inception/contract-design/contract-summary.md` — C10 and C11
