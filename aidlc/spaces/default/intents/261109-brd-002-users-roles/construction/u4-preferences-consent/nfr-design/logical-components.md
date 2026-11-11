# Logical Components — u4-preferences-consent

**Upstream:** `security-design.md`, `reliability-design.md`, `performance-design.md`, `functional-spec.md`, `components.md` (PreferencesConsent).

## Component inventory

| Component | Layer | Responsibility | Depends on |
|---|---|---|---|
| `Preferences` (use case) | application | Read with per-field source; partial write with closed-set validation | `PreferencesRepository` (port), `UserLookup` (port), `ObservationSink` (port) |
| `ConsentState` (use case) | application | Resolve the newest event per kind and compute `missing` | `ConsentRepository` (port), `ObservationSink` (port) |
| `ConsentGrant` (use case) | application | Validate kind and policy version; insert a `granted` event | `ConsentRepository` (port), `ObservationSink` (port) |
| `ConsentWithdraw` (use case) | application | Row-locked read, state check, insert `withdrawn` with the inherited version, insert the outbox row — one transaction | `ConsentRepository` (port), `AuditOutbox` (port), `ObservationSink` (port) |
| `ConsentHistory` (use case) | application | Paged history read | `ConsentRepository` (port) |
| `PreferencesRepository` (port) | application trait | Load/upsert one user's preferences | implemented by `PgPreferencesRepository` |
| `ConsentRepository` (port) | application trait | Newest-per-kind, history, insert, and the transactional withdraw | implemented by `PgConsentRepository` |
| `AuditOutbox` (port) | application trait | Enqueue an audit intent inside the caller's transaction | implemented by `PgAuditOutbox` |
| `UserLookup` (port) | application trait | Confirm the user exists (u1) | implemented in-process by the registry repository |
| `ObservationSink` (port) | application trait | Counters, histogram, gauges | implemented by `ConsentMetrics` at the `http` boundary |
| `PgPreferencesRepository` | infrastructure | sqlx upsert and read | Identity Postgres (`identity_app`) |
| `PgConsentRepository` | infrastructure | `DISTINCT ON` state read, paged history, transactional insert with `FOR UPDATE` | Identity Postgres |
| `PgAuditOutbox` | infrastructure | Insert into `consent_audit_outbox` inside the caller's transaction | Identity Postgres |
| `AuditOutboxDrainer` | infrastructure worker | `FOR UPDATE SKIP LOCKED` batches, C9 post with idempotency key, mark delivered | Postgres, C9 |
| `ConsentMetrics` | infrastructure (http crate) | Emits the `consent_*` metrics | metrics registry |
| `PreferencesRoute`, `ConsentRoute` | http | C6 endpoints and the `/v1/me/...` surface; DTO mapping | the application use cases |

## Import direction

Every arrow points inward. `application` sees only its ports; `infrastructure` implements them;
`http` maps DTOs and calls use cases. The drainer is infrastructure and is started by the
composition root, not by a use case.

## Boundaries and failure domains

- **One failure domain** inside the Identity process; no other service links the code.
- **The drainer is a separate unit of work** in the same process: a C9 outage grows the outbox and
  alerts; it never blocks a withdrawal.
- **The erasure path** runs as `identity_retention` and is the only writer that can delete.

## Blast radius

| Failure | Blast radius |
|---|---|
| Consent repository bug | Every gate call and consent write |
| Drainer stopped | Audit delivery lag only; writes continue |
| Postgres down | Gate refuses every connection (fail closed) |
| C9 down | Outbox grows; nothing user-visible |

## Shared resources

| Resource | Shared with | Isolation |
|---|---|---|
| Identity Postgres (`identity_app`) | The rest of the Identity service | Own schema; three new tables and one altered |
| `identity_retention` role | The erasure job (u5) | NOLOGIN, delete-only on the two tables |
| Audit service (C9) | Every service | Per-service token; outbox-mediated |
| C6 | U7 (person's surface), `account-api` (the gate) | JWT plus service-token allowlist |

## What this unit deliberately does not own

No new container, no cache tier, no queue. The outbox is a database table, not a broker — the
project forbids message queues, and a drained table is the same pattern without one.
