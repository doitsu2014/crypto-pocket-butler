# Code Summary — u9-notification

## What was built

The Notification service as a four-crate Clean Architecture Cargo workspace under
`services/notification/`, with two binaries wired from `src/lib.rs`:

| Binary | Port | Purpose |
|---|---|---|
| `notification-api` | 8099 | C9 submit and anonymise, C1 feed and preferences, `/healthz`, `/readyz`, `/metrics` |
| `notification-worker` | 8100 | the single leased delivery poller (5 s), C11 delivery and retention runs, probes, metrics |
| `notification-migrate` | — | one-shot release job under `notification_migrator` |

| Layer | Responsibility |
|---|---|
| `domain` | closed type set with per-type payload allow-lists, the content guard, preference defaults and the mandatory-email rule, the stored alert and delivery attempt, retry bounds, the alert-status derivation, the twelve-month cutoff, C10 action names |
| `application` | submit/dedupe, preferences view/save, feed keyset page, delivery with retries and per-run provider cap, retention sweep, erasure, and the C10 trail with bounded retry |
| `infrastructure` | sqlx repositories (alerts, attempts, preferences, wake flag + lease), JWKS validation, C3 contact client, provider client, C10 client, service-token registry, readiness |
| `http` | Axum routers, DTO mapping at the boundary, RFC 7807 problems, JWT and service-token middleware, Prometheus metrics |
| `migrations` | `0001_init` up/down: four tables, roles `notification_svc` and `notification_migrator`, grants, feed/due/retention indexes |

The alerts table is the queue; there is no message queue. Notification never reads another service's
tables: the verified email address is read from Identity's C3 at delivery time and is never stored,
logged, or traced.

### Behaviour delivered

- **BR1.1 idempotency dedupe** — `alerts.idempotency_key` is unique; `INSERT ... ON CONFLICT DO
  NOTHING RETURNING id` distinguishes new from duplicate, and a duplicate returns `deduplicated`
  with the existing alert id and sends nothing (`src/repositories/alerts.rs`,
  `application/src/submit.rs`).
- **BR1.2 guardrail crossings** — Portfolio owns the inside/outside state and the crossing key;
  Notification dedupes a re-submitted key and makes a new key (a new crossing) a new alert.
- **BR2.1–BR2.3 preferences** — a missing row resolves to the documented default (in-app on for
  every type; email on only for `brief`); a saved change is read on the next delivery (no cache);
  the view always returns all five types and both channels; disabling a mandatory email is refused
  with the field-level reason `security.email` / `key_revoked.email` and audited
  (`domain/src/preference.rs`, `application/src/preferences.rs`).
- **BR3.1 mandatory email** — `security` and `key_revoked` always queue an email regardless of the
  stored preference, and the submit sets the durable `wake_requested_at` flag; a stored `false` is
  kept but resolved to `true` at delivery.
- **BR4.1 bounded retries** — 3 attempts per channel (initial + 1 min + 5 min), each recorded;
  the third transient failure, or any permanent failure, is `abandoned`; the alert becomes
  `delivered`, `partially_delivered`, or `failed` from the enabled channels' outcomes.
- **BR4.2 verified address only** — C3 `GET /v1/users/{userId}/contact` is called per delivery with
  a 150 ms budget; a missing/unverified address (and a 404) is a permanent `address` error; a
  network or 5xx failure is transient and retried.
- **BR4.3 email off keeps the brief in-app** — a disabled brief email produces no email attempt.
- **BR5.1 content guard** — only the per-type allow-list is stored; a value matching an email, a
  `0x` full address, a 32–44 base58 key, an IPv4/IPv6 literal, a token-shaped value, or the `sk-` /
  `pk-` / `Bearer` / `eyJ` families is dropped and its field name recorded in `sanitisedFields`; the
  notification still delivers. Canonical UUIDs (the allow-listed `portfolioId`, `assetId`,
  `accountId`, `briefId` values) are exempt so identifiers survive.
- **NFR1.5 audit ordering** — the alert commits first; the trail then writes C10 with three attempts
  and 1 s / 2 s / 5 s pauses, counting a persistent failure. Mandatory delivery/abandonment and
  mandatory-preference refusals are audited with actor, action, target `type:alertId`, timestamp,
  allow-listed context, and outcome.
- **NFR2.5 handoff** — the API only writes the durable wake flag; the worker polls the flag and the
  queue every 5 s and holds the single-flight lease (`LEASE_SECONDS = 600`) while it runs.
- **NFR9.2 authorization** — owner-only feed/preferences (JWT + `X-Auth-Subject` agreement plus a
  path-owner check); C9 submit by per-caller service tokens (`alerts.submit`); erasure by Identity
  only (`privacy.anonymise`); C11 runs by `runs`; smoke by `smoke`.
- **NFR13.1 erasure** — one fresh UUIDv4 replaces `user_id` on the user's alerts and delivery
  attempts in one transaction; no mapping is retained, so a second call finds nothing.
- **NFR17 metrics** — the eight required series are exported: request duration histogram, request
  errors, alerts by type/status, attempts by channel/status, duplicates by type, mandatory
  delivery by type/status, audit failures, and `notification_health`.
- **NFR20.4/20.6** — batches of 200, provider concurrency 20, 1,200 email calls per run with the
  excess deferred a minute, and a nightly retention sweep deleting attempts then alerts older than
  twelve months.

## Review findings addressed (review-01)

| Finding | Fix |
|---|---|
| R-01 Major: the five-second poll recorded no delivery metrics | `record_delivery_report(&Metrics, &DeliveryReport)` now lives in `services/notification/http/src/metrics.rs`; both the C11 handler and `spawn_delivery_poll` call it. `services/notification/http/tests/http.rs > a_poll_driven_run_records_delivery_metrics` runs `poll` and asserts `notification_alerts_total`, `notification_attempts_total` and `notification_mandatory_delivery_total`. |
| R-02 Major: a type disabled for in-app still appeared in the feed | `services/notification/infrastructure/src/repositories/alerts.rs > list_for_user` now carries `NOT EXISTS (… channel = 'in_app' AND enabled = false)` (BR2.1, WF4.2); the application and HTTP fakes apply the same rule. Tests: `feed_hides_a_type_whose_in_app_switch_is_off`, `feed_still_shows_other_types_when_one_is_off`, `the_feed_hides_a_type_disabled_for_in_app` (HTTP), `the_feed_hides_a_type_disabled_for_in_app` (adapter, ignored without a DB). |
| R-03 Major: C3 contact endpoint not served | Identity now owns the address: `users.email` / `users.email_verified` (migration `0002_contact`), refreshed from the validated token (`domain/src/user.rs > apply_claims`, cleared by `anonymise`), and `GET /v1/users/{userId}/contact` on the internal service-token router (`http/src/handlers.rs > get_contact`, `application/src/principal.rs > contact`). Notification's client already targets it. End-to-end test: `a_mandatory_security_email_is_delivered_through_the_c3_contact` (C3 read once, provider receives the address, alert delivered, audit written). Identity tests: `c3_contact_returns_the_verified_address`, `an_unconfirmed_contact_answers_not_verified`, `a_deleted_users_contact_is_not_found`, plus the three HTTP route tests. |
| R-04 Minor: histogram and 5xx counter carried only `service` | `notification_request_duration_seconds` and `notification_request_errors_total` now carry bounded `route` (the matched `MatchedPath` pattern) and `status` labels; `record_metrics` moved to `route_layer` so the pattern is available. Test: `request_metrics_are_labelled_by_route_and_status`. |
| R-05 Minor: per-run email cap was load-then-increment | `DeliveryControl::reserve_email(owner, cap)` is one `UPDATE … WHERE lease_owner = $1 AND emails_sent < $2 RETURNING emails_sent` on `delivery_control`; the new lease resets `emails_sent` to 0. The in-memory counter is gone. |
| R-06 Minor: one `now` per run | `deliver` captures `self.clock.now()` per alert and uses it for the attempt timestamp, the retry deadline, and the status update. |
| R-07 Minor: audit context narrowed | C10's allow-list (`services/audit/domain/src/context.rs`) gained `channel` and `sanitisedFields`; the trail sends `channel=email` and the removed field names (never their values) on mandatory delivery/abandonment. The C3 end-to-end test asserts both keys. |
| R-08 Minor: preference save not transactional | `PreferenceRepository::upsert_all` writes every row in one transaction; `PreferenceService::save` calls it once. |
| R-09 Minor: four traceability targets | NFR1.6 now points at the infrastructure design's Supabase backup/PITR element (joint with the platform); NFR20.1 at the infrastructure design's replica/scaling element plus `infra/swarm-stack.yml`; NFR20.2 at the sizing element plus the migration indexes; NFR17.6 names its owner and stage. |
| R-10 Minor: adapter suite unrun | Re-assessed below (D-09): still unrun here, now recorded as 14 ignored tests; no disposable Postgres is reachable in this worktree (`TEST_DATABASE_URL` unset, no Docker daemon). |
| R-11 Minor: no owner for the gateway route and OTLP | `code-generation/follow-ups.md` records the U1 API Gateway C1 route wiring (with route paths and the `notification-api:8099` target) and the OTLP exporter as a U1 Operation-phase observability item. |
| R-12 Minor: service-token registry never seeded | `infra/sql/notification-service-tokens.sql` is the release-job step: it inserts the SHA-256 of each caller token (gateway, identity, account, portfolio, reporting, job-orchestration) with `ON CONFLICT DO NOTHING`, and documents insert-then-delete rotation. The registry primary key became `(service, token_sha256)` so both hashes resolve during a rotation. The smoke token stays in-process from its Docker secret. |
| R-13 Major (pass 2): C3 contact accepted any valid service token | Identity's service middleware now authorises per caller: `require_contact` allows only `notification`, `require_run` only `job-orchestration`, `require_support_check` only `gateway`; a valid token issued to any other service gets 403 (`services/identity/http/src/middleware.rs`, `routes.rs`). The old any-known-token `require_service` is gone, so no internal route is intentionally shared. Tests: `the_contact_endpoint_refuses_another_services_token`, `the_c11_sweep_endpoint_refuses_another_services_token`, `the_support_grant_check_is_gateway_only`, plus the three allowed-caller tests. |
| R-14 Minor (pass 2): the C3 caller token is not seeded | `identity_service_tokens` must include a `notification` entry equal to `notification_outbound_token`; recorded in `infra/swarm-stack.yml` (secret header and both Identity environments), `.env.example`, and `follow-ups.md`, including the rotation order. |

## Verification output

All commands were run in this worktree after the review fixes.

```
$ cargo test -p notification-domain -p notification-application -p notification-infrastructure -p notification-http
   Running tests\domain.rs        -> ok. 37 passed; 0 failed; 0 ignored
   Running tests\application.rs   -> ok. 52 passed; 0 failed; 0 ignored
   Running tests\http.rs          -> ok. 36 passed; 0 failed; 0 ignored
   Running tests\repositories.rs  -> ok. 0 passed; 0 failed; 14 ignored
   (all lib and doc-test targets: 0 tests)

$ cargo test -p identity
   Running tests\application.rs   -> ok. 22 passed; 0 failed; 0 ignored
   Running tests\domain.rs        -> ok. 28 passed; 0 failed; 0 ignored
   Running tests\http.rs          -> ok. 21 passed; 0 failed; 0 ignored
   Running tests\worker.rs        -> ok. 3 passed; 0 failed; 0 ignored
   Running tests\repositories.rs  -> ok. 0 passed; 0 failed; 9 ignored

$ cargo test -p audit            (C10 allow-list extension)
   -> 42 + 30 + 25 + 4 passed; 0 failed; 14 ignored
```

Test volume by suite:

| Crate | Suite | Tests |
|---|---|---|
| notification-domain | `tests/domain.rs` | 37 |
| notification-application | `tests/application.rs` | 52 |
| notification-http | `tests/http.rs` | 36 |
| notification-infrastructure | `tests/repositories.rs` | 14 (ignored; need `TEST_DATABASE_URL`) |
| **Executed by the unit command** | | **125 passing, 0 failing** |
| identity (C3 contact) | `tests/application.rs` 22, `tests/domain.rs` 28, `tests/http.rs` 21, `tests/worker.rs` 3 | **74 passing, 0 failing**; 9 adapter tests ignored |
| audit (context allow-list) | application 42, domain 30, http 25, worker 4 | **101 passing, 0 failing**; 14 adapter tests ignored |

```
$ cargo fmt --all -- --check        -> FMT OK
$ cargo clippy --workspace --all-targets -- -D warnings -> clean
$ cargo build -p notification --bins -> clean
```

### Test-first evidence

Each layer was written test-first with the exact unit command run at each red step:

| Step | Red | Green |
|---|---|---|
| Data model + domain | `cargo test -p notification-domain` — `E0432` unresolved imports, 4 compile errors | 37 pass |
| Business logic + repositories | `cargo test -p notification-application` — 5 compile errors | 52 pass |
| Repository adapters | `cargo test -p notification-infrastructure` — 68 compile errors | compiles; 14 integration tests ignored |
| API | `cargo test -p notification-http` — unresolved imports | 36 pass |

## Deviations and follow-ups

| # | Item | Detail |
|---|---|---|
| D-01 | **C3 contact now served** | Resolved with R-03. Identity stores `users.email` / `users.email_verified`, refreshes them from the validated Supabase token, clears them on deletion, and serves `GET /v1/users/{userId}/contact` to service-token callers on the internal router. Notification's 404/`verified:false` handling remains as the permanent-address path for a user with no confirmed address. |
| D-02 | **C10 context extended** | Resolved with R-07: C10 accepts `channel` and `sanitisedFields`, and the trail sends them. A future emitter must stay inside the same eight-key allow-list. |
| D-03 | **System-actor actions** | C10 rejects a `system` actor unless the action is on its allow-list, so the erasure audit uses the allow-listed `system.privacy.deletion` rather than a Notification-specific action. Delivery/abandonment events are user-attributed (`actorId` = the recipient). |
| D-04 | **Port consistency with existing consumers** | The infrastructure design fixes the API at 8099 and the worker at 8100. The pre-existing `NOTIFICATION_URL` defaults in `services/account/src/config.rs`, `infra/swarm-stack.yml`, and `.env.example` pointed at 8094, which is the market-data worker's port. They were moved to 8099 (three files, two occurrences each in the first two) as a deployment-consistency change in this unit. The gateway's C1 route wiring is recorded for U1 in `follow-ups.md`. |
| D-05 | **Extra C11 job** | The design lists only `notification-deliver`. The worker also exposes `POST /v1/internal/run/notification-retention` (job `notification-retention`), mirroring analytics/audit, so Job Orchestration can schedule the nightly twelve-month sweep the scalability design requires. |
| D-06 | **Audit retry counts** | NFR1.5 says "3 attempts (1 s, 2 s, 5 s)". Implemented as three sends with a pause after each failure (1 s, 2 s, 5 s), matching the existing `ledger` trail, so the last pause precedes the persistent-failure count. |
| D-07 | **`content` error category reserved** | BR5.1 always delivers after sanitising, so no production path assigns the `content` category; it remains a valid stored value (schema check, domain round-trip test) for a future undeliverable-content rule. |
| D-08 | **NFR17.6 OTLP spans (owner: U1)** | No service in this repository wires an OpenTelemetry exporter yet (all use `tracing-subscriber` JSON only). U1 owns the shared exporter as an Operation-phase observability item (recorded in `follow-ups.md`); Notification emits structured events carrying ids, type, channel, and status only, and attaches spans when it lands. |
| D-09 | **Integration tests unrun** | The 14 `notification-infrastructure` adapter tests (and identity's 9) are `#[ignore]`d and need `TEST_DATABASE_URL`. No disposable Postgres is reachable in this worktree (`TEST_DATABASE_URL` unset, no Docker daemon), so they were compiled but not run. They remain a release gate: CI must run `cargo test -p notification-infrastructure -- --ignored` (and the identity equivalent) against a disposable schema before the unit is called complete. |
| D-10 | **Boundary gate not run** | `scripts/check-boundaries.sh` needs `python3`, which is absent in this shell. The boundaries were checked by inspection: `domain` has no local or framework dependency, `application` depends on `domain` only (no axum/sqlx/reqwest/tower), `infrastructure` depends on `domain` + `application` (no http), `http` on `domain` + `application`; DTOs live only in `http`. |
| D-11 | **No pipeline definition in this repo** | The deployment step for R-12 lives in `infra/sql/notification-service-tokens.sql`; when the CI/release pipeline is configured (still unresolved in team memory) it must run that file after `notification-migrate` with the six caller-token digests. The C3 caller entry (R-14) is recorded in the same places: `identity_service_tokens` maps `notification` to `notification_outbound_token`. |
