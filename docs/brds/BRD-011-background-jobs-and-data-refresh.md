# BRD-011 — Background Jobs & Data Refresh

Draft v1.2 · Operations/Product · 2026-11-04. The scheduled processing that keeps the catalogue, prices, FX, transactions, snapshots, notifications, and billing current, with idempotency, failure handling, and observability. Business-critical: if it stops, valuations go stale and users are not told. v1.2 adds transaction sync, FX, notifications, billing, and rolling timezone-aware EOD snapshots.

**In scope:** catalogue refresh · multi-source price collection · FX collection · account and transaction sync · EOD snapshots per user timezone · brief generation · notification dispatch · billing renewal and dunning · retention · reconciliation · manual admin triggers · scheduling, retries, idempotency, concurrency, run logs, dead-man detection.
**Out of scope:** realtime streaming feeds · on-demand user sync (BRD-004) · analytics pipelines (BRD-021) · cost-basis processing (BRD-013).

## Jobs

| Job | Purpose | Default cadence |
|-----|---------|-----------------|
| Asset catalogue refresh | Update assets, contract mappings, instrument/pair metadata | Daily 00:00 UTC |
| Price collection | Fetch prices for tracked assets from priority sources | Every 1 minute for top assets; every 15 minutes for the tail |
| FX collection | Fetch and store base-currency rates | Hourly; daily historical close |
| Account sync | Refresh holdings for active accounts | Per plan (15 min Free; 1 min Pro top assets) |
| Transaction sync | Import new exchange/on-chain transactions | Per plan; idempotent |
| EOD snapshots | Value every active portfolio at local midnight | Rolling; per user timezone |
| Brief generation | Compose daily/weekly briefs | Per user timezone and preference |
| Notification dispatch | Deliver queued alerts and digests | Every minute |
| Billing renewal and dunning | Charge, retry, and notify | Provider-driven; daily reconciliation |
| Reconciliation | Compare ledger, holdings, and portfolio state | Hourly |
| Retention | Delete or anonymise per BRD-017 | Daily |
| Dead-man check | Alert if the scheduler heartbeat is stale | Every minute |

Schedules are configuration items, not hard-coded facts; changes need operations approval and a BRD update.

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-JOB-01 | Run the catalogue refresh daily. | Must |
| FR-JOB-02 | Run price collection on the configured interval. | Must |
| FR-JOB-03 | Create EOD snapshots for all active portfolios on schedule. | Must |
| FR-JOB-04 | Each run records start, end, outcome, and affected-record summary. | Must |
| FR-JOB-05 | Failed runs retry without duplicating successful side effects. | Must |
| FR-JOB-06 | No interactive user session required. | Must |
| FR-JOB-07 | Admins can trigger each job manually. | Should |
| FR-JOB-08 | Manual triggers are admin-authorized and audited. | Must |
| FR-JOB-09 | No overlapping runs of the same job. | Must |
| FR-JOB-10 | Price collection tolerates per-asset failure and continues. | Must |
| FR-JOB-11 | EOD continues for other portfolios when one fails. | Must |
| FR-JOB-12 | Alert operators on failure or non-run. | Must |
| FR-JOB-13 | Expose job status and last-run info to admins. | Must |
| FR-JOB-14 | Configurable schedules without code changes. | Should |
| FR-JOB-15 | Bounded backfill of missing prices/snapshots. | Should |
| FR-JOB-16 | Sync transactions from supported exchanges and imports idempotently. | Must |
| FR-JOB-17 | Collect and store FX rates with source and timestamp. | Must |
| FR-JOB-18 | Dispatch notifications with retry, dedupe, and delivery logs. | Must |
| FR-JOB-19 | Run billing renewal, dunning, and entitlement reconciliation. | Must |
| FR-JOB-20 | Run EOD snapshots at each user's local midnight, including DST. | Must |
| FR-JOB-21 | Apply plan-specific sync cadence and quotas. | Must |
| FR-JOB-22 | Detect a stale or revoked exchange key during sync and alert. | Must |
| FR-JOB-23 | Run retention and deletion jobs per BRD-017. | Must |
| FR-JOB-24 | Reconcile ledger quantity, holdings, and portfolio valuations. | Must |
| FR-JOB-25 | Aggregate consent-aware analytics events on schedule. | Must |
| FR-JOB-26 | Alert when a data source or provider is unhealthy. | Should |

## Business rules

| ID | Rule |
|----|------|
| BR-JOB-01 | Scheduled times are UTC unless documented otherwise; user-facing jobs use the user's timezone. |
| BR-JOB-02 | Jobs use the latest stored prices; missing prices are never fabricated. |
| BR-JOB-03 | EOD snapshots are idempotent per portfolio + local date + type; re-runs update. |
| BR-JOB-04 | Failures never corrupt data; partial writes are avoided or clearly recorded. |
| BR-JOB-05 | Jobs process all users without leaking data across users. |
| BR-JOB-06 | Job logs contain no credentials, tokens, or sensitive personal data. |
| BR-JOB-07 | Missed schedules are detectable; silent failure is unacceptable. |
| BR-JOB-08 | Manual triggers are rate-limited. |
| BR-JOB-09 | Long-running jobs are observable and interruptible without inconsistent state. |
| BR-JOB-10 | A billing or notification job failure never blocks read-only portfolio access. |
| BR-JOB-11 | A retention job never deletes records under legal hold. |
| BR-JOB-12 | A reconciliation mismatch is surfaced, not auto-corrected silently. |

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
| AC-JOB-08 | A transaction sync run is idempotent and records imported/skipped counts. |
| AC-JOB-09 | FX rates are stored with source and timestamp. |
| AC-JOB-10 | A user in each launch timezone receives one EOD snapshot per local date across DST. |
| AC-JOB-11 | A notification job retries a transient provider failure without duplicate delivery. |
| AC-JOB-12 | A retention run deletes only records past the schedule and not under legal hold. |
| AC-JOB-13 | A stale scheduler heartbeat fires the dead-man alert within 15 minutes. |

## NFRs

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-JOB-01 | Top-asset price collection fits its interval | < 1 min |
| NFR-JOB-02 | Tail price collection fits its interval | < 15 min |
| NFR-JOB-03 | EOD completes before 06:00 local time | < 6 h after local midnight |
| NFR-JOB-04 | Failure detection latency | < 5 min |
| NFR-JOB-05 | Run-record retention | ≥ 90 days |
| NFR-JOB-06 | Notification dispatch latency | < 1 min for urgent, < 15 min for digest |

## Notes

- The scheduler survives application restarts; providers rate-limit and fail — retries use backoff; snapshot ordering depends on the price and FX jobs; job load is bounded alongside interactive traffic.
- Multi-source price collection must respect each source's rate limit and licence.
- Timezone-aware EOD requires a rolling scheduler, not a single 23:00 UTC run.

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Exact top-asset tier and tail cadence at launch? | Product/Data |
| OQ-002 | Acceptable backfill window after an outage? | Operations |
| OQ-003 | Who receives job alerts, via which channel? | Operations |
| OQ-004 | Resolved by D-006: user timezone default, with optional portfolio override. | Product |
| OQ-005 | Job-log retention period? | Compliance |
