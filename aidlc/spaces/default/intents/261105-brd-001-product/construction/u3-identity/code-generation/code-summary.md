# Code Summary — u3-identity

## What was built

The Identity service as a five-package Cargo workspace under `services/identity/`, matching the
walking skeleton's shape and the approved plan step for step. Tests came first at every layer:
domain (25), application (19), HTTP (14), worker (3), and the adapter suite (9, `#[ignore]`d
without a disposable Postgres). The unit-scoped command is
`cargo test --manifest-path services/identity/Cargo.toml`; all 61 runnable tests pass.

| Package | Responsibility |
|---|---|
| `identity-domain` | `User` lifecycle (pending → active, deleted terminal), validated preferences, append-only `ConsentRecord` history, `SupportAccessGrant` (reason, distinct admin, 60-minute window), `DeviceSession`, `JurisdictionRule` gate, `ShreddingKey`, and the `ExportRequest` / `DeletionRequest` state machines with monotonic progress and retryable deletion steps |
| `identity-application` | JIT provisioning and principal resolution (BR1.1–BR1.5), consent (BR3.1–BR3.2), preferences (BR3.3), session list/revoke (BR8.1–BR8.2), support request/approve/revoke/check (BR4.1–BR4.4), the sign-up gate (BR7.1–BR7.3), export request/status and its worker (BR5.1–BR5.4), deletion request/status and its worker (BR6.1–BR6.7), and the shared `Trail` audit path |
| `identity-infrastructure` | sqlx repositories behind the port traits, `identity.protect`/`unprotect`/key-lifecycle calls, JWKS validation with the `aal`/`email_confirmed`/`has_exchange_credentials`/`session_id` claims, HS256 gate tokens, the C10 audit client, the Supabase Auth admin client, the C5/C7/C8 contributor client, the private Storage adapter, the wake transport, migrations, and the readiness probe |
| `identity-http` | Axum router, DTOs, RFC 7807 problems, correlation id, security headers, JWT middleware, service-token middleware, Prometheus text metrics, and the worker's internal listener |
| `identity` | Composition root and the two binaries: `identity-api` (3 replicas) and `identity-worker` (1 replica, wake endpoint plus 60-second sweep) |

## HTTP surface

| Method and path | Identity | Behaviour |
|---|---|---|
| `POST /v1/signup-gate` | public (pre-auth) | applies the jurisdiction rule; returns `disclosuresRequired` until acknowledged, then a 10-minute signed `gateToken`; a blocked or under-age declaration is a 403 problem |
| `GET /v1/principals/{authUserId}` | JWT; the path subject must equal the validated token subject | provisions on first use, refreshes the role, moves a verified user to active, returns `{userId, role, status, nextStep}`; a deleted account is a 403 |
| `POST /v1/consent` | JWT | appends one decision for the caller; 201 with the record |
| `GET /v1/consent/{userId}` | JWT, owner or admin | newest analytics and marketing decisions; absent means not granted |
| `PATCH /v1/users/{userId}/preferences` | JWT, owner | validates locale, base currency, timezone; 422 names the field path; 200 returns the stored primitives |
| `GET /v1/users/{userId}/sessions` | JWT, owner or admin | Identity's `DeviceSession` view with device, coarse location, last activity, and `current` |
| `DELETE /v1/users/{userId}/sessions/{sessionId}` | JWT, owner or admin; `confirmCurrent=true` for the current session | calls Supabase Auth first, then marks the row; 409 until the current session is confirmed; 503 when Auth is unavailable |
| `POST /v1/exports` | JWT, owner | creates or reuses the active request, wakes the worker, 202 with progress |
| `GET /v1/exports/{exportId}` | JWT, owner | progress; when ready and inside the 24-hour window, an owner-only signed link |
| `POST /v1/privacy/deletions` | JWT, owner | creates or reuses the active request, wakes the worker, 202 |
| `GET /v1/privacy/deletions/{deletionId}` | JWT, owner | the request state, retained categories, and failure reason |
| `POST /v1/support-grants` | JWT + `support` | reason-recorded request, stored `pending_approval` |
| `POST /v1/support-grants/{grantId}/approve` | JWT + `admin` | sets the distinct approver, expiry (60 minutes), and `active` |
| `DELETE /v1/support-grants/{grantId}` | JWT + `admin` | `revoked`; terminal |
| `GET /v1/internal/support-grants/{grantId}/check?userId=` | internal service token | verifies the grant on the metadata read path; the read audit event is written before the allow or deny |
| `POST /v1/internal/run/audit-privacy-sweep` | internal service token | C11 run endpoint; wakes the worker, 202 |
| `GET /healthz`, `GET /readyz`, `GET /metrics` | public | readiness checks Postgres and `_sqlx_migrations`; metrics are low-cardinality |

Deny-by-default: only the routes above exist; anything else is a 404 problem document. A missing,
invalid, or unavailable identity provider is refused before any write.

## Workflows and rules

- **WF1/WF2** — principal resolution, JIT provisioning, verified → active, and the MFA gate are in
  `services/identity/application/src/principal.rs` and `domain/src/user.rs`. The role comes only
  from `app_metadata.role`; unknown is `user`. A deleted account is refused with a plain
  explanation.
- **WF3** — the session view is upserted on every authenticated request from the token's
  `session_id` plus the gateway's `x-device-label`/`x-geo-location` (never the full IP). Revoking
  the current session requires confirmation.
- **WF4** — consent is deny-by-default and append-only; preferences are validated and returned as
  stored primitives.
- **WF6** — the gate is enforced server-side before Supabase Auth: a blocked or unknown country
  and a below-minimum age are refused, disclosures must be acknowledged, and the signed token is
  verified by the pre-sign-up hook.
- **WF7** — a support grant needs a non-blank reason, an admin distinct from the requester, a
  60-minute window, and an audit event ahead of the read; an audit failure refuses the read.
- **WF8** — the export worker calls Account, Ledger, and Portfolio in a fixed order, records
  progress monotonically, refuses a credential-shaped archive field, stores the archive in the
  private bucket, and serves an owner-only 24-hour signed link. A failed contribution fails the
  request and never stores a partial archive.
- **WF9** — the deletion worker anonymises each contributor, deletes the Supabase Auth identity,
  anonymises Identity's own fields and consent history, destroys the shredding key last, and
  audits completion before marking it. A retry resumes the remaining steps; completed destruction
  is never repeated.

## Database

`services/identity/migrations/0001_init.up.sql` creates:

- `users`, `user_personal`, `shredding_keys`, `consent_records`, `support_access_grants`,
  `device_sessions`, `export_requests`, `deletion_requests`, and `jurisdiction_rules`, with the
  state-shape checks, the distinct-approver and 60-minute-window checks, and the performance-design
  indexes;
- a partial unique index for one active deletion and one active export per user;
- `identity.protect(text, bigint)` and `identity.unprotect(bytea, bigint)` as SECURITY DEFINER
  functions over pgsodium deterministic AEAD, plus `identity.create_shredding_key(text)` and
  `identity.destroy_shredding_key(bigint)`. The service passes a key id only; key material never
  leaves Vault;
- `user_personal` stores an unsalted SHA-256 of the Supabase subject for the indexed lookup and
  the deterministic ciphertext for the Auth admin delete. Destroying the key makes the ciphertext
  unreadable; the hash is a pseudonym, not a personal identifier;
- the launch jurisdiction seed: VN, GB, and all 27 EU member states, each `allowed`, `minimumAge`
  18, `disclosuresRequired` true; any other country is blocked until Compliance adds it;
- roles `identity_app`, `identity_readonly`, and `identity_migrator` with exactly the service's
  grants; the roles are `NOLOGIN` (the platform Vault bootstrap owns LOGIN and passwords).

`infra/sql/identity-schema.sql` is the bootstrap: schema, roles, search paths, and the pgsodium
extension when available. The down migration refuses to drop a schema that holds user records.

## Worker

`identity-worker` runs `PrivacyWorker` (`ExportWorker` then `DeletionWorker`) on a 60-second
sweep and immediately on `POST /internal/wake`. Both workers claim with one atomic
`UPDATE ... FOR UPDATE SKIP LOCKED` and a 60-second lease, so a dead worker's record is re-claimed
without double execution. Expired ready exports have their archive deleted on the same sweep.

## Tests and gates

- `cargo test --manifest-path services/identity/Cargo.toml` — 61 passed, 0 failed, 9 ignored.
- `cargo fmt --all -- --check` — clean.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `scripts/check-boundaries.sh` — `boundaries OK (20 packages checked)`; `domain` imports no
  framework, `application` imports neither adapter crate, and no crate imports `http`.
- The adapter suite (`tests/repositories.rs`, 9 tests) is `#[ignore]`d without
  `TEST_DATABASE_URL`; run it against a disposable Supabase-compatible database with pgsodium.

## Deployment assets

- `services/identity/Dockerfile.api` and `Dockerfile.worker`: multi-stage, non-root (UID 10002),
  read-only root filesystem via the Swarm settings, digest-pinned `rust:1-slim-bookworm` and
  `debian:bookworm-slim` bases.
- `infra/swarm-stack.yml`: `identity-api` (3 replicas, 0.5 vCPU / 512 MB, internal only) and
  `identity-worker` (1 replica), health checks on `/healthz`, `failure_action: rollback`, the
  internal overlay declared `encrypted: true`, and Docker-secret references for the database DSN,
  the service-token registry, the Supabase service-role key, and the gate-token key.
- `.env.example` records the non-secret names and the secret-file paths.

## Deviations and open concerns

1. **Preference validation is lexical, not registry-backed.** `BaseCurrency` accepts any three
   upper-case ASCII letters and `TimeZone` accepts the IANA lexical shape, rather than checking the
   full ISO 4217 and tz databases. The design says "valid ISO 4217" and "valid IANA"; a follow-up
   should add a static ISO 4217 allowlist or a tz-database dependency. The error path and field
   reporting are already in place.
2. **The adapter suite needs pgsodium.** It is `#[ignore]`d locally; CI must provision a
   Supabase-compatible database with the extension, or the protect/unprotect and key-lifecycle
   assertions do not run. No adapter test was executed in this environment.
3. **The pgsodium call shape is assumed.** The migration calls
   `pgsodium.crypto_aead_det_encrypt/decrypt`, `pgsodium.create_key(name => ...)`, and
   `DELETE FROM pgsodium.key`; verify these against the project's pgsodium version during the
   first migration run.
4. **The Supabase Auth single-session logout route is an assumption.** `HttpAuthAdmin` calls
   `POST /admin/users/{user}/sessions/{session}/logout`; if the project's Auth version exposes only
   a user-level logout, that one adapter changes and the session semantics stay the same.
5. **The C5/C7/C8 privacy operations are contract change requests.** The contributor client uses
   the shapes recorded in the contract amendment; the owning units must accept them. Until then
   the worker cannot complete a real export or deletion.
6. **Coverage is unmeasured.** The team's 80% per-crate target is not verified because the coverage
   tool is not chosen; it must be checked in CI. The suite's volume is 5–8 tests per component.
7. **Monitoring configuration is deferred.** The metrics, structured logs, and correlation ids are
   emitted, but the Prometheus alert rules, Grafana dashboards, and OTLP export are deployment
   configuration and are recorded as `Deferred` in `traceability.json`.
8. **No frontend behaviour.** The SPA screens for US1.1, US3.5, US7.x, and US9.3 belong to
   u2-web-app; this unit exposes the internal surface only.

## Build and Test loop-back repair (2026-11-09)

The adapter suite was run against a pgsodium-capable PostgreSQL 15 (Supabase image). Three
defects were repaired in this unit:

1. **Schema-qualified SQL** — `keys.rs` and `users.rs` called `identity.create_shredding_key`,
   `identity.destroy_shredding_key`, `identity.protect` and `identity.unprotect`. The adapter
   harness runs migrations into a disposable schema with `search_path` set to it, and the
   migration creates unqualified objects on purpose, so the `identity.` prefix is removed in both
   repositories and in the two test calls.
2. **pgsodium key id column** — this pgsodium version's `pgsodium.key` has `id uuid` and
   `key_id bigint`; `create_shredding_key` now selects `key_id`, and `destroy_shredding_key`
   deletes `WHERE key_id = target_key_id` (the parameter was renamed to avoid the ambiguous
   reference).
3. **Test environment** — the adapter run needs a pgsodium-capable database and the test role to
   be a member of `identity_app`, with EXECUTE on the bigint-keyed AEAD functions and
   SELECT/DELETE on `pgsodium.key`.

Result: 8 of the 9 adapter tests pass. The remaining assertion — "a destroyed key no longer
decrypts the subject" — cannot pass in this pgsodium build: deleting the `pgsodium.key` row does
not evict the key from pgsodium's in-memory key cache, so the same cluster still decrypts until
it restarts. That is recorded as an environment/design finding for Build and Test, not a target
that was lowered.
