# Reliability Requirements — u4-preferences-consent

**Upstream:** `functional-spec.md` (W0, W4), `rules.md` (BR4.5, BR4.6), `contract-summary.md` (C6, C9), `security-requirements.md` (NFR19.1–19.4).

## A blocking precondition: C6 must carry the gate

The consent-state query (W0), the 503 it answers on failure, and the 409/422/404 statuses W2–W5
use are **not in the passed C6 contract**, whose single consumer today is U7. NFR1.2 and the
performance budget for W0 are stated against that amendment. It is a precondition of this unit:
C6 gains the consent-state endpoint, `account-api` as a consumer, and those statuses, or the gate
has no interface to build.

## Availability

| ID | Requirement |
|---|---|
| NFR1.1 | The unit is available whenever the Identity service and its Postgres are. The end-to-end target is the service's 99.9% monthly. |
| NFR1.2 | A consent-state read failure fails closed: the gate answers 503 and the source connection is refused. It never returns an empty `missing` list on a failure. |
| NFR1.3 | The gate and the preferences read ride the service's existing database readiness check; no new probe. |

## Fault tolerance

| Failure | Effect | Behaviour |
|---|---|---|
| Postgres read slow or down | Preferences and the gate fail | 503; the connection is refused, never allowed on a guess |
| Insert fails on a grant or withdrawal | No event recorded | Error returned; never a claimed success |
| **C9 down on a withdrawal** | The event and its outbox row committed; delivery pending | The withdrawal is accepted — its audit intent is durable — and the drainer retries. An undelivered row older than 15 minutes alerts |
| Outbox insert fails | Neither row commits | The withdrawal fails; state and intent commit together or not at all (NFR19.3) |
| Duplicate submit of the same grant | One event per submit | State unchanged; grants are idempotent in effect |

## The C9 client

| ID | Requirement |
|---|---|
| NFR1.4 | The drainer's C9 client uses connect 5 s / read 10 s, at most three attempts with exponential backoff and jitter, and every request carries its outbox row's idempotency key so a retried post cannot duplicate the audit entry. |
| NFR1.5 | The request path makes **no** outbound C9 call: a withdrawal returns after its own transaction commits, so the C9 dependency never appears in the withdrawal's latency budget. This is what "no retries in the request path" means; the drainer is where retries live. |
| NFR1.6 | A row that exhausts its retries is left in the outbox, alerted, and replayable by a manual drain; it is never deleted to make the alert go away. |

## Consistency

| ID | Requirement |
|---|---|
| NFR24.1 | The gate reads the newest event on every call, so a withdrawal takes effect at the next attempt with no cache to stale and no replica to lag. |

## Recovery

| ID | Requirement |
|---|---|
| NFR18.1 | Preferences, consent events, and the outbox are rows in the Identity schema, covered by the platform's Postgres backup (RPO ≤ 24 h, RTO ≤ 4 h). A restored outbox replays undelivered audit intents rather than losing them. |

## Failure-mode checklist

| Question | Answer |
|---|---|
| Unavailable dependency? | Postgres (fail closed) and C9 (deferred through the outbox). |
| Response time doubles? | The latency alert fires at the budget. |
| Throughput exceeds capacity? | Replica count; no state to migrate. |
| Corrupted data? | A malformed preference value is refused on write; a malformed stored value is treated as unset. |
| Graceful path? | None for the gate by design; the outbox is the grace for audit delivery only. |
| Blast radius? | One user's records; no cross-user read. |
| Recovery? | Platform restore, plus outbox replay. |
