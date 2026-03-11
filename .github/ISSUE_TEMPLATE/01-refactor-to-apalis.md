# 🛠️ Refactor Job Runner to Use Apalis

## 📋 Overview

Track the refactoring of the current job system to use [Apalis](https://github.com/rapoth/apalis), while preserving all existing business logic.

---

## 🎯 Current Situation

| Aspect | Current | Target |
|--------|---------|--------|
| Scheduler | `tokio-cron-scheduler` | `apalis` |
| Cron format | 7 fields | 6 fields |
| Storage | In-memory | PostgreSQL-backed |
| Visibility | Minimal | Full queue visibility |

---

## ✅ What Stays the Same

| Feature | Status |
|---------|--------|
| Job business logic | Unchanged |
| API endpoints | Unchanged |
| Database operations | Unchanged |
| Logging/metrics | Preserved |
| Manual triggers | Same behavior |

---

## 🔄 Cron Format Changes

| Action | Current | Apalis |
|--------|---------|--------|
| Every 15 min | `0 */15 * * * *` | `*/15 * * * *` |
| Daily 23:00 | `0 0 23 * * *` | `0 0 23 * * *` |
| Hourly | `0 0 * * * *` | `0 * * * *` |

---

## 📝 Implementation Plan

### Phase 1: Core Migration

- Add Apalis dependencies to `Cargo.toml`
- Create `apalis_runner.rs` (same interface as `runner.rs`)
- Update job functions for Apalis context
- Update `main.rs` for worker registration
- Test both systems in parallel
- Update documentation

### Phase 2: Cleanup

- Remove `tokio-cron-scheduler`
- Add job queue visibility
- Implement retry logic
- Add job metrics dashboard

---

## 🧪 Testing Strategy

- Unit tests: Verify same `JobResult` output
- Integration tests: Match cron behavior
- Regression tests: API endpoints unchanged

---

## 🚀 Success Criteria

- All existing jobs run same frequency and results
- No change to API endpoints or manual triggers
- Job timing/logging/metrics preserved
- Database operations remain idempotent

---

## 🏗️ Labels

`type: refactoring`, `priority: medium`, `area: backend`, `area: jobs`, `docs: required`

---

## 📦 Dependencies

```toml
# In api/Cargo.toml

# Apalis core
apalis = { version = "0.4", features = ["full"] }

# PostgreSQL storage backend
apalis-sql = { version = "0.4" }
tokio-postgres = { version = "0.7", features = ["with-uuid-0_8", "with-serde_json-1"] }

# Keep tokio-cron-scheduler for migration phase
tokio-cron-scheduler = "0.13"
```

---

## 📂 New Files

- `api/src/jobs/apalis_runner.rs` - New runner with Apalis integration
- `docs/migration/01-apalis-migration.md` - Step-by-step migration guide

---

## 🗂️ Modified Files

- `api/src/main.rs` - Update job registration to use Apalis
- `api/Cargo.toml` - Add Apalis dependencies
- `api/.env.example` - Update cron format examples

---

## 🔄 Migration Steps

### 1. Add Dependencies

```toml
# Apalis core
apalis = { version = "0.4", features = ["full"] }

# PostgreSQL storage backend
apalis-sql = { version = "0.4" }
tokio-postgres = { version = "0.7", features = ["with-uuid-0_8", "with-serde_json-1"] }

# Keep tokio-cron-scheduler for migration
tokio-cron-scheduler = "0.13"
```

### 2. Create `api/src/jobs/apalis_runner.rs`

```rust
// Copy from runner.rs, adapt to Apalis context type
```

### 3. Update `api/src/main.rs`

```rust
// Replace tokio-cron-scheduler setup with Apalis worker registration
```

### 4. Update `.env.example`

```bash
APALIS_FETCH_ALL_COINS_ENABLED=true
APALIS_FETCH_ALL_COINS_CRON="*/15 * * * *"  # Changed from: 0 */15 * * * *

APALIS_EOD_SNAPSHOT_ENABLED=true
APALIS_EOD_SNAPSHOT_CRON="0 0 23 * * *"  # Same format (no year field)
```

---

## 📊 Job Registry

| Job | Function | Cron (Current) | Cron (Apalis) |
|-----|----------|---------------|---------------|
| fetch_all_coins | `fetch_all_coins::fetch_all_coins()` | `0 */15 * * * *` | `*/15 * * * *` |
| portfolio_snapshot | `portfolio_snapshot::create_all_portfolio_snapshots()` | `0 0 23 * * *` | `0 0 23 * * *` |

---

## 🛡️ Rollback Plan

If issues arise:

1. **Quick rollback:** Comment out Apalis registration in `main.rs`
2. **Feature flag:** Use `USE_APALIS=false` environment variable
3. **Feature flag (Cargo):** Add `apalis` feature flag, default off

---

## 📚 Related Documentation

- [Apalis Documentation](https://docs.rs/apalis)
- [Apalis GitHub](https://github.com/rapoth/apalis)
- `docs/api/jobs.md` - Current job system documentation

---

## 📌 Notes

- Apalis uses standard UNIX cron format (6 fields: sec min hour day month weekday)
- tokio-cron-scheduler uses 7-field format (includes year)
- Database-backed queue allows for distributed job processing
- Full visibility into job queue state via database queries

---

## 🎉 Benefits After Migration

- Better job visibility and management
- Distributed job processing capability
- Persistence across application restarts
- Standard cron format (more tools compatible)
- Better retry handling with exponential backoff
