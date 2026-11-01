# BRD-011 — Background Jobs & Data Refresh

| Field | Value |
|-------|-------|
| Document ID | BRD-011 |
| Version | 1.0 |
| Status | Draft — requires operations sign-off |
| Owner | Operations / Product |
| Last updated | 2026-11-01 |
| Related documents | BRD-004, BRD-005, BRD-007, BRD-010, BRD-012 |

## 1. Purpose

Define the scheduled processing that keeps the asset catalogue, prices, and portfolio history current, together with the guarantees around idempotency, failure handling, and observability. Scheduled processing is business-critical: if it stops, valuations become stale without users necessarily noticing.

## 2. Scope

### In scope

- Daily asset-catalogue refresh.
- Recurring market-price collection.
- End-of-day portfolio snapshot creation.
- Manual job triggers for operations and support.
- Scheduling, retries, idempotency, concurrency, and run logging.

### Out of scope

- Real-time streaming price feeds (future).
- Per-user on-demand sync of exchange/wallet balances (covered by BRD-004; these jobs may invoke it in future).
- Data-warehouse or analytics pipelines (post-MVP).
- Cost-basis or transaction-ledger processing (post-MVP).

## 3. Scheduled jobs

| Job | Purpose | Default schedule (UTC) | Outputs |
|-----|---------|------------------------|---------|
| Asset catalogue refresh | Fetch the top asset list and metadata; update assets and contract mappings | Daily at 00:00 | Asset records, contract mappings |
| Price collection | Fetch current prices for tracked assets | Every 15 minutes | Price observations |
| EOD portfolio snapshots | Value every portfolio and record an end-of-day snapshot | Daily at 23:00 | Snapshot per active portfolio |

> Schedule times are the current documented defaults and are configuration items, not hard-coded business facts. Changes require operations approval and must be reflected in this BRD.

## 4. Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-JOB-01 | The system must run the asset-catalogue refresh on a daily schedule. | Must |
| FR-JOB-02 | The system must run price collection on the configured interval. | Must |
| FR-JOB-03 | The system must create EOD snapshots for all active portfolios on a daily schedule. | Must |
| FR-JOB-04 | Each job run must record start time, end time, outcome, and a summary of affected records. | Must |
| FR-JOB-05 | A failed job run must be retryable without duplicating successful side effects. | Must |
| FR-JOB-06 | Job execution must not require an interactive user session. | Must |
| FR-JOB-07 | An Admin must be able to trigger each job manually. | Should |
| FR-JOB-08 | Manual triggers must be authorized (admin role) and audited. | Must |
| FR-JOB-09 | The system must prevent overlapping runs of the same job. | Must |
| FR-JOB-10 | Price collection must tolerate per-asset failures and continue with the remaining assets. | Must |
| FR-JOB-11 | EOD snapshot creation must continue for other portfolios when one portfolio fails. | Must |
| FR-JOB-12 | The system should alert operators when a scheduled job fails or does not run. | Should |
| FR-JOB-13 | The system should expose job status and last-run information to administrators. | Should |
| FR-JOB-14 | The system should support configurable schedules without code changes. | Could |
| FR-JOB-15 | The system should support backfilling missing price or snapshot data for a bounded period. | Could |

## 5. Business rules

| ID | Rule |
|----|------|
| BR-JOB-01 | Scheduled times are defined in UTC unless a documented exception exists. |
| BR-JOB-02 | Price collection and EOD snapshots must use the latest successfully stored prices; they must not fabricate missing prices. |
| BR-JOB-03 | EOD snapshots are idempotent per portfolio, date, and type; a re-run updates the existing record. |
| BR-JOB-04 | A job failure must not corrupt existing data; partial writes must be avoided or clearly recorded. |
| BR-JOB-05 | Jobs must process all users without leaking one user's data into another's record. |
| BR-JOB-06 | Job logs must not contain credentials, tokens, or sensitive personal data. |
| BR-JOB-07 | A missed schedule must be detectable; silent failure is not acceptable. |
| BR-JOB-08 | Manual job triggers must be rate-limited to prevent operational abuse. |
| BR-JOB-09 | Long-running jobs must be observable and interruptible without leaving inconsistent state. |

## 6. Data requirements

| ID | Data element | Description | Sensitivity |
|----|--------------|-------------|-------------|
| DR-JOB-01 | Job run record | Job name, start/end, outcome, counts, error summary | Internal |
| DR-JOB-02 | Asset refresh results | Added/updated assets and contracts | Internal |
| DR-JOB-03 | Price observations | Asset, price, source, timestamp | Internal |
| DR-JOB-04 | Snapshot outputs | Portfolio, date, type, value | Sensitive |
| DR-JOB-05 | Manual trigger audit | Actor, job, timestamp | Audit |
| DR-JOB-06 | Alert events | Failure/absence notifications | Internal |

## 7. Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-JOB-01 | Given the daily schedule, when the catalogue job runs, then asset and contract data are updated and a run record exists. |
| AC-JOB-02 | Given price collection, when one asset fails, then the remaining assets are still updated and the failure is recorded. |
| AC-JOB-03 | Given two overlapping triggers for the same job, when the second starts, then it is rejected or queued, and only one run executes. |
| AC-JOB-04 | Given an EOD snapshot already exists for today, when the job re-runs, then the existing snapshot is updated and no duplicate is created. |
| AC-JOB-05 | Given a non-admin user, when a manual trigger is attempted, then access is denied. |
| AC-JOB-06 | Given a job failure, when operations inspect the system, then the failure and affected scope are visible and an alert is produced. |
| AC-JOB-07 | Given any job log, when inspected, then it contains no credentials or token values. |

## 8. Non-functional requirements

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-JOB-01 | Price collection completes within the schedule interval under normal load. | < 15 minutes |
| NFR-JOB-02 | EOD snapshot creation completes before the next business day begins. | < 1 hour for expected portfolio volume |
| NFR-JOB-03 | Job failure detection latency | < 15 minutes after expected completion |
| NFR-JOB-04 | Job records are retained for operational review. | ≥ 90 days (proposed) |

## 9. Assumptions, dependencies, constraints

- The scheduler runs in the deployed environment and survives application restarts.
- External providers enforce rate limits and may be unavailable; retries use backoff.
- Snapshot generation depends on prices from the price job; ordering must be guaranteed.
- Job execution shares the same data stores as the interactive application; load must be bounded.

## 10. Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Is the price interval 15 minutes at launch, or must it meet the <30-second freshness goal in BRD-001? | Product |
| OQ-002 | What is the acceptable backfill window after an outage? | Operations |
| OQ-003 | Who receives job alerts, and through which channel? | Operations |
| OQ-004 | Should EOD use a fixed UTC cutoff or each user's local midnight? | Product |
| OQ-005 | What is the retention period for job logs? | Compliance |
