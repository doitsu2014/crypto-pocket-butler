# Observability Design — u3-support-access

**Upstream:** `observability-requirements.md` (NFR17.1–NFR17.13), `security-design.md`
(the audit field set), `reliability-design.md` (the drainer and its heartbeat), the existing
`services/identity/http/src/metrics.rs`.

## Metrics collection

The service renders Prometheus text by hand in `IdentityMetrics::render`; the support counters are
added to that same struct and rendered from it. **Every series is emitted with its zero value on
first render**, as `authorisation_decisions_total` already does, so a route that has never been
called is still visible and an alert can distinguish "zero" from "absent".

### Series

| Series | Type | Labels | Rendered from |
|---|---|---|---|
| `support_grants_total` | counter | `operation` (request, approve, revoke), `outcome` (accepted, refused, failed) | a `BTreeMap<(String, String), u64>` under one mutex |
| `support_reads_total` | counter | `outcome` (served, refused, unavailable, failed) | four atomics |
| `support_read_duration_seconds` | histogram | `outcome` | fixed buckets 25, 50, 100, 150, 200, 300, 500, 1000, +Inf, one `_sum`/`_count` pair per outcome |
| `support_metadata_source_errors_total` | counter | `source` (account, portfolio), `cause` (timeout, unavailable, malformed) | a map under one mutex |
| `support_audit_write_failures_total` | counter | `cause` (read_append, refusal_append) | two atomics |
| `support_outbox_deliveries_total` | counter | `outcome` (delivered, retried, dead) | three atomics on the sink |
| `support_outbox_pending` | gauge | — | a scrape-time `COUNT(*)` of `pending` rows, via `SupportOutboxBacklog` |
| `support_outbox_oldest_age_seconds` | gauge | — | a scrape-time `MIN(created_at)` of `pending` rows, via `SupportOutboxBacklog` |
| `support_outbox_last_pass_timestamp_seconds` | gauge | — | set at the end of every support pass, even a failed one — the one reading that must come from the process |
| `support_grant_decision_errors_total` | counter | `cause` (timeout, unavailable) | two atomics |

No label is a grant id, a user id, an email address, or a reason text; the label vocabularies are
closed and bounded, so cardinality is fixed by the code (NFR22.1).

### Where the drainer gauges come from

Two of the three are **read at scrape time, not pushed by the pass** (NFR1.8, NFR17.7):
`support_outbox_pending` and `support_outbox_oldest_age_seconds` are a `COUNT(*)` and a
`MIN(created_at)` over `state = 'pending'`, executed by the `/metrics` handler through
`SupportOutboxBacklog`, so a drainer that has stopped still reports its true backlog. When the
table is empty the count is `0` and the oldest age is emitted as `0`.

The third, `support_outbox_last_pass_timestamp_seconds`, cannot be a scrape read: it is the
heartbeat, and only the process can know that its pass last ran. It is written at the end of every
support pass regardless of outcome, so "running but failing" (fresh heartbeat, rising backlog) is
distinguishable from "stopped" (stale or absent heartbeat) — the distinction the alert needs. An
absent series counts as stalled.

## SLI / SLO

| SLI | Definition | SLO |
|---|---|---|
| Metadata read availability | `support_reads_total{outcome="served"} / (served + unavailable + failed)` over 30 days; `refused` is excluded because a refusal by a rule is the product working, not an outage | ~99.6% — the product of its four fail-closed dependencies (NFR1.2) |
| Metadata read latency | share of reads under 300 ms, from the histogram | 99% |
| Grant write availability | `support_grants_total{outcome="accepted"} / (accepted + failed)` over 30 days; `refused` is excluded for the same reason | 99.9%, the platform target for a path whose only fail-closed dependency is its own database and audit outbox |
| Audit delivery freshness | `support_outbox_oldest_age_seconds` | p99 of the backlog age ≤ 15 minutes |
| Error budget | `1 − SLO` | read: ~0.4% of what was actually served or attempted; writes: 0.1% |

Both availability SLIs therefore count only outcomes the service was responsible for: an
`unavailable` or `failed` reading is a failed request, a `refused` one is a correct answer.

The read SLO is deliberately **not** the platform's 99.9%: it composes four services that each
target 99.9%, and claiming otherwise would hide that arithmetic (NFR1.2). The write SLO keeps the
platform number because an accepted mutation waits for nothing but its own database.

## Alerting

| Alert | Expression (shape) | For | Action |
|---|---|---|---|
| `SupportMetadataSourceErrors` | rate of `support_metadata_source_errors_total` per source / rate of reads > 0.002 | 10 min | Find the owner; the read is refusing |
| `SupportReadUnavailable` | `support_reads_total{outcome="unavailable"}` share > 0.004 | 30 min | The composed budget is breached |
| `SupportAuditFailingClosed` | increase of `support_audit_write_failures_total` > 0 | 5 min | The audit path is down; reads and refusals are refusing |
| `SupportOutboxStalled` | `support_outbox_oldest_age_seconds > 900` **or** `support_outbox_last_pass_timestamp_seconds` older than 300 s **or** `absent(support_outbox_last_pass_timestamp_seconds)` | 5 min | Check the identity-worker; the alert does not depend on the drainer emitting |
| `SupportOutboxDeadRows` | increase of `support_outbox_deliveries_total{outcome="dead"}` > 0 | 15 min | Run the replay statement (NFR1.5) |
| `SupportReadLatency` | histogram quantile 0.95 > 0.15 or 0.99 > 0.30 | 10 min | Investigate the slow source |
| `SupportWriteLatency` | HTTP p95 for the three write routes > 0.08 | 10 min | Investigate the database |

Every threshold sits below its SLO breach and every alert names an action; no alert pages for a
condition nobody can act on (`memory/team.md` — "delete alerts nobody acts on").

## Logging

- One structured `tracing` line per mutation, per refusal, and per audit-write failure. A **served**
  read writes no info-level line — its record is the audit event.
- Field set: `correlation_id`, `action`, `actor_id`, `target_user_id`, `grant_id` (all identifiers),
  `outcome`, `refusal_code` where applicable, `source` on a fan-out failure. Never the reason text,
  an email address, a token, or a source response body (NFR22.2).
- Level policy: `warn` for a source failure, an audit-write failure, and a dead-lettered row;
  `info` for mutations; `error` for a database failure that refuses a request.

## Tracing

- The read is one parent span (`support.metadata.read`) with two child spans
  (`support.metadata.account`, `support.metadata.portfolio`) started with `try_join!`, and a third
  (`support.metadata.audit`) for the append. The child that ends without a response is the failing
  dependency, which is what the latency histogram cannot tell you.
- A mutation is one span (`support.grant.request|approve|revoke`) covering the transaction, with
  the outbox insert as a child span so a commit that missed its row is visible.
- A drain pass is a span per row with `outbox_id`; the trace exports through the service's existing
  OTLP pipeline, and the correlation id comes from the gateway's request header, unchanged.

## Dashboards

One board on the existing Grafana: writes by operation and outcome; reads by outcome; read latency
by outcome; source errors by source and cause; audit failures; and the outbox trio (pending, oldest
age, last pass) on one row. The outbox row is deliberately three series side by side, because a
rising backlog with a fresh heartbeat is a different incident from one with a stale heartbeat.