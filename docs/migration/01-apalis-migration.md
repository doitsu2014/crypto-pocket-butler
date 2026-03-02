# Apalis Job-Runner Migration Guide

## Overview

This document describes the Phase 1 migration of the Crypto Pocket Butler job
scheduler from `tokio-cron-scheduler` to
[Apalis](https://github.com/geofmureithi/apalis) (v0.4).

All existing business logic, API endpoints, database operations and log
messages are preserved.  Only the scheduling layer changes.

---

## What Changed

| Aspect | Before | After |
|--------|--------|-------|
| Scheduler crate | `tokio-cron-scheduler 0.13` | `apalis 0.4` |
| Job registration | `JobScheduler` + `Job::new_async` | `Monitor` + `WorkerBuilder` + `CronStream` |
| State injection | `db.clone()` captured in closure | `Extension(db)` layer on each worker |
| Env variable prefix | `FETCH_ALL_COINS_*` / `EOD_SNAPSHOT_*` | `APALIS_FETCH_ALL_COINS_*` / `APALIS_EOD_SNAPSHOT_*` |
| Cron variable suffix | `_SCHEDULE` | `_CRON` |

> **Note** `tokio-cron-scheduler` is kept as a Cargo dependency for the
> rollback period (Phase 2 will remove it once Apalis is validated in
> production).

---

## Cron Expression Format

Both cron implementations use the **same 6-field format**:

```
sec  min  hour  day-of-month  month  day-of-week
```

| Job | Expression | Meaning |
|-----|-----------|---------|
| fetch-all-coins | `0 */15 * * * *` | Every 15 minutes on the 0-second mark |
| eod-snapshot | `0 0 23 * * *` | Daily at 23:00 UTC |

---

## Environment Variables

### Old variables (no longer read)

```bash
FETCH_ALL_COINS_ENABLED=true
FETCH_ALL_COINS_SCHEDULE=0 */15 * * * *
EOD_SNAPSHOT_ENABLED=true
EOD_SNAPSHOT_SCHEDULE=0 0 23 * * *
```

### New variables

```bash
# Enable/disable fetch all coins job (default: true)
APALIS_FETCH_ALL_COINS_ENABLED=true
# Cron expression – 6 fields: sec min hour day month weekday
APALIS_FETCH_ALL_COINS_CRON=0 */15 * * * *

# Enable/disable EOD snapshot job (default: true)
APALIS_EOD_SNAPSHOT_ENABLED=true
APALIS_EOD_SNAPSHOT_CRON=0 0 23 * * *
```

Update your `.env` file (or deployment secrets) before rolling out this
release.

---

## New Files

| File | Purpose |
|------|---------|
| `api/src/jobs/apalis_runner.rs` | Apalis job types, handlers and `build_monitor()` helper |

## Modified Files

| File | Change |
|------|--------|
| `api/Cargo.toml` | Added `apalis = { version = "0.4", features = ["cron", "extensions"] }` |
| `api/src/jobs/mod.rs` | Exposed `pub mod apalis_runner` |
| `api/src/main.rs` | Replaced `JobScheduler` setup with `build_monitor()` + `tokio::spawn` |
| `api/.env.example` | Updated job scheduler env-var section |

---

## Architecture

```
main()
  │
  ├── DbConfig::from_env()  ──────────────────────────── DatabaseConnection
  │
  ├── jobs::apalis_runner::build_monitor(db)
  │     │
  │     ├── WorkerBuilder("apalis-fetch-all-coins")
  │     │     ├── Extension(db)           ← state injection
  │     │     ├── CronStream("0 */15 * * * *")
  │     │     └── job_fn(handle_fetch_all_coins)
  │     │
  │     └── WorkerBuilder("apalis-eod-snapshot")
  │           ├── Extension(db)
  │           ├── CronStream("0 0 23 * * *")
  │           └── job_fn(handle_eod_snapshot)
  │
  ├── tokio::spawn(monitor.run())   ← background task
  │
  └── axum::serve(...)              ← HTTP server (foreground)
```

---

## Rollback

If issues arise after deployment:

1. **Quick rollback** – set `APALIS_FETCH_ALL_COINS_ENABLED=false` and
   `APALIS_EOD_SNAPSHOT_ENABLED=false` to disable all Apalis workers without
   a redeploy.
2. **Code rollback** – revert the changes to `main.rs` to restore the
   `tokio-cron-scheduler` setup; the dependency is still present.

---

## Phase 2 (Planned)

- Remove `tokio-cron-scheduler` dependency.
- Add PostgreSQL-backed `apalis-sql` storage for job persistence and
  visibility across restarts.
- Add retry logic and a job-metrics dashboard.
