# BRD-011 — Background Jobs & Data Refresh

Draft v1.1 · Operations/Product · 2026-11-02. The scheduled processing that keeps the catalogue, prices, and snapshots current, with idempotency, failure handling, and observability. Business-critical: if it stops, valuations go stale without anyone noticing.

**In scope:** daily catalogue refresh · recurring price collection · EOD snapshots · manual admin triggers · scheduling, retries, idempotency, concurrency, run logs.
**Out of scope:** realtime streaming feeds · on-demand user sync (BRD-004) · analytics pipelines · cost-basis processing.

## Jobs

| Job | Purpose | Default (UTC) |
|-----|---------|---------------|
| Asset catalogue refresh | Update assets, contract mappings, instrument/pair metadata | Daily 00:00 |
| Price collection | Fetch prices for tracked assets | Every 15 min |
| EOD snapshots | Value every portfolio, record end-of-day snapshot | Daily 23:00 |

Schedules are configuration items, not hard-coded facts; changes need operations approval and a BRD update.

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-JOB-01 | Run the catalogue refresh daily. | Must |
| FR-JOB-02 | Run price collection on the configured interval. | Must |
| FR-JOB-03 | Create EOD snapshots for all active portfolios daily. | Must |
| FR-JOB-04 | Each run records start, end, outcome, and affected-record summary. | Must |
| FR-JOB-05 | Failed runs retry without duplicating successful side effects. | Must |
| FR-JOB-06 | No interactive user session required. | Must |
| FR-JOB-07 | Admins can trigger each job manually. | Should |
| FR-JOB-08 | Manual triggers are admin-authorized and audited. | Must |
| FR-JOB-09 | No overlapping runs of the same job. | Must |
| FR-JOB-10 | Price collection tolerates per-asset failure and continues. | Must |
| FR-JOB-11 | EOD continues for other portfolios when one fails. | Must |
| FR-JOB-12 | Alert operators on failure or non-run. | Should |
| FR-JOB-13 | Expose job status and last-run info to admins. | Should |
| FR-JOB-14 | Configurable schedules without code changes. | Could |
| FR-JOB-15 | Bounded backfill of missing prices/snapshots. | Could |

## Business rules

| ID | Rule |
|----|------|
| BR-JOB-01 | Scheduled times are UTC unless documented otherwise. |
| BR-JOB-02 | Jobs use the latest stored prices; missing prices are never fabricated. |
| BR-JOB-03 | EOD snapshots are idempotent per portfolio + date + type; re-runs update. |
| BR-JOB-04 | Failures never corrupt data; partial writes are avoided or clearly recorded. |
| BR-JOB-05 | Jobs process all users without leaking data across users. |
| BR-JOB-06 | Job logs contain no credentials, tokens, or sensitive personal data. |
| BR-JOB-07 | Missed schedules are detectable; silent failure is unacceptable. |
| BR-JOB-08 | Manual triggers are rate-limited. |
| BR-JOB-09 | Long-running jobs are observable and interruptible without inconsistent state. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-JOB-01 | The catalogue job updates asset/contract data and leaves a run record. |
| AC-JOB-02 | One asset's price failure doesn't stop the others; the failure is recorded. |
| AC-JOB-03 | Two overlapping triggers of the same job: one runs, the other is rejected/deferred. |
| AC-JOB-04 | Re-running today's EOD updates the snapshot; no duplicate. |
| AC-JOB-05 | A non-admin manual trigger is denied. |
| AC-JOB-06 | A failure is visible to operations with scope, and an alert fires. |
| AC-JOB-07 | No job log contains credentials or token values. |

## NFRs

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-JOB-01 | Price collection fits its interval | < 15 min |
| NFR-JOB-02 | EOD completes before next business day | < 1 hour |
| NFR-JOB-03 | Failure detection latency | < 15 min |
| NFR-JOB-04 | Run-record retention | ≥ 90 days (proposed) |

## Notes

- The scheduler survives application restarts; providers rate-limit and fail — retries use backoff; snapshot ordering depends on the price job; job load is bounded alongside interactive traffic.

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | 15-minute price interval at launch, or meet BRD-001's <30s freshness goal? | Product |
| OQ-002 | Acceptable backfill window after an outage? | Operations |
| OQ-003 | Who receives job alerts, via which channel? | Operations |
| OQ-004 | EOD cutoff: fixed UTC or user-local midnight? | Product |
| OQ-005 | Job-log retention period? | Compliance |
