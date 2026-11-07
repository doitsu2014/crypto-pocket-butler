# Code Summary — u4-account

## What was built

The Account service under `services/account/`, a four-crate Clean Architecture
workspace (`account-domain`, `account-application`, `account-infrastructure`,
`account-http`) plus the `account` composition root with three binaries:

- `account-api` (port 8091) — C1 onboarding and re-sync routes, C5 holdings and
  transactions reads, privacy export/anonymise, the C11 run endpoint, `/healthz`,
  `/readyz`, `/metrics`.
- `account-worker` (port 8092) — the supervisor loop (sync claim/lease/renew,
  fenced snapshot write, CSV processing, periodic key re-verification), probe
  endpoints, and the internal wake endpoint.
- `account-migrate` — the release job entrypoint that applies the versioned sqlx
  up migrations; a rollback applies the matching down migration.

## Routes

| Route | Identity | Handler |
|---|---|---|
| `POST /v1/accounts/exchange` | Supabase JWT, AAL2 | `add_exchange` (verify-before-create) |
| `POST /v1/accounts/wallet` | Supabase JWT | `add_wallet` (C4 chain check, address format) |
| `POST /v1/accounts/walletconnect` | Supabase JWT | `add_walletconnect` (opaque session ref) |
| `POST /v1/accounts/{accountId}/resync` | Supabase JWT | `resync` (202 with the run id) |
| `GET /v1/users/{userId}/accounts` | Supabase JWT | `list_accounts` (owner only) |
| `POST /v1/accounts/{accountId}/imports` | Supabase JWT | `upload_import` (10 MiB cap, staged) |
| `GET /v1/imports/{batchId}` | Supabase JWT | `get_import` (owner only) |
| `GET /v1/users/{userId}/holdings` | service token (portfolio/ledger) | `get_holdings` |
| `GET /v1/accounts/{accountId}/transactions` | service token (portfolio/ledger) | `get_transactions` |
| `GET /v1/internal/privacy/export` | service token (identity) | `privacy_export` |
| `POST /v1/internal/privacy/anonymise` | service token (identity) | `privacy_anonymise` |
| `POST /v1/internal/run/{jobName}` | service token (job-orchestration) | `run_account_sync` (`account-sync`) |

Every refusal is RFC 7807 `application/problem+json` with a correlation id; a
validation refusal names field paths only.

## Layer map

- **domain** — `Account`, `Credential`, `WalletConnection`, `SyncRun`,
  `HoldingSnapshot`, `ImportedTransaction`, `ImportBatch`, `ImportRowError`;
  venue/label/address/decimal validation; dedupe-key derivation; secret-shaped
  input detection; the C10 record shape; the service-token allow-list.
- **application** — use cases and inward ports: `AccountService` (WF1–WF3,
  resync), `SyncService` (WF4, lease, fenced write, failure path), `ImportService`
  (WF5), `HoldingService` (WF6), `VerificationService` (WF7), `PrivacyService`
  (WF8), `Trail`.
- **infrastructure** — sqlx repositories for accounts, credentials, wallets, sync
  runs, holdings, imports, privacy, and the service-token registry; Supabase
  Vault (SQL), Supabase Auth admin (C15), Audit (C10), Notification (C9), Asset
  Registry (C4), Binance/OKX read-only clients, EVM/Solana JSON-RPC clients,
  JWKS validation, readiness, wake transport, bounded retry helpers.
- **http** — Axum router, DTOs, problem documents, correlation/JWT/service-token
  middleware, Prometheus metrics, API and worker servers.

## Persistence

`services/account/migrations/0001_init.up.sql` creates the `account` schema:
`accounts`, `credentials`, `wallet_connections`, `sync_runs`,
`holding_snapshots`, `import_batches`, `import_upload`, `imported_transactions`,
`import_row_errors`, `service_token_registry`, `privacy_anonymisations`, with the
per-user case-insensitive label index, the one-live-run index, the
`(account_id, dedupe_key)` unique index, the 10 MiB upload check, and the
`account_svc` / `account_readonly` / `account_migrator` roles. The down migration
refuses to drop a schema that holds account records and otherwise reverses the up
migration.

## Test record (TDD)

The Testing Contract's order was followed for every layer: the failing run was
recorded before the implementation. The exact unit-scoped command:

```
cargo test -p account-domain -p account-application -p account-infrastructure -p account-http
```

| Suite | Tests | Result |
|---|---|---|
| `account-domain` (`tests/domain.rs`) | 63 | all pass |
| `account-application` (`tests/application.rs`) | 69 | all pass |
| `account-infrastructure` (`tests/providers.rs`) | 5 | all pass |
| `account-infrastructure` (`tests/repositories.rs`) | 13 | **ignored** (need `TEST_DATABASE_URL`) |
| `account-http` (`tests/http.rs`) | 24 | all pass |
| **Total executed** | **161** | **161 passed, 0 failed** |

`cargo fmt --all -- --check` passes. `cargo clippy -p account-domain -p
account-application -p account-infrastructure -p account-http -p account
--all-targets -- -D warnings` passes. `cargo check --workspace --all-targets`
passes.

### Not executed

The 13 adapter integration tests in
`services/account/infrastructure/tests/repositories.rs` were not executed: this
environment has no `TEST_DATABASE_URL` and no disposable Postgres. They are marked
`#[ignore = "requires TEST_DATABASE_URL (disposable Postgres)"]` and cover the
migration up/down/re-up, the label and one-live-run indexes, the fenced snapshot
write, import staging/dedupe, the service-token registry, and the privacy ledger.
Run them with:

```
TEST_DATABASE_URL=postgres://... cargo test -p account-infrastructure --test repositories -- --ignored
```

No live Binance, OKX, EVM, Solana, Supabase, Audit, Notification, or Asset
Registry call was made; those adapters are covered by the port fakes in the
application and HTTP suites and by the HMAC/retry unit tests.

## Deviations

- The C1 exchange-key DTO carries an optional `passphrase` field for OKX. The
  original functional entity listed only a key and a secret; OKX's signed API
  requires a passphrase, and it is stored in the same Vault secret.
- `POST /v1/accounts/{accountId}/imports` requires an existing `csv` account; the
  functional spec creates csv accounts through the import surface, and this unit
  exposes no separate create-csv route.
- The C11 endpoint accepts an optional `{ "accountIds": [...] }` body. Empty or
  absent runs every pending run; named accounts are claimed and processed
  immediately, and a live lease returns 409.
- The Audit C10 event context is identifier-only (`accountId`, `runId`,
  `category`, counts); no key, token, or full address is written.
