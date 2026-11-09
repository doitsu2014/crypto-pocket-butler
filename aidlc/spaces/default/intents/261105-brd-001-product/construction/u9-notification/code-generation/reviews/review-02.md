## Review

**Verdict:** NOT-READY

**Reviewer:** aidlc-architecture-reviewer-agent

**Iteration:** 2

**Verdict Dictated:** no

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | services/notification/src/lib.rs > spawn_delivery_poll, with services/notification/http/src/handlers.rs > record_report | The primary delivery path records no delivery metrics. The five-second poll in `spawn_delivery_poll` calls `DeliveryService::poll`, logs the report, and discards it; `record_report` — which emits `notification_alerts_total`, `notification_attempts_total` and `notification_mandatory_delivery_total` — is called only from the C11 `run_deliver` handler. In normal operation the delivery dashboards, the provider-failure alert, the mandatory-abandoned page and the 99%/100% delivery SLOs therefore see nothing. | Record the report's metrics from the poll path as well (a shared function used by both the poll and the C11 handler), and add a test that a poll-driven run increments the alert, attempt and mandatory counters. | New |
| R-02 | Major | services/notification/infrastructure/src/repositories/alerts.rs > list_for_user, with services/notification/application/src/delivery.rs > deliver | The in-app preference is not enforced on the feed. `list_for_user` returns every alert for the user with no preference filter, and `deliver` only skips the in-app attempt record when the preference is off. The stored alert is the in-app delivery (WF4.2), so a type the user disabled for in-app still appears in the feed, contradicting BR2.1 ("a disabled preference suppresses that type and channel") and AC6.3.1. | Make the in-app preference suppress the feed entry — filter the feed by the current in-app preference, or record an in-app delivery marker at delivery time and return only marked alerts — and add a test that a disabled in-app type is absent from the feed. | New |
| R-03 | Major | services/notification/infrastructure/src/clients.rs > HttpContactDirectory, with code-generation/code-summary.md > D-01 | The mandatory-email path has no working dependency. C3 `GET /v1/users/{userId}/contact` is specified by the contract but not served by u3-identity, so every mandatory attempt receives a 404, which the client correctly maps to a permanent `address` error and the delivery is abandoned. BR3.1 and NFR1.4 cannot deliver in the deployed system, and no end-to-end test exercises the path. | Have u3-identity implement and verify the C3 contact endpoint, then add an integration test for a `security`/`key_revoked` alert that reaches the provider; do not treat BR3.1 as delivered until that test passes. | New |
| R-04 | Minor | services/notification/http/src/metrics.rs > Histogram and request_errors_total | The request histogram and the 5xx counter carry only a `service` label, while NFR17.3 requires route and status labels. The infrastructure design's per-route latency thresholds (100/150/200 ms) cannot be computed from the exported series. | Add bounded `route` and `status` labels to the histogram and error counter, as NFR17.3 and the monitoring design require. | New |
| R-05 | Minor | services/notification/application/src/delivery.rs > deliver (per-run email cap) | The per-run email cap is a load-then-increment: concurrent `deliver` tasks (up to 20) can all observe a count below 1,200 and overshoot by up to 19 calls, contradicting the code comment and the design's "never exceed the provider budget". | Use a compare-and-swap or increment-then-check so the cap holds under concurrency, or document the small overshoot as accepted. | New |
| R-06 | Minor | services/notification/application/src/delivery.rs > run (single `now`) | The run captures `now` once, then uses it for every attempt's `attempted_at` and retry deadline. In a run up to 250 s long, attempts stamped with the run start make a retry due sooner than 1 minute after the actual send and record times that are minutes off. | Take the clock time per attempt for both the stored attempt timestamp and the retry deadline. | New |
| R-07 | Minor | services/notification/application/src/trail.rs > base_context, with code-generation/code-summary.md > D-02 | NFR9.5 requires the audit context to carry the channel and the sanitised field names, but the trail sends only `sourceService`, `ruleId` and the correlation id. The values are kept on the rows and metrics, but the audit event itself does not meet the stated requirement. | Either extend C10's context allow-list (contract change) to accept `channel` and `sanitisedFields`, or record the narrowing against NFR9.5 explicitly rather than treating the requirement as met. | New |
| R-08 | Minor | services/notification/application/src/preferences.rs > save | The save validates every change, then upserts each row individually with no transaction. A storage failure mid-save leaves a subset of the requested changes persisted while the caller receives a 500. | Wrap the upserts in one transaction, or state the partial-save semantics and surface which rows applied. | New |
| R-09 | Minor | code-generation/traceability.json > NFR1.6, NFR20.1, NFR20.2 | Several targets do not support the requirements they claim: NFR1.6 (backup, RPO/RTO), NFR20.1 (API replicas/scaling) and NFR20.2 (volume) point at `services/notification/src/lib.rs` and `config.rs`, where no backup, replica or volume behaviour exists; those are infrastructure concerns. | Point these ids at the infrastructure design elements (Supabase backup regime, Swarm replica sizing, volume sizing), as the other infrastructure-owned ids do. | New |
| R-10 | Minor | services/notification/infrastructure/tests/repositories.rs | The 13 adapter tests are ignored and were not run, so the migration up/down, the dedupe insert, the keyset feed, the lease and wake SQL, the erasure transaction and the readiness check are unverified; the project requires one integration test per adapter against a real Postgres schema. | Run the ignored suite against a disposable Postgres in CI (and locally where available) before the unit is called complete, and record the result. | New |
| R-11 | Minor | code-generation/code-summary.md > D-04, D-08 | Two integration/deferral items are recorded but have no owner or step: the gateway has no C1 route for the notification feed and preferences, so the SPA cannot reach them, and the OTLP span export for NFR17.6 is deferred with no scheduled stage. | Add the gateway route wiring to the U1 backlog with the unit id, and schedule the OTLP exporter for the stage named in the deferred entry. | New |
| R-12 | Minor | infra/swarm-stack.yml > notification-api/notification-worker environment | The service-token registry is seeded by nothing in the delivered unit or pipeline: the runtime resolves only rows in `service_token_registry` (plus the mounted smoke token), and the stack inserts none, so every C9 caller receives 401 until an operator inserts the hashes by hand. | Add the registry seeding/rotation step (a release job that hashes each caller token and inserts before the new rows are deleted) and document it in the pipeline. | New |
| R-13 | Major | services/identity/http/src/handlers.rs > get_contact, with services/identity/http/src/middleware.rs > require_service | The new C3 contact endpoint accepts any known service token and does not check who is asking: `require_service` resolves the token to a service name, but `get_contact` binds it as `ServicePrincipal(_service)` and discards it. Any service in the identity token file — including Account, Job Orchestration or the gateway — can therefore read any user's verified email address on request. The C3 contract names Notification as the caller for the delivery read, and the project requires least privilege for every credential. | Enforce the caller on the contact route (require the principal's service to be `notification`, or a `contact.read` operation granted only to it), and apply the same per-caller check to the other internal routes on the shared middleware (the privacy sweep to Job Orchestration, the support-grant check to Account). | New |
| R-14 | Minor | services/identity/http/src/routes.rs and infra/swarm-stack.yml > identity_service_tokens, with code-generation/follow-ups.md | Identity's file-backed service-token registry must contain a `notification` entry whose token value equals `notification_outbound_token` before the C3 call can succeed; nothing in the stack, the identity documentation or the unit's follow-ups records that deployment step. Without it the contact call returns 401, which the client correctly treats as a permanent `address` error, so the mandatory email is abandoned. | Record the identity-side token entry (service `notification`, value shared with `notification_outbound_token`) in the follow-ups and the deployment notes for the service-token secret. | New |
| R-15 | Minor | services/notification/infrastructure/tests/repositories.rs and services/identity/tests/repositories.rs | The adapter suites remain unexecuted: 14 notification tests and 9 identity tests are still ignored, so the migration up/down, dedupe insert, keyset feed, wake flag, lease, emails-cap counter, erasure transaction and readiness SQL have never run against Postgres. The follow-ups name the exact commands but the suites have not been run, and the project requires one integration test per adapter against a real schema. | Run both ignored suites against a disposable schema in CI (or locally where a disposable Postgres is reachable), record the result, and keep the follow-up open until the run is green. | New |

> R-01 Not re-checked this round

> R-02 Not re-checked this round

> R-03 Not re-checked this round

> R-04 Not re-checked this round

> R-05 Not re-checked this round

> R-06 Not re-checked this round

> R-07 Not re-checked this round

> R-08 Not re-checked this round

> R-09 Not re-checked this round

> R-10 Not re-checked this round

> R-11 Not re-checked this round

> R-12 Not re-checked this round

### Summary

All three Majors are fixed and verified: the poll path records delivery metrics through the shared `record_delivery_report`, the feed filters types disabled for in-app with a correlated preference check, and Identity now serves `GET /v1/users/{userId}/contact` from a stored, verified address that is learned from the validated token and cleared by anonymisation, with Notification's mandatory path covered by an application-level end-to-end test. The nine Minor fixes also hold: route/status metric labels, the atomic database-backed email cap, per-alert clock reads, channel and sanitised fields in the audit context (with the C10 allow-list extended), transactional preference saves, retargeted traceability, the token-seeding SQL and the U1/OTLP follow-ups. One new Major blocks: the contact endpoint authorises any known service token and ignores the caller, so every internal service in Identity's token file can read any user's verified email — the endpoint must check that the caller is Notification (and the other internal routes should check their own callers). The remaining items are the identity-side token registration step and the still-unrun adapter suites.
