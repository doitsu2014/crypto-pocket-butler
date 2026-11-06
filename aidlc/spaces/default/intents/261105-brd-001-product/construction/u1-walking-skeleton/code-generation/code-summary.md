# Code Summary — u1-walking-skeleton

## What was built

The thinnest end-to-end Asset Registry path:

```
React SPA shell → API Gateway → Asset Registry → Supabase Postgres (registry schema)
```

- **Root Rust workspace** (`Cargo.toml`) with two services, each a Clean Architecture set of four
  crates plus a thin composition-root package:
  - `services/asset-registry/{domain,application,infrastructure,http}` + `asset-registry` binary
  - `services/gateway/{domain,application,infrastructure,http}` + `gateway` binary
- **Asset Registry**
  - Domain: `Asset`, `Chain`, `TokenContract`, `PriceSource`, `UnmappedObservation`, `Price`, and
    the BR1.1/BR1.2/BR2.1/BR2.2 invariants.
  - Application: `ResolveAsset` (WF1, BR1.1/BR1.3), `ResolvePrice` (WF2, BR2.1/BR2.2),
    `RegistryAdmin` (WF3, BR1.2/BR2.1/BR3.1), all behind inward-defined ports.
  - Infrastructure: sqlx repositories, versioned up/down migrations, JWKS validator with a
    5-minute refresh and a 10-minute stale window, local structured audit sink, DB readiness.
  - HTTP: Axum routes for `GET /v1/assets/resolve`, `GET /v1/assets/{id}/price`, the
    `/v1/admin/*` registry operations, `/healthz`, `/readyz`, `/metrics`; RFC 7807 errors with
    `correlationId`; JWT enforcement and admin-role gating.
- **API Gateway**
  - Domain: deny-by-default route table, path patterns, correlation-id validation, error mapping.
  - Application: `AuthorizeAndRoute` — validate JWT, resolve the subject as principal, match the
    declared route, enforce the admin role, honour/generate the correlation id, forward the JWT
    unchanged, bounded retries for idempotent methods, idempotency keys for mutations.
  - Infrastructure: JWKS validator (same cache semantics), reqwest upstream client (no redirects,
    hop-by-hop headers stripped), structured audit sink, registry readiness probe.
  - HTTP: edge proxy, security headers, in-memory fixed-window rate limits (anonymous stricter),
    request/error metrics, `/healthz`, `/readyz`, `/metrics`.
- **Web SPA shell** (`web/`): Vite + React 18 + TypeScript 5, Tailwind CSS v4 mapped to Parchment
  Ledger tokens, TanStack Query mutation, one API-client module, in-memory-only session token,
  vitest + Testing Library tests.
- **Deployment**: multi-stage non-root Dockerfiles for both services and the SPA, an nginx SPA
  server config, a Docker Swarm stack with 2 replicas each, health-gated updates with
  `failure_action: rollback`, and a registry schema provisioning snippet.

## How to run

### Backend

```bash
cargo test --workspace                       # all unit + service-scoped tests
cargo test --manifest-path services/asset-registry/Cargo.toml
cargo test --manifest-path services/gateway/Cargo.toml
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

Integration tests against a real Postgres schema (they create and drop a disposable schema):

```bash
docker run --rm -d --name cpb-test-postgres \
  -e POSTGRES_PASSWORD=test -e POSTGRES_USER=test -e POSTGRES_DB=cpb_test \
  -p 55432:5432 postgres:16-alpine

TEST_DATABASE_URL=postgres://test:test@localhost:55432/cpb_test \
  cargo test --manifest-path services/asset-registry/Cargo.toml --test repositories -- --ignored
```

Run the services locally (migrations run at startup):

```bash
DATABASE_URL=postgres://... \
JWT_JWKS_URL=https://<project>.supabase.co/auth/v1/.well-known/jwks.json \
JWT_ISSUER=https://<project>.supabase.co/auth/v1 \
JWT_AUDIENCE=authenticated \
cargo run -p asset-registry

ASSET_REGISTRY_URL=http://127.0.0.1:8081 \
JWT_JWKS_URL=... JWT_ISSUER=... JWT_AUDIENCE=... \
cargo run -p gateway
```

### Frontend

```bash
yarn --cwd web test         # vitest run
yarn --cwd web lint         # ESLint
yarn --cwd web build        # tsc -b && vite build
```

### Deployment

```bash
docker build -f services/asset-registry/Dockerfile -t <registry>/asset-registry:<tag> .
docker build -f services/gateway/Dockerfile -t <registry>/gateway:<tag> .
docker build -f web/Dockerfile -t <registry>/web:<tag> .

printf '%s' "$DATABASE_URL" | docker secret create registry_database_url -
REGISTRY=<registry> TAG=<tag> JWT_JWKS_URL=... JWT_ISSUER=... JWT_AUDIENCE=... \
  docker stack deploy -c infra/swarm-stack.yml cpb
```

Rollback: `docker service update --rollback cpb_asset-registry` (and the same for gateway/web).
A failed post-update health check triggers the stack's `failure_action: rollback` automatically.
The registry schema/role is provisioned once from `infra/sql/registry-schema.sql`.

## Test evidence

| Suite | Result |
|---|---|
| `services/asset-registry` (domain 14, application 27, HTTP 13, smoke 1) | 55 passed |
| `services/asset-registry` repository integration against Postgres 16 | 7 passed |
| `services/gateway` (domain 10, application 14, HTTP 12, JWT 9, upstream 4, smoke 1) | 50 passed |
| `web` (API client 3, SPA shell 4) | 7 passed |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `yarn --cwd web lint` | clean |
| `yarn --cwd web build` | success (182 kB JS / 11 kB CSS) |

TDD order was kept per layer: the failing test file was added first, run to record the failure,
then the implementation was written to pass. Repository tests were written first and run
against a disposable Postgres schema.

## Key decisions

1. **Single root Cargo workspace + service composition-root packages.** The two services are
   members of one workspace so `cargo fmt --all` and `cargo clippy --workspace` cover
   everything. Each service directory also has a `Cargo.toml` (the composition root and the
   service-scoped test suite) so the approved
   `cargo test --manifest-path services/<name>/Cargo.toml` commands run the full service suite.
2. **Native assets are modelled in a `native_assets` join table.** The entity list has no native
   flag; BR1.1 requires symbol-only resolution for native assets. The table keeps that rule
   chain-scoped and is seeded with ETH/BNB/SOL at launch coverage.
3. **Contract addresses keep their observed case but also store a family-aware lookup key.**
   EVM/BNB compare case-insensitively, Solana exactly, and `(chain_id, contract_address_key)` is
   unique, so one chain-plus-contract observation can never resolve to two assets (BR1.1).
4. **JWT validation is implemented in both services.** The gateway validates before routing and
   forwards the token unchanged; the registry validates again. Both use a 5-minute refresh and a
   10-minute stale window, then fail closed with 503.
5. **Gateway retries are bounded and idempotency-aware.** GET/HEAD/OPTIONS/PUT/DELETE retry up
   to three attempts on timeout/unavailable; POST/PATCH are not retried and always carry an
   `Idempotency-Key` (caller-supplied keys are preserved).
6. **The price provider is a placeholder.** `NoPriceProvider` returns "no usable price" until
   Market Data (U5) exists, so the registry reports unpriced and never zero (BR2.2). The
   `GET /v1/assets/{id}/price` route exposes WF2 ahead of the C4 contract's next revision.
7. **Yarn 4.18.1 is the only Yarn release that can be pinned.** The project rule says Yarn ≥ 6,
   but no Yarn 6 release exists; 4.18.1 is the current Berry release and is pinned through
   corepack. A minimal root `package.json` was added so the approved `yarn --cwd web test`
   command resolves the pinned version from the repository root.
8. **Rate limiting keys on the forwarded client IP and token presence**, with a stricter
   anonymous budget. The regional dimension remains the edge/CDN's job.

## Known gaps and follow-ups

- **Audit coupling.** A registry change is written, then audited; if the local audit sink fails
  the use case returns an error but the write is not rolled back. Transactional coupling with
  the Audit service (U12) is required to satisfy BR3.1 strictly. This is covered by a test and
  recorded here rather than hidden.
- **Identity.** Until U3 exists, the gateway uses the validated JWT subject as the principal
  (as the functional spec allows) and does not call Identity.
- **Market data.** The price route is wired but always unpriced until U5 supplies providers.
- **Account/portfolio.** AC1.2.3 (wallet with no balances) needs U4; the shell's unresolved
  state is the skeleton's honest stand-in.
- **Coverage tooling.** The team's 80% per-crate line-coverage floor is not measured because the
  coverage tool is unresolved; the suite is green and clippy-clean.
- **CI platform and ordered gate list** remain unresolved project-wide.
- **Repository hygiene.** The existing `.gitignore` does not cover `target/`, `web/node_modules/`,
  `web/dist/`, `web/.yarn/`, or `web/tsconfig.tsbuildinfo`. They were left untouched per the
  instruction not to change pre-existing files; add them before the first commit.
- **No response-body size cap on upstream responses** beyond the reqwest default; add one when
  a real provider returns large payloads.

## Pre-existing files touched

- `aidlc/spaces/default/intents/261105-brd-001-product/construction/u1-walking-skeleton/code-generation/code-generation-plan.md`
  — completion boxes ticked only.
- `aidlc/spaces/default/intents/261105-brd-001-product/construction/u1-walking-skeleton/code-generation/unit-test-instructions.md`
  — the exact integration-test command was appended.
- No application, design-system, documentation, or configuration file that existed before this
  work was modified. The root `package.json`, `.dockerignore`, and Rust `Cargo.lock` are new files
  required by the approved build and test commands.
