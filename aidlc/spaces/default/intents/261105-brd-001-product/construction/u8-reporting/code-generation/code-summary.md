# Code Summary — u8-reporting

## What was built

The Reporting service as a five-package Cargo workspace under `services/reporting/`,
mirroring the portfolio and ledger services: four Clean Architecture crates plus a thin
composition package that owns the three binaries.

| Package | Responsibility |
|---|---|
| `reporting-domain` | `Brief` and `BriefDelivery`, the daily/weekly period and its data status, the composition rules that turn C8 and C7 reads into sections with per-line provenance, the internal service allow-list, and the C10 event shape |
| `reporting-application` | owner-scoped brief reads, the local period window, the leased C11 generation run (C3 due users → C8 valuation → C7 performance → C9 email → C10 outcome), and privacy export/anonymise |
| `reporting-infrastructure` | sqlx repositories, the C3/C7/C8/C9/C10 clients, JWKS validation, the service-token registry, the Postgres readiness probe, migrations, the clock, and identifier generation |
| `reporting-http` | C1 brief routes, C1/internal privacy routes, the worker's C11 listener, DTO mapping, RFC 7807 problems, correlation id, security headers, and Prometheus text metrics |
| `reporting` | Composition root and three binaries: `reporting-api` (8103), `reporting-worker` (8104), `reporting-migrate` |

### HTTP surface

| Method and path | Identity | Behaviour |
|---|---|---|
| `GET /v1/users/{userId}/briefs?period=&cursor=&limit=` | JWT | the caller's briefs, newest first, optionally narrowed to `daily` or `weekly`; another user's list is 403 (BR3.1) |
| `GET /v1/users/{userId}/briefs/{briefId}` | JWT | one brief with its sections and delivery attempts; another user's brief is 404 |
| `GET /v1/internal/privacy/export` | service token (`privacy_export`) | Identity's export of the user's briefs with sections |
| `POST /v1/internal/privacy/anonymise` | service token (`privacy_anonymise`) | replaces the brief subject with a random opaque reference once per idempotency key |
| `GET /v1/internal/smoke` | service token (`run_brief_generation`) | release smoke reader |
| `POST /v1/internal/run/brief-generation` | service token (`run_brief_generation`) | worker C11 run; 409 when another run holds the lease |
| `GET /healthz`, `/readyz`, `/metrics` | public | readiness checks only Postgres (source outages must not fail it) |

The operation check reads the caller's stored `operations` from the service-token registry
(`ServiceCaller::from_registered`); the static table in the domain is only a validation of the
seeded labels, so narrowing or rotating a caller's allow-list takes effect.

### Database

`services/reporting/migrations/0001_init.up.sql` creates `briefs` (unique
`(user_id, period, period_start)`, the status/sections shape check, and the `(user_id, updated_at)`
index), `brief_deliveries` (unique per brief and channel), `brief_runs` (one live lease per job),
`service_token_registry`, and `privacy_anonymisations`. Roles: `reporting_svc` (DML with a
column-scoped UPDATE on `briefs` and no DELETE), `reporting_readonly`, and `reporting_migrator`
(DDL plus the two-year brief prune, which the runtime role cannot perform). The down migration
refuses to drop a schema that holds briefs.

### Generation (BR1.1–BR1.3, BR2.1–BR2.3)

1. The run claims a 1,800 s single-flight lease, renewed every 60 s, and sweeps expired leases.
2. C3 says which users are due for `daily` and `weekly`; Reporting derives the local window at
   07:00 in the user's IANA timezone (DST-correct via `chrono-tz`), falling back to UTC for an
   unknown zone.
3. One brief per window: a generated window is never regenerated; a failed, or an over-an-hour
   pending, brief is retried.
4. C8 valuations and C7 performance are folded into sections. Each allocation row carries its
   as-of, price source, freshness, observed time, and FX provenance; the performance section
   carries realised, net invested, unrealised, coverage, FX provenance, and its as-of. A figure
   whose source could not be priced stays `null` (rendered as an em dash), never zero; the overall
   status is the weakest of the two reads.
5. An unproducible brief is stored as `failed` with an explicit reason and audited to C10.
6. Every generated brief submits a C9 alert of the contracted type `brief` with the idempotency key
   `brief:{briefId}:email` and `channel: email`, so it adds no second in-app entry. The delivery
   attempt is stored as `submitted` (Notification accepted it), `suppressed`, or `failed`; the
   submission outcome is audited as `reporting.brief.email_submitted`, falling back to the
   allow-listed `system.reporting.brief` when C10 refuses the finer action. Suppression at
   delivery time happens inside C9 (a preference), not at submission; the stored row therefore
   means "accepted".

### Cross-unit changes made for this unit

- Portfolio gained the C8 reporting reads: `GET /v1/users/{userId}/portfolios` and
  `GET /v1/portfolios/{portfolioId}/valuation`, with the `portfolios_read` and `valuation_read`
  operations added to its registry seed.
- Identity gained the C3 due-user read: `GET /v1/internal/users/due?period=&at=` returns the
  active users and their timezones to Reporting's token only (403 for any other caller), paged at
  1,000 users per read.
- Ledger's `GET /v1/users/{userId}/pnl` now accepts Reporting's service token holding `pnl.read`
  (JWT callers keep the owner-only rule), seeded by `infra/sql/ledger-service-tokens.sql`.
- Notification gained the C9 `channel` field: a migration adds `requested_channel` to `alerts`, a
  requested channel narrows the stored preference, a mandatory email still wins, and an
  `email`-only alert is excluded from the in-app feed. The `brief` payload allow-list gained
  `period`, `asOf`, `dataStatus`, and `hasUnpriced`.
- Audit's system-action allow-list gained the reporting events and the `system.reporting.brief`
  fallback, so Reporting's system-attributed events are accepted.
- `infra/swarm-stack.yml` gained `reporting-api`, `reporting-worker`, `reporting-migrate`, their
  four secrets, their health checks and update configs, and the token-map entries for Identity and
  Audit.

## Tests

`cargo test -p reporting-domain -p reporting-application -p reporting-infrastructure -p reporting-http -p reporting`
runs 133 tests (46 + 45 + 17 + 14 + 10 + 1) across the four crates and the service package. The
workspace suite is 1,293 tests, all green; `cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets -- -D warnings` are clean on this tree. Database adapter
tests that need `TEST_DATABASE_URL` remain `#[ignore]`d, as in the other units.
