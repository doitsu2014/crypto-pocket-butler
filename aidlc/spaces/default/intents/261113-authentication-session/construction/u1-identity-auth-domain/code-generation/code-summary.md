# Code Generation Summary — U1 Identity Auth Domain

## Files changed

- `services/identity/domain/src/session.rs` — added immutable per-session `first_seen_at`, 30-day expiry calculation and cap check; session activity does not reset the cap anchor.
- `services/identity/infrastructure/src/repositories/sessions.rs` — maps and persists the first-seen anchor; session updates preserve the stored anchor.
- `services/identity/tests/domain.rs` — added regression coverage for the per-session cap boundary and refresh/activity not extending it.
- `services/identity/tests/common/fakes.rs` — preserves first-seen time when fake session upserts update activity.
- `services/identity/tests/repositories.rs` — repository integration test asserts first-seen time survives upsert.
- `services/identity/migrations/0009_session_absolute_cap.up.sql` and `.down.sql` — add/backfill/remove the session-level first-seen column. Backfill uses `last_activity_at` as the conservative available historical anchor.

## Key implementation decisions

- The 30-day absolute cap belongs to each provider session, not `User.created_at` or a user-level first-sign-in timestamp. Refreshing activity preserves the original session anchor; a new provider session receives a new anchor.
- Existing rows are backfilled from the existing `last_activity_at` value because no historical per-session creation timestamp is stored. This conservatively limits existing sessions based on their last recorded activity; it cannot reconstruct actual original sign-in time.
- Removed the proposed expiry expression index after inspection: no current query filters on the derived expiry, and the expression may be rejected as non-immutable by PostgreSQL. No unsupported cache, auth service, MFA adapter, recovery workflow, or gateway route was added.
- Existing `SessionService` already enforces session ownership, explicit confirmation for revoking the current session, provider-first revocation, no local mutation on provider failure and idempotent repeated revoke. These behaviors remain unchanged.
- Current Identity claims already carry trusted `has_exchange_credentials` and `assurance`, and `User::next_step()` represents the existing limited-session decision. Provider MFA enrolment/factor lifecycle has no Identity port or route and remains outside U1.

## Test results

- `cargo test -p identity-domain` — passed, 4 tests.
- `cargo test -p identity --lib` — passed, 13 tests.
- `cargo test -p identity --test domain` — passed, 43 tests.
- `cargo fmt --all -- --check` — passed.
- `cargo clippy -p identity --all-targets -- -D warnings` — passed.
- `cargo test -p identity --test repositories a_device_session_upserts_one_row_per_auth_session -- --ignored` — test compiled but did not execute because `TEST_DATABASE_URL` is unset (`TEST_DATABASE_URL must point at a disposable Postgres database for integration tests: NotPresent`). Migration up/down and repository round-trip remain unverified against PostgreSQL.

No coverage figure is reported: the required disposable database was unavailable, so the full coverage contract could not be measured.

## Deferred or unimplemented U1 behaviors

- U1 implements the per-session 30-day cap. Supabase's actual refresh enforcement/configuration and browser multi-tab token rotation behavior require provider/adapter and SPA work in U2/U3.
- MFA factor enrolment and recovery-code issuance remain Supabase Auth/provider-adapter work; current Identity has no port for these operations.
- Persisted single-use signup-gate authorization, support-assisted recovery cases, and their approval/re-proof lifecycle are not added: current Identity ports/contracts do not define their persistence/handler behavior. These remain explicit implementation gaps for the appropriate boundary owners rather than inferred functionality.
- First-seen alert decision has no new second history store; current `device_sessions` unique `(user_id, auth_session_id)` is the available observed-session history. Notification integration behavior remains U2/U4 work.
- Gateway declarations, SPA updates and integrated reachability/browser proof are not part of U1.

No unrelated project files were intentionally changed by this Unit.