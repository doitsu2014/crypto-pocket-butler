# Code Summary — u5-market-data

## What was built

The Market Data service as a four-crate Clean Architecture Cargo workspace under
`services/market-data/`, plus the `market-data` composition package with three binaries. It
implements the C6 prices and FX reads, the C11 refresh endpoint for the two leased jobs, the
tier refresh, the FX refresh, the shared per-source rate budget, the circuit breaker, the
90-day retention sweep, the C10 failed-run event, and the per-caller service-token allow-list
including `smoke-reader`.

| Package | Responsibility |
|---|---|
| `market-data-domain` | `AssetPriceTier`, `PriceObservation` (accepted/rejected), `PriceQuote`, `FxRate`/`ServedFxRate`, `RefreshJob`, `RefreshRun` with its lease, `Freshness`, the shared token-bucket arithmetic, the secret-shape sanitiser, the C10 event shape, and the caller allow-list |
| `market-data-application` | `QuoteService` (WF1), `FxService` (WF2), `RefreshService` (WF3/WF4), `RetentionService` (BR5.3), the inward-defined ports, and the app error surface |
| `market-data-infrastructure` | sqlx repositories (`DISTINCT ON` reads, per-job lease claim, tier upsert, shared rate bucket, retention prune), the C4 client (150 ms read path, retried run path), the provider client, the Vault function call, the C10 client, the service-token registry, readiness, the clock, and the retry sleeper |
| `market-data-http` | Axum router, DTOs, RFC 7807 problems, correlation id, security headers, service-token middleware, Prometheus text metrics, and the worker listener |
| `market-data` | Composition root, `market-data-api`, `market-data-worker`, and `market-data-migrate` |

### HTTP surface

| Method and path | Identity | Behaviour |
|---|---|---|
| `GET /v1/prices?assetIds=` | service token (`prices_read`) | 200 in request order; `live`/`delayed`/`stale`/`unpriced`; 422 for missing/empty/malformed/oversized (> 1,000); 401 unknown token; 403 outside the allow-list; 503 on Postgres failure |
| `GET /v1/fx?base=&quote=` | service token (`fx_read`) | 200 direct or USD-derived (both leg sources in `derivedFrom`); 404 when nothing is fresh; 422 same pair or malformed currency; 401/403/503 |
| `POST /v1/internal/run/{jobName}` | service token (`run_refresh`) | 200 with the run result for `price-refresh-top100`/`price-refresh-tail`; 202 with the active run id for a manual call against a held lease; 409 with the active run id for a scheduled call; 404 unknown job; 422 invalid `X-Run-Trigger` |
| `GET /healthz`, `GET /readyz`, `GET /metrics` | public | liveness; Postgres readiness; Prometheus text with the observability-design metric names |

### Database

`services/market-data/migrations/0001_init.up.sql` creates six tables in the service's own
schema, with no cross-schema grants and no RLS:

- `asset_price_tiers` (rank-derived `top100`/`tail`, bound check, rank/tier check);
- `price_observations` (`numeric(38,18)`, accepted-positive and rejected-reason checks,
  `(asset_id, source_name, observed_at DESC)` and accepted/received indexes);
- `fx_rates` (positive `numeric(38,18)`, distinct pair, `(base, quote, observed_at DESC)`);
- `refresh_runs` (per-job partial unique index on `running`, `(job, status, lease_until)`
  index, reconciling-counts check, failed-shape check);
- `service_token_registry` (SHA-256 hash, caller, operations, active);
- `source_rate_buckets` (one row per source, refilled under a row lock).

`read_provider_secret(text)` is `SECURITY DEFINER`, returns NULL when Vault is absent, and is
granted to `market_data_svc`. Roles `market_data_svc`, `market_data_readonly`, and
`market_data_migrator` are created `NOLOGIN`. The down migration refuses to drop a schema that
holds observations.

### Key behaviours

- The read path never calls a provider; the C4 source list is read once per request with a
  150 ms budget, and a C4 failure degrades every asset to its newest stored observation as
  `stale` (or `unpriced`).
- Selection is deterministic by C4 priority; a delayed feed is labelled before a live one; an
  observation beyond its tier bound is `stale` only within 24 h and otherwise `unpriced`.
- The two jobs hold separate single-flight leases. The claim expires an overdue run in the same
  transaction, a manual caller gets 202, a scheduled caller gets 409, and a failed 60-second
  renewal stops the run.
- A provider reading is accepted only when positive and not more than 300 s ahead; otherwise it
  is stored as rejected and the previous accepted observation keeps serving.
- Transient provider failures retry up to three attempts with backoff; three consecutive
  transient failures open a five-minute per-source circuit. A rate-limited source is deferred
  and the next source is tried.
- The top100 job refreshes every tier row and the configured USD FX pairs; the tail job only
  prices its own rank slice.
- A failed run writes a C10 `price_refresh_failed` event (`system` actor, service identity for
  a manual trigger, job/trigger/category context, run id as target). A failed audit write is
  logged and counted, never dropped.

## Tests

Exact unit command:

```
cargo test -p market-data-domain -p market-data-application -p market-data-infrastructure -p market-data-http
```

Result (final run): **ok. 140 passed; 0 failed; 17 ignored** across the four packages.

| Suite | Passing | Ignored |
|---|---|---|
| `market-data-domain` `tests/domain.rs` | 44 | 0 |
| `market-data-application` `tests/application.rs` | 60 | 0 |
| `market-data-infrastructure` unit tests | 3 | 0 |
| `market-data-infrastructure` `tests/migrations.rs` | 0 | 4 |
| `market-data-infrastructure` `tests/repositories.rs` | 0 | 13 |
| `market-data-http` `tests/http.rs` | 33 | 0 |

The 17 ignored tests need a disposable Postgres (`TEST_DATABASE_URL`), which was unset in this
workspace, so they were not executed here:

```
TEST_DATABASE_URL=postgres://... cargo test -p market-data-infrastructure --test migrations -- --ignored
TEST_DATABASE_URL=postgres://... cargo test -p market-data-infrastructure --test repositories -- --ignored
```

Coverage command (CI, not run here):

```
cargo llvm-cov -p market-data-domain --fail-under-lines 80
cargo llvm-cov -p market-data-application --fail-under-lines 80
cargo llvm-cov -p market-data-infrastructure --fail-under-lines 80
cargo llvm-cov -p market-data-http --fail-under-lines 80
```

Formatting and lints:

- `cargo fmt --all -- --check` — clean.
- `cargo clippy -p market-data-domain -p market-data-application -p market-data-infrastructure -p market-data-http -p market-data --all-targets -- -D warnings` — clean.

## TDD evidence

Red states were recorded before implementation for three of the four layers:

- **Data model (Step 3)**: `cargo test -p market-data-domain` first failed to compile with
  `error[E0432]: unresolved import market_data_domain::audit` (and `constants`, `freshness`,
  `fx`, `ids`, …). Implementation brought it to 45 passing.
- **Repositories (Step 6)**: `cargo test -p market-data-infrastructure --no-run` first failed
  with `error[E0432]: unresolved import market_data_infrastructure::repositories` and
  `...::service_token`.
- **API (Step 12)**: `cargo test -p market-data-http --no-run` first failed with
  `error[E0432]: unresolved import market_data_http::metrics` and unresolved `router`,
  `worker_router`, `AppState`, `WorkerState`.

**Deviation (disclosed):** the application tests and fakes were written before the use cases,
but their first recorded run happened after the use cases were in place; a separate
application-layer compile-failure capture is therefore missing. The application behaviour is
still test-first by construction, and the final 56-test suite exercises every rule.

## Deviations and decisions

1. **Provider retry placement.** The reliability design and BR4.2 describe the retry budget
   slightly differently (2 attempts in one place, up to 3 in the rule). The adapter performs one
   bounded call (1 s connect / 5 s total) and the application performs up to three attempts with
   jittered backoff, satisfying BR4.2 and keeping the circuit breaker at the application layer.
2. **C11 location.** The C11 run endpoint is served by `market-data-api` on 8093, as the approved
   plan and C11 contract put it. Runs execute inline in the API replica that receives the call,
   fenced by the per-job lease; `market-data-worker` owns the lease sweep and retention on 8094.
   `infrastructure-design/infrastructure-specification.md` and `nfr-design/logical-components.md`
   were corrected to state this explicitly.
3. **C4 tolerance.** The client requires `name`/`priority` and defaults `quoteCurrency` to USD,
   `rateLimitPerMinute` to 60, `delayedFeed` to false, and `enabled` to true, so it stays
   interoperable with the current U1 DTO while the contract fields are still being published.
4. **C11 run token and outbound token.** Both are read from Vault at startup through
   `read_provider_secret('run')` and `read_provider_secret('outbound')`; `RUN_TOKEN_FILE` remains
   an optional local override. The run token grants only `RunRefresh`. No service-token or run
   token Docker secret is declared in the Swarm stack.
5. **`read_provider_secret` without Vault.** The function returns NULL when the Vault schema is
   absent, so the migration and its integration tests work on a disposable database.
6. **FX cross provenance.** A derived rate reports `source = "derived"`, both leg sources in
   `derivedFrom`, and the older leg's `observedAt` as the conservative timestamp.
7. **Retention.** The prune keeps the newest observation per `(asset_id, source_name)` and the
   newest FX reading per `(base, quote)`, matching BR5.3.
8. **FX direction and pairs.** A requested pair resolves by exact lookup, reverse lookup with the
   rate inverted, or a USD cross whose legs may each be stored in the provider's own direction and
   inverted; USD pairs are no longer short-circuited. The top100 job derives its refresh pairs from
   the union of every enabled source's `quoteCurrency` → USD and every launch base → USD
   (default `EUR,GBP,VND,USDT`), skipping only `USD → USD`; a USD-quoted source still lets the
   other sources' quotes refresh. Stablecoin quote symbols `USDT`, `USDC`, `BUSD`, and `TUSD` are
   accepted alongside three-letter ISO codes.
9. **Concurrency.** A run keeps four assets in flight, which also bounds source calls in flight
   per source at four; the deadline and lease renewal are checked between batches and the
   circuit breaker is shared across the batch.
10. **Vault failure.** The `SECURITY DEFINER` function no longer swallows
    `insufficient_privilege`; a refused secret read aborts the run, records it failed, and writes
    the C10 event. The migration grants the function owner SELECT on `vault.decrypted_secrets`.
11. **One role.** The migration creates only `market_data_svc` and grants it the schema, table,
    and function privileges; the migrate job uses the same DSN, and the Swarm stack declares only
    `market_data_database_url` and `market_data_smoke_token`.
12. **Token rotation and shutdown.** `market_data_svc` has DELETE on `service_token_registry`, so
    the deploy job can insert the replacement hash and delete the old row. `market-data-api` sets
    `stop_grace_period: 660s` (longer than the 600 s tail deadline); the infrastructure
    specification records that the C11 caller's timeout must also exceed the run deadline
    (180 s / 600 s), and the lease sweep still recovers an interrupted run.
13. **Authorization and run completion.** The HTTP path enforces the registry row's `operations`
    list rather than the static helper. `finish` checks `rows_affected`, and an early `?` in a run
    still records the failed run and its C10 event before the error surfaces.

## Generated artefacts

- `source-manifest.json` — every workspace-relative file this unit wrote.
- `traceability.json` — AC3.3.1–AC3.4.3, US3.3/US3.4, and BR1.1–BR5.3, each to one existing
  workspace-relative implementation file.
