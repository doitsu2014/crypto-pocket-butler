# Reliability Requirements — u3-support-access

**Upstream:** `functional-spec.md` (W1–W5, the no-callback rule, the fail-closed read),
`rules.md` (BR3.13), `contract-summary.md` (C9, C10, timeouts and retries), `requirements.md`
(NFR21, NFR24), `memory/project.md` (up/down migrations, rolling update on the health check).

## What the unit depends on

| Dependency | Used by | Bound |
|---|---|---|
| Identity Postgres (own schema) | every path | pool acquire 5 s; `SET LOCAL statement_timeout` 2 s on mutations, 500 ms on the read's statements |
| Audit service (C10) | every path — accepted mutation (via outbox), refusal (own transaction, via outbox), served read (synchronous) | reads and rejections: 200 ms, one attempt (`HttpAuditClient`); support outbox sink: 1 s connect, 2 s request, one attempt per row, batch 5, retries across passes, idempotency key = outbox row id, inside the shipped 30 s claim lease |
| Account service (new internal endpoint) | W4 only | connect 1 s, request 120 ms, one attempt |
| Portfolio service (new internal endpoint) | W4 only | connect 1 s, request 120 ms, one attempt |
| Supabase JWKS | every path | the service's existing validator, with its existing refresh and staleness bounds |

Nothing else. In particular the read does **not** depend on U2's C4 endpoint: the role comes from
the validated claims the middleware already produced, and the grant decision is a local read.

## Availability

| ID | Requirement |
|---|---|
| NFR1.1 | The write paths (request, approve, revoke) are available whenever Identity's database and its audit path are: the grant row and its outbox row are one transaction, and delivery is asynchronous. A refusal additionally needs its own audit transaction to commit. |
| NFR1.2 | The metadata read is available only when **all four** of its fail-closed dependencies answer: Identity's database, Account, Portfolio and the audit store. Its availability is therefore the **product** of the four, not the single-service 99.9% the platform targets elsewhere: at 99.9% each the read path is ~99.6%, and that is what this unit promises for it — the service's overall 99.9% (BRD-012 NFR-AVL-01) is not claimed for this composed path. The projection is never returned partially: an Account failure and a Portfolio failure both answer `503 metadata-source-unavailable`, logged and counted with the failing source named. A half-projection would be worse than a refusal, because the caller cannot tell what is missing. |
| NFR1.3 | `/healthz` is the Swarm update gate and reports only that the process is up. `/readyz` reports the database and the audit store; Account and Portfolio reachability is reported but does **not** fail readiness — a source outage refuses the read (the designed behaviour), it does not take every Identity replica out of rotation. |

## Failure matrix

| Failure | Path | Behaviour | Signal |
|---|---|---|---|
| Account endpoint slow or down | W4 | 503 `metadata-source-unavailable`; no partial projection; Portfolio's answer is discarded | `support_metadata_source_errors_total{source="account"}`, `support_reads_total{outcome="unavailable"}` |
| Portfolio endpoint slow or down | W4 | as above, source `portfolio` | same counters, source `portfolio` |
| Both sources down | W4 | one 503; one audit `denied` event, not two | same |
| Audit service down, read | W4 | 503 `audit-unavailable`; the projection is not returned | `support_audit_write_failures_total{cause="read_append"}` |
| Audit service down, refusal | W1–W4 refusal | 503 `audit-unavailable` instead of the clean 403/409/422 | `support_audit_write_failures_total{cause="refusal_append"}` |
| Audit service down, accepted mutation | W1–W3 | **not** an error: the mutation commits with its outbox row and the drainer delivers later | `support_outbox_pending`, `support_outbox_oldest_age_seconds` |
| Identity database down | all | 503 at the pool; no path guesses | `support_grants_total{outcome="failed"}`, `support_reads_total{outcome="failed"}`; `/readyz` fails |
| Drainer down or wedged | W5 | mutations keep succeeding; audit delivery lags and the backlog ages | `support_outbox_oldest_age_seconds` rising; `support_outbox_last_pass_timestamp_seconds` stale |
| A row exhausts its attempts | W5 | row becomes `dead` with `last_error`; it is no longer picked | `support_outbox_deliveries_total{outcome="dead"}` |
| JWKS unreachable | all | 401 by the existing validator policy; U3 inherits it unchanged | existing auth counters |

## Outbox delivery

| ID | Requirement |
|---|---|
| NFR1.4 | `support_audit_outbox` carries retry state, mirroring `consent_audit_outbox`: `state` in (`pending`, `delivered`, `dead`), `attempts` (0–10, budget 3), `next_attempt_at`, `last_error` (≤ 300 characters), `created_at`, `delivered_at`, and a unique `idempotency_key`. The claim query selects only `pending` rows whose `next_attempt_at` is due, oldest first, with `FOR UPDATE SKIP LOCKED`. A row that exhausts the budget becomes `dead` and is never re-picked. **The vocabulary is `dead`, not `failed`** — this requirement supersedes `SupportAuditEvent.deliveryState` in `functional-design/entities.md`, and the migration, the metric labels and the entity are to use `dead`. |
| NFR1.5 | **Replay is a documented, operator-run statement, not an automatic retry:** `UPDATE support_audit_outbox SET state = 'pending', attempts = 0, next_attempt_at = now(), last_error = NULL, delivered_at = NULL WHERE id = $1 AND state = 'dead'`. It resets the same row's retry state and keeps its idempotency key, so a replayed event appends once. The statement lives in the Operations-stage runbook with the dead-row alert, under the `identity_migrator` role; no endpoint and no new command is added. |
| NFR1.6 | Delivery is at-least-once and duplicate-free in effect: the idempotency key is the outbox row's id, stable across attempts, and C10 appends once for it. Retries happen **across passes** through `next_attempt_at` (the shipped drainer's backoff), never inside a pass; only transient failures (429 and 5xx) are retried, and a 4xx is recorded as `last_error` and dead-lettered at once. |
| NFR1.7 | Delivered rows are deleted 24 hours after delivery, by the drainer's existing cleanup, so the outbox does not become a second audit store. |
| NFR1.8 | **The backlog alert must not depend on the drainer running.** The gauges are exposed on the service's `/metrics` endpoint (a scrape-time read of shared counters), not pushed from the drain pass, and the alert pairs them with `support_outbox_last_pass_timestamp_seconds` so a dead or wedged drainer — which keeps emitting nothing — is caught by staleness and by `absent()` when the process itself is gone. A drainer that dies silently is the exact failure the outbox exists to make visible. |

## Timeouts and retries

| ID | Requirement |
|---|---|
| NFR1.9 | No support path retries a write automatically. The host endpoints are not idempotent, and the contract retries only idempotent calls carrying an idempotency key; the gateway supplies one header, but Identity does not translate it into a grant identity, so a client retry is a second request. BR3.3 is what makes that safe: while a live grant exists a duplicate cannot be created, so a retried request answers 409 rather than producing two grants. |
| NFR1.10 | The two source reads are single-attempt with a 120 ms request timeout inside the 500 ms overall deadline. A retry would consume most of the remaining budget and could not keep the p99; a source that is slow is refused, not waited for. The timeout sits above the source's p99 target (NFR2.6), so it cuts off failures, not normal reads. |
| NFR1.11 | The synchronous audit append on a served read is single-attempt with the existing 200 ms client timeout. A failed append refuses the read; a retry would neither help nor fit the deadline, and the read is not represented in the outbox (a read has no mutation to reconcile against). |

## Graceful degradation

There is **no** degraded mode for the metadata read: it serves the projection or it refuses. The
only degradation in the unit is temporal — an accepted mutation's audit delivery lags behind the
mutation, bounded by the drainer's cadence and surfaced by NFR1.8. Reads take the stricter path on
purpose, because a served read is the one event an investigator may later need to prove happened.

## Recovery and rollback

| ID | Requirement |
|---|---|
| NFR18.1 | Grants and outbox rows live in the Identity schema and inherit the platform's backup (RPO ≤ 24 h, RTO ≤ 4 h, BRD-012). Audit events live in the audit store, append-only. U3 adds no store of its own. |
| NFR18.2 | Migration `0007_support_access` ships as an up/down pair. The down migration restores the original `support_grants_approval_shape` clause as **`NOT VALID`**, then drops the outbox and the partial index. `NOT VALID` is the point: rows written while 0007 was live — including an `expired` request that was never approved — stay exactly as recorded, while every later write is checked against the pre-0007 rule the rolled-back code follows. The down step therefore never refuses, never blocks a rollback, and never rewrites evidence. |
| NFR18.3 | The rolling update is the Swarm stack's existing policy: `/healthz` gates it, `parallelism: 1`, `failure_action: rollback`, `monitor: 30s` (`memory/project.md`, `infra/swarm-stack.yml`). U3 adds no new service and no new deployment unit. |
| NFR18.4 | The unit is replica-safe with no coordination: no in-process grant state, no local cache, and an outbox claim that tolerates N replicas (NFR20.5). A replica restart loses nothing that matters. |