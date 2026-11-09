# Follow-ups — u9-notification

Items the unit hands to another owner or stage. Each names the unit id and the concrete step.

## U1 (u1-walking-skeleton) — ApiGateway C1 notification routes

The gateway has no route for the notification feed or preferences, so the SPA cannot reach them.
Wire these paths through the deny-by-default gateway request path, forwarding the Supabase JWT
unchanged (C1, NFR9.2), and set `X-Auth-Subject` to the token subject:

| Route | Upstream |
|---|---|
| `GET /v1/users/{userId}/notifications` | `http://notification-api:8099` |
| `GET /v1/users/{userId}/notifications/preferences` | `http://notification-api:8099` |
| `PUT /v1/users/{userId}/notifications/preferences` | `http://notification-api:8099` |

The C9 (`POST /v1/alerts`, `POST /v1/internal/privacy/anonymise`) and C11
(`POST /v1/internal/run/notification-*`) paths are internal and are not exposed through the
gateway. **Status:** open, owned by U1.

## U1 (u1-walking-skeleton) — Operation-phase observability: OTLP export (NFR17.6)

Owner: U1, in the Operation phase observability workstream, which installs the shared OpenTelemetry
exporter for every service. Notification currently emits structured `tracing` events with
allow-listed fields (service, alertId, type, channel, status, counts); when the exporter lands it
attaches submit → dedupe → deliver → audit spans with those attributes only. **Status:** scheduled,
owned by U1.

## Release pipeline — token seeding and adapter gates

- **Identity service tokens (C3 contact):** the `identity_service_tokens` Docker secret must be a
  JSON map that includes `"notification"` with the **same token** as
  `notification_outbound_token`. Identity authorises `GET /v1/users/{userId}/contact` by caller:
  only the `notification` entry may invoke it, and every other valid service token is refused with
  403. Recorded in `infra/swarm-stack.yml` (secret header and both Identity services) and
  `.env.example`. Rotate both secrets in the same release: seed the new `notification` entry in
  Identity first (both tokens can coexist in the JSON map only as one per service, so rotate
  Identity and Notification together), then update `notification_outbound_token`.
- **Per-caller authorization on Identity's internal routes:** all three are caller-scoped now —
  `GET /v1/users/{userId}/contact` → `notification`; `POST /v1/internal/run/audit-privacy-sweep` →
  `job-orchestration`; `GET /v1/internal/support-grants/{grantId}/check` → `gateway`. The JWT
  routes remain user-scoped by `require_jwt`. No internal route is intentionally shared.
- **Service-token registry (R-12):** after `notification-migrate`, run
  `infra/sql/notification-service-tokens.sql` with the SHA-256 of the gateway, identity, account,
  portfolio, reporting, and job-orchestration caller tokens. Rotation is insert-then-delete: run
  the file with the new digest, roll the consumers, then delete the superseded digest.
- **Adapter suites (R-10):** no disposable Postgres is reachable from the unit's worktree, so the
  ignored suites were compiled but not run. CI must run, against a disposable schema:
  `cargo test -p notification-infrastructure -- --ignored` (14 tests) and
  `cargo test -p identity --test repositories -- --ignored` (9 tests).
