# Code Summary — u7-ledger

## What was built

The Ledger service as a five-package Cargo workspace under `services/ledger/`, matching the
walking skeleton's four-crate Clean Architecture shape and the approved plan step for step.

| Package | Responsibility |
|---|---|
| `ledger-domain` | Append-only `Transaction` with canonical and external keys (BR1.2, BR1.3) and a price currency, per-field `Correction` validation and effective-value chains (BR4.1, BR2.3), the FIFO lot engine with fee attachment, matched-transfer basis, and external-transfer consumption (BR2.1, BR2.4, BR2.5), transfer matching by asset/opposite-kind/1%/24 h (BR6.1), the mutable `LedgerSubject`, P&L with C6 marks, C5 holdings, base-currency FX, and coverage (BR2.2, BR3.1), and the stored `PnlSnapshot` |
| `ledger-application` | C5 ingest with canonical dedupe and line-error refusal (BR1.1–BR1.4), recorded trades with the user-price label and the holdings guard (BR5.1, BR5.2, NFR1.7), corrections with commit-first audit retry (BR4.1, NFR1.4), P&L/coverage reads against C5 holdings and C6 prices/FX, C7 privacy export/anonymise (NFR9.2, NFR13.1), the leased reconcile (NFR1.5, NFR20.4), the rebuilder, and the C10 trail |
| `ledger-infrastructure` | sqlx repositories for transactions, corrections, subjects, lots, snapshots, transfer pairs, anonymisation requests, and the reconcile lease; JWKS validation; the Postgres service-token registry; C4/C5/C6/C10 HTTP clients with bounded retry; the one-shot anonymiser gateway; the readiness probe; migrations; Postgres TLS enforcement |
| `ledger-http` | Two Axum routers, DTOs, RFC 7807 problems, correlation id, security headers, `X-Auth-Subject` agreement, JWT user paths, service-token operation allow-lists, Prometheus metrics |
| `ledger` | Composition root and three binaries: `ledger-api`, `ledger-worker`, `ledger-migrate` |

### HTTP surface

| Method and path | Identity | Behaviour |
|---|---|---|
| `GET /v1/users/{userId}/transactions` | JWT, `X-Auth-Subject == sub`, owner only | keyset page (newest first, cursor `executedAt\|id`, limit ≤ 200); each row carries its `currency`; a missing price is returned as `price: null` with `priceMissing: true`, never zero |
| `GET /v1/users/{userId}/pnl` | JWT, owner only | total and per-asset P&L with `currency`, `fxAsOf`, `fxEstimated`, `coverage`, `coverageLabel` (complete/partial/unavailable), and the named `coverageGap`; holdings come from C5, marks and FX from C6, and an unpriced holding is named and never valued at zero. A C5/C6 outage returns a degraded 200 with `marksUnavailable`, `holdingsUnavailable`, or `fxUnavailable` set; only Postgres failure is a 503 |
| `POST /v1/transactions/{transactionId}/corrections` | JWT, owner only | 201 with the stored old/new value; 422 for an invalid field value; 404 unknown; 403 another user's row; the C10 audit is written after commit with 1 s/2 s/5 s retry |
| `POST /v1/users/{userId}/trades` | service token + `trades.write` (Portfolio) | 201 with `source = recorded_trade` and `priceSource = user`; a missing price or a sell beyond holdings is 422 |
| `GET /v1/internal/privacy/export` | service token + `privacy.export` (Identity) | transactions and corrections for the user |
| `POST /v1/internal/privacy/anonymise` | Identity service token + `privacy.anonymise` | clears the subject's user id, sets a non-reversible reference; idempotent by key; audited |
| `GET /v1/internal/smoke` | service token + `smoke` | readiness |
| `POST /v1/internal/run/ledger-reconcile` | service token + `runs` (worker) | leased, batched (≤ 5,000 transactions) rebuild of every subject |
| `POST /v1/internal/run/ledger-ingest` | service token + `runs` (worker) | WF1 ingest of one user's C5 rows |
| `GET /healthz`, `GET /readyz`, `GET /metrics` | public | readiness checks Postgres and `_sqlx_migrations` only, so a C5 outage never removes tasks |

Deny by default: only the routes above exist; anything else is a 404 problem document.

### Database

`services/ledger/migrations/0001_init.up.sql` creates:

- `ledger_subjects` with the mutable `user_id` and `anonymised_ref` (identity check);
- `transactions` with `canonical_key UNIQUE` and `external_key UNIQUE`, kind/source checks,
  a `currency` column, positive-quantity and non-negative-price checks, and the
  `(subject_id, executed_at DESC, id DESC)` list index and `(subject_id, asset_id, executed_at)`
  scan index;
- `corrections` with the field enum check and the transaction index;
- `transfer_pairs` with nullable out/in legs (one must be present) and the matched-state
  consistency check, so a lone transfer_in can hold a pending pair;
- `cost_basis_lots` with `remaining BETWEEN 0 AND quantity`, the acquisition `currency`, and the
  FIFO index;
- `pnl_snapshots` with `currency`, `fx_as_of`, `fx_estimated`, the coverage range, the
  coverage/figure consistency check, and the user and portfolio unique scope indexes;
- `anonymisation_requests` keyed by idempotency key (no user id);
- `reconcile_leases` seeded with `ledger-reconcile`;
- `service_token_registry` storing only SHA-256 hashes and operation allow-lists;
- roles `ledger_svc`, `ledger_anonymiser`, `ledger_readonly`, `ledger_migrator` with exactly the
  grants each needs: no UPDATE or DELETE on transactions/corrections, no UPDATE on
  `ledger_subjects` for `ledger_svc`, and UPDATE on `ledger_subjects` only for
  `ledger_anonymiser`.

The down migration refuses to drop a ledger that holds financial rows; the roles survive a
rollback. Role creation and grants are skipped with a notice when a disposable database's role
cannot manage roles.

### Deployment assets

- `services/ledger/Dockerfile.api` (8097), `Dockerfile.worker` (8098), `Dockerfile.migrate`:
  multi-stage, `USER 10001`.
- `infra/swarm-stack.yml`: `ledger-api` (2 replicas, 0.25 vCPU / 256 MB, start-first),
  `ledger-worker` (1 replica, 0.5 vCPU / 512 MB, stop-first so reconcile never overlaps), and
  `ledger-migrate` (one-shot). All join only the encrypted `internal` overlay; secrets are
  `ledger_database_url`, `ledger_migrate_database_url`, `ledger_anonymiser_database_url`,
  `ledger_outbound_token`, and `ledger_smoke_token`.
- `.env.example` gained the non-secret ledger variables and secret paths, including
  `MARKET_DATA_URL` (C6) and `LEDGER_BASE_CURRENCY`.

## How to run it

```
cargo test -p ledger-domain -p ledger-application -p ledger-infrastructure -p ledger-http
cargo fmt --all -- --check
cargo clippy -p ledger -p ledger-domain -p ledger-application \
  -p ledger-infrastructure -p ledger-http --all-targets -- -D warnings
```

Adapter suite (needs a disposable Postgres whose user may create schemas):

```
TEST_DATABASE_URL=postgres://test:test@localhost:55432/cpb_test \
  cargo test -p ledger-infrastructure --features integration -- --ignored
```

## Verification output (run in this workspace)

```
$ cargo test -p ledger-domain -p ledger-application -p ledger-infrastructure -p ledger-http
ledger-domain:         54 passed; 0 failed
ledger-application:    56 passed; 0 failed
ledger-infrastructure:  0 passed; 0 failed (12 adapter tests are #[ignore]d behind the feature)
ledger-http:           27 passed; 0 failed

$ cargo fmt --all -- --check
(no output, exit 0)

$ cargo clippy -p ledger -p ledger-domain -p ledger-application \
    -p ledger-infrastructure -p ledger-http --all-targets -- -D warnings
(no warnings, exit 0)
```

137 tests run and pass. The 12 adapter tests compile under `--features integration` and are
`#[ignore]`d because no disposable Postgres is available here.

## Gaps and deviations accepted

1. **P&L marks holdings with C6.** `GET /v1/prices` supplies the current unit price and freshness;
   a holding without a price is named in `coverageGap` and returned as `priceFreshness: unpriced`
   with `markPrice: null`, never zero. `GET /v1/fx` converts every non-base amount at compute
   time, and `currency`, `fxAsOf`, and `fxEstimated` are returned. The read-path C6 call is one
   attempt bounded to 150 ms (assumed C6 p95 ≤ 120 ms), so the 400 ms `getPnl` budget holds.
2. **A C5 or C6 outage is a degraded 200, never a 503.** Coverage is measured against Account's
   C5 `getHoldings`; on a C6 failure the affected holdings are labelled unpriced, `coverageLabel`
   is `partial` with the gap naming them, the figures are withheld, and `marksUnavailable` (or
   `holdingsUnavailable` / `fxUnavailable`) is set. Only a Postgres failure returns 503. These
   labelled degraded reads count as good in the P&L availability SLI; only a 5xx counts as bad.
3. **An unmatched transfer_out consumes the outgoing lots with no realised gain.** Only a matched
   transfer is a no-op; a transfer out to an untracked wallet removes basis without a disposal.
4. **The snapshot short-circuits only when it is fresh.** A read whose stored snapshot
   `computedAt` is later than the last transaction or correction returns the stored totals and
   rebuilds the per-asset view from the stored lots; per-asset realised figures are not persisted,
   so a short-circuited read leaves them null rather than inventing them.
5. **A correction or trade still rebuilds the whole subject,** not only the affected asset, because
   the total snapshot needs every asset. This is recorded as a deliberate deviation from the
   performance design's affected-asset optimisation.
6. **A lone transfer_in now gets a pending pair row** (`inTransactionId` set, `outTransactionId`
   null), so a later outbound leg matches it in either ingest order.
7. **Account discovery uses C5 `getHoldings`.** C5 has no user-accounts read, so an account with no
   holdings and no transactions is never visited; the C5 amendment records this note. The
   transaction payload reads an optional `source` (default `venue`) and `externalId`.
8. **Two run endpoints are extensions:** the C7 transaction list (the base C7 contract lists only
   `/pnl`) and the C11 `ledger-ingest` run. Both use the existing operation allow-list (`runs` for
   ingest).
9. **OTLP span export and the two-year retention sweep are deferred to the operation stage.**
   Structured tracing events are emitted now; the retention window (two years) is fixed in the
   design and anonymise exists, but no scheduled sweep is implemented in this unit.
10. **Anonymisation idempotency is keyed by the caller's key**, not the user id, so the user id can
    be cleared. A retry with the same key returns the same reference; a retry with a new key after
    the user id is gone reports already-applied without the reference.
11. **The anonymiser role is opened per request.** `PgAnonymiserGateway` creates a one-connection
    pool from `ledger_anonymiser_database_url`, runs the single UPDATE, and closes it, so the
    runtime pool never holds that role.
12. **Postgres TLS is enforced in code** (`PgSslMode::Require` in `db.rs`) as well as by the
    encrypted internal overlay in the swarm stack.
13. **80% line coverage per crate was not measured.** `cargo llvm-cov` is not installed here. Every
    test file covers a happy path and at least two error/edge cases; the coverage gate remains
    unverified.
14. **The 12 adapter tests were not executed** because no disposable Postgres was available. They
    cover subjects, transactions and both keys, duplicate conflict, pagination, corrections, lots,
    snapshots, transfer pairs, anonymisation records, the lease, token resolution, and readiness.
15. **Audit context carries a reason hash**, never the free-text reason or an amount; the reason
    stays on the correction row.
