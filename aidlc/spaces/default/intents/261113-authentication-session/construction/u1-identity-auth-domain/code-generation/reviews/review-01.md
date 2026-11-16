## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-11-15T02:31:34Z
**Iteration:** 1
**Verdict Dictated:** no

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Critical | services/identity/migrations/0009_session_absolute_cap.up.sql > UPDATE device_sessions SET first_seen_at = last_activity_at | Existing `device_sessions` rows have no creation time, only `last_activity_at`. Copying that column into `first_seen_at` starts a new 30-day window at the latest activity. A session that is already older than 30 days and was active recently stays inside the cap for up to another 30 days, which is the population the absolute cap is meant to end. The up comment calls this the first observed activity and a conservative anchor; it is the latest activity and the most permissive choice. The down migration only drops `first_seen_at` and does not delete session rows. | Do not backfill `first_seen_at` from `last_activity_at`. Set existing anchors so absolute expiry is at or before migration time, and say in the migration comment that the original sign-in time is unknown so those sessions are expired rather than extended. Add a disposable-database test that a recently active row whose history is already past 30 days is outside the cap after the up migration. | New |
| R-02 | Major | services/identity/migrations/0009_session_absolute_cap.up.sql > ALTER COLUMN first_seen_at SET NOT NULL | The new column is NOT NULL and has no default. The previous session insert names only `id`, `user_id`, `auth_session_id`, `device`, `location`, `last_activity_at`, and `revoked_at`. Identity migrates before the API and worker are updated, and `identity-api` rolls out start-first, so the old binary is the writer after this migration. That insert then fails the not-null check. `record_session` logs the failure and still resolves the principal, so sign-in continues while device-session rows stop being written until the new binary is live. | Give `first_seen_at` a default so the previous insert succeeds until every replica writes the column. Keep the backfill for existing rows separate from that default. Do not leave a NOT NULL column with no default in front of the old writer. | New |
| R-03 | Major | aidlc/spaces/default/intents/261113-authentication-session/construction/u1-identity-auth-domain/code-generation/traceability.json > coverage rows AC2.1.3, BR1.3, and BR1.4 | Those rows are marked done against `services/identity/tests/domain.rs`. The tests only show that an in-memory session expires 30 days after `first_seen_at` and that `touch` does not move that timestamp. No Identity path refuses refresh or access at the cap: `is_within_absolute_cap` is called only from tests, and principal recording still builds a session at the current time and upserts it on every authenticated request. BR1.4 also requires a successful refresh to rotate the provider refresh token. That behavior is not in this change, and AC2.1.1 and AC2.1.2 are marked deferred in the same file. | Do not mark AC2.1.3, BR1.3, or BR1.4 done until a session path refuses refresh at the 30-day boundary and a successful refresh keeps the original anchor. Until that exists, mark the rows deferred and name the missing refresh-eligibility behavior, not the domain predicate tests. | New |
| R-04 | Minor | services/identity/tests/common/fakes.rs > FakeSessions::update | Postgres `update` does not write `first_seen_at`, and the fake upsert keeps the stored anchor. The fake `update` replaces the stored session with the incoming value, so a different `first_seen_at` on that object would stick in tests and not in Postgres. The repository test checks the anchor after upsert, not after update. | Keep the stored `first_seen_at` in the fake update, and assert on the disposable-database session test that a later revoke or activity update does not change it. | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| cargo test -p identity-domain | PASS. 4 passed, 0 failed, 0 ignored. | Matches the code summary. Covers the in-memory cap predicate only. |
| cargo test -p identity --lib | PASS. 13 passed, 0 failed, 0 ignored. | Matches the code summary. These tests do not exercise the session cap. |
| cargo test -p identity --test domain | PASS. 43 passed, 0 failed, 0 ignored. | Matches the code summary. The two new session tests check the in-memory anchor, not refresh refusal. |
| cargo test -p identity --test repositories a_device_session_upserts_one_row_per_auth_session -- --ignored | FAIL. Panicked at services/identity/tests/common/mod.rs:656: TEST_DATABASE_URL must point at a disposable Postgres database for integration tests: NotPresent. No database URL was set, and no shared database was used. | The upsert and migration round-trip are not proven. This agrees with the code summary. A disposable TEST_DATABASE_URL is required before that test can run. |

### Summary

The change stays inside Identity, the source manifest matches the seven changed application files, and Postgres upsert and update leave `first_seen_at` unchanged. The up migration is not safe to ship: it restarts the 30-day cap from last activity, and the NOT NULL column rejects the previous session insert until the new binary is live. The rows that say the cap and refresh rotation are done overstate a predicate that nothing calls.
