# Monitoring Design — u3-support-access

**Upstream:** `nfr-design/observability-design.md` (NFR17.1–NFR17.13, NFR1.8), `nfr-design/reliability-design.md`,
`infra/prometheus/job-orchestration-rules.yml` (the house shape for a rule file),
`memory/team.md` (`## On-call` — alerts must be actionable).

## Metrics & KPIs

**Collection gap first:** the repository declares no Prometheus scrape job — there is no
`prometheus.yml`, no scrape configuration and no ServiceMonitor anywhere, only the rule file
`infra/prometheus/job-orchestration-rules.yml`. `/metrics` is served (Identity renders it by hand),
but nothing in this repository says what scrapes it. Every row and alert below therefore assumes
the platform's Prometheus targets `identity-api` and `identity-worker`; where it does not, the
series are uncollected and the alert cannot fire. Closing that gap is a platform task, recorded
here rather than hidden.

Scraped from `/metrics` on `identity-api`; every series is rendered zero-initialised, in the
hand-rolled exposition style of `services/identity/http/src/metrics.rs`. Identity's audit-outbox
drainer runs **inside the `identity-api` process** (`services/identity/src/lib.rs:291`), and
`identity-api` runs **3 replicas**, so each counter below arrives as three series and every rate
reads `sum(rate(...))`; the per-replica heartbeat is read with `max()` (the freshest successful pass
anywhere), because one stuck replica is not an outbox incident while all three stuck is.

| Metric | Source | Threshold (alert window) | Why it matters |
|---|---|---|---|
| `support_grants_total{operation,outcome}` | identity-api, summed over replicas | `failed` share > 0.05% over 30 min | Volume and refusal mix per write path; `failed` is a 503 |
| `support_reads_total{outcome}` | identity-api, summed | `unavailable` share > 0.3% over 30 min | The one read path, split by served / refused / unavailable / failed |
| `support_read_duration_seconds` histogram | identity-api | p95 > 150 ms, p99 > 300 ms over 10 min | Guards the composed read budget |
| `support_metadata_source_errors_total{source,cause}` | identity-api | > 0.2% per source over 10 min | Names the failing owner so the right team is paged |
| `support_audit_write_failures_total{cause}` | identity-api | any increase over 5 min | A read or refusal failed closed |
| `support_outbox_deliveries_total{outcome}` | identity-api, summed | any increase of `dead` over 15 min | The one delivery class nobody can retry automatically |
| `support_outbox_pending` | identity-api `/metrics` (scrape-time count) | > 100 rows for 15 min | Backlog, readable even when the drainer has stopped |
| `support_outbox_oldest_age_seconds` | identity-api `/metrics` (scrape-time min) | > 900 s for 5 min | How long an undelivered intent has waited |
| `support_outbox_last_pass_timestamp_seconds` | identity-api, `max()` over replicas | stale > 300 s, or absent | The drainer heartbeat — the only in-process reading |
| `support_grant_decision_errors_total{cause}` | identity-api, summed | > 0.1% over 10 min | The grant lookup as a dependency of the read |
| `http_requests_total{service="identity",route=...}` | identity-api, summed | p95 > 80 ms on the write routes over 10 min | Existing series, now carrying the four support routes |

Each threshold sits **below** the SLO breach it protects (0.05% against a 0.1% write budget, 0.3%
against a ~0.4% read budget) so an alert fires while there is still budget to spend.

## Alerts

One new rule file, `infra/prometheus/support-rules.yml`: a `groups:` list whose every rule carries
`severity` and `service` labels and **both** `summary` and `runbook` annotations. The shipped
`job-orchestration-rules.yml` is the starting point, not the model — two of its three rules carry no
`runbook` at all, so that file is the one to bring up to this shape rather than an example to copy.

| Alert | Condition | Severity | Routes to |
|---|---|---|---|
| `SupportMetadataSourceErrors` | per-source error rate > 0.2% for 10 min | P2 | The on-call of the failing owner (account or portfolio), copied to Identity on-call |
| `SupportReadUnavailable` | `sum(rate(support_reads_total{outcome="unavailable"}[5m])) / sum(rate(support_reads_total[5m])) > 0.003` for 30 min | P2 | Identity on-call — the composed budget is being spent |
| `SupportWriteFailures` | `sum(rate(support_grants_total{outcome="failed"}[5m])) / sum(rate(support_grants_total[5m])) > 0.0005` for 30 min | P2 | Identity on-call — the write path is answering 503 |
| `SupportGrantDecisionErrors` | `sum(rate(support_grant_decision_errors_total[10m])) / sum(rate(support_reads_total[10m])) > 0.001` for 10 min | P3 | Identity on-call — the grant lookup is failing inside the read |
| `SupportAuditFailingClosed` | `sum(increase(support_audit_write_failures_total[5m])) > 0` | P1 | Identity on-call; support reads and refusals are failing closed |
| `SupportOutboxBacklog` | `max(support_outbox_pending) > 100` for 15 min | P2 | Identity on-call — the drainer is not keeping up |
| `SupportOutboxStalled` | `max(support_outbox_oldest_age_seconds) > 900` **or** `time() - max(support_outbox_last_pass_timestamp_seconds) > 300` **or** `absent(support_outbox_last_pass_timestamp_seconds)` | P2 | Identity on-call — check the drainer inside `identity-api`, not `identity-worker` |
| `SupportOutboxDeadRows` | `sum(increase(support_outbox_deliveries_total{outcome="dead"}[15m])) > 0` | P2 | Identity on-call — run the NFR1.5 replay statement |
| `SupportReadLatency` | histogram p95 > 150 ms or p99 > 300 ms for 10 min | P3 | Identity on-call |
| `SupportWriteLatency` | HTTP p95 > 80 ms on the three write routes for 10 min | P3 | Identity on-call |

Every threshold sits below its SLO breach and every alert names a runbook action. No alert pages
on a condition nobody can act on (`memory/team.md`).

## SLIs / SLOs

| SLI | SLO target | Measurement window |
|---|---|---|
| Metadata read availability: `served / (served + unavailable + failed)`; `refused` excluded | ~99.6% (the product of four fail-closed dependencies) | 30-day rolling |
| Metadata read latency: share of reads under 300 ms | 99% | 30-day rolling |
| Grant write availability: `accepted / (accepted + failed)`; `refused` excluded | 99.9% | 30-day rolling |
| Audit delivery freshness: p99 of `support_outbox_oldest_age_seconds` | ≤ 15 min | 30-day rolling |
| Error budget | read ~0.4%, writes 0.1% | 30-day rolling |

The read SLO is deliberately lower than the platform's 99.9%: it composes four services, and
claiming the platform number would hide that arithmetic (`observability-design.md`, NFR1.2).

## Logs & Tracing

- **Logs** reach stdout through `tracing`/`tracing-subscriber`, and Docker captures them. **No
  Loki, Promtail, Alloy or other log-shipping configuration exists in the repository**, so the
  route onward from the node is a platform concern recorded here, not a fact about today. One
  structured line per mutation, per refusal and per audit-write failure; a served read logs nothing
  at info — its record is the audit event.
  Fields: `correlation_id`, `action`, `actor_id`, `target_user_id`, `grant_id`, `outcome`,
  `refusal_code`, `source`. Never the reason text, an email address, a token or a source body.
- **Levels**: `warn` for a source failure, an audit-write failure and a dead-lettered row; `info`
  for mutations; `error` for a database failure that refuses a request.
- **Traces**: **Identity has no tracing exporter.** No `opentelemetry` or OTLP dependency appears
  in any `Cargo.toml` or source file, and `infra/` holds only `prometheus/job-orchestration-rules.yml`
  — so nothing leaves the process today. What the unit will emit once the platform lands an
  exporter: `support.metadata.read` with `support.metadata.account`, `.portfolio` and `.audit`
  children; `support.grant.request|approve|revoke` with the outbox insert as a child; one span per
  drained row carrying `outbox_id`. The correlation id comes from the gateway header, unchanged.
  This paragraph is the shape to build, not a description of the current export path — the same
  honesty the scrape gap gets above.
- **Cardinality**: no grant id, user id, email or reason text becomes a metric label; every label
  vocabulary is a closed enum in the code.

## Dashboards

One board on the existing Grafana:

| Row | Panels |
|---|---|
| Writes | `support_grants_total` by operation and outcome; HTTP p95 by write route |
| Reads | `support_reads_total` by outcome; read latency p50/p95/p99 by outcome |
| Sources | `support_metadata_source_errors_total` by source and cause |
| Audit | `support_audit_write_failures_total`; `support_outbox_deliveries_total` by outcome |
| Outbox | pending count, oldest age and last-pass heartbeat side by side — a rising backlog with a fresh heartbeat is a different incident from one with a stale heartbeat |