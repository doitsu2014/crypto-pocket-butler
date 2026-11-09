# Localhost stack

Runs the whole of Crypto Pocket Butler on one machine with Docker Compose:

- **Supabase, self-hosted**: Postgres 15 (`supabase/postgres`, with pgsodium),
  Auth (GoTrue), REST (PostgREST), Storage, Studio, Mailpit and Kong.
- **Every Rust service and worker** in the repository, plus the API Gateway and
  the Asset Registry, each built from its own `services/*/Dockerfile`.
- **The schema migrations** for all eleven service schemas, applied once by a
  provisioning job, and the local service-token registries.
- **The SPA and the support console**, served by nginx at `/` and `/support`,
  with `/v1` proxied to the gateway exactly as the CDN does in production.

This folder is a developer convenience. Production is the Docker Swarm stack in
`infra/swarm-stack.yml` with Vault-issued credentials and one image per service;
nothing here is a production credential or deployment model.

## Prerequisites

- Docker Desktop (or another Docker engine with Compose v2) and roughly 8 GB of
  memory free. The first `--build` compiles each service image from its own
  Dockerfile.
- Node.js on the host (the repository already requires it for the web app) to
  generate the local keys.
- No other Supabase stack on ports 54321-54324. If the Supabase CLI stack is
  running, stop it first (`supabase stop`).

## Start

```bash
cd deployment/localhost
./scripts/init-env.sh          # generates .env, secrets/ and the Kong config
docker compose up --build
./scripts/create-admin.sh      # once the stack is healthy
```

`init-env.sh` is idempotent: it leaves an existing `.env` alone. To rotate every
local key, delete `.env`, `secrets/` and `supabase/kong.generated.yml` and run it
again.

### Admin login

`create-admin.sh` creates `admin@cpb.local` with the password stored as
`ADMIN_PASSWORD` in `.env`. The role is set in `app_metadata.role = "admin"`,
which is where the services read it from (the top-level `role` claim stays
`authenticated`, as Supabase expects).

```bash
grep '^ADMIN_' .env
```

## Endpoints

| What | URL |
|---|---|
| SPA | http://localhost:8080 |
| Support console | http://localhost:8080/support/ |
| Supabase API (Kong) | http://127.0.0.1:54321 |
| Supabase Auth JWKS | http://127.0.0.1:54321/auth/v1/.well-known/jwks.json |
| Supabase Studio | http://127.0.0.1:54323 |
| Mailpit (auth emails) | http://127.0.0.1:54324 |
| Postgres | `postgresql://postgres:postgres@127.0.0.1:54322/postgres` |
| Gateway | http://localhost:8082 |
| Asset Registry | http://localhost:8081 |
| Identity API / worker | http://localhost:8083 / http://localhost:8085 |
| Audit API | http://localhost:8084 |
| Account API / worker | http://localhost:8091 / http://localhost:8092 |
| Market Data API / worker | http://localhost:8093 / http://localhost:8094 |
| Analytics API / worker | http://localhost:8095 / http://localhost:8096 |
| Ledger API / worker | http://localhost:8097 / http://localhost:8098 |
| Notification API / worker | http://localhost:8099 / http://localhost:8100 |
| Portfolio API / worker | http://localhost:8101 / http://localhost:8102 |
| Reporting API / worker | http://localhost:8103 / http://localhost:8104 |
| Job Orchestration | http://localhost:8105 |

The per-service ports are published for `curl` and debugging. The SPA only ever
talks to its own origin: nginx proxies `/v1` to the gateway, and the gateway
forwards to the owning service.

## How authentication is wired

The services validate access tokens against a JWKS over RS256 only. A Supabase
project's default HS256 shared secret therefore cannot satisfy them. This stack
generates an RSA key pair in `init-env.sh` and gives the private half to Auth
(`GOTRUE_JWT_KEYS`); Auth signs every access token with it and publishes the
public half at `/.well-known/jwks.json`, which every service fetches at
`http://auth:9999/.well-known/jwks.json`.

The anon and service-role API keys are RS256 JWTs signed by the same key, with
`iss`, `aud` and `role` claims; Kong, PostgREST and Storage all validate them
with the same public JWKS. `SUPABASE_JWT_ISSUER` in `.env` is used for both
GoTrue and every service, so the issuer check cannot drift.

## Provisioning

`init/01-roles.sql` creates the eleven schema names and gives every service role
a local LOGIN and password (production mints these from Supabase Vault instead).
`scripts/provision.sh` then applies each service's versioned `*.up.sql`
migrations with `search_path` pinned to that service's schema, and seeds the
service-token registries with the digests of the generated tokens.

The local seeding is deliberately permissive: every caller may perform every
operation its target defines, while caller names stay exact (so the
identity-only privacy paths keep their checks). Do not reuse it anywhere shared.

## Known gaps

- **The gateway only routes Asset Registry paths.** The walking-skeleton route
  table (`services/gateway/domain/src/route.rs`) declares the `/v1/assets`
  paths; other services are reachable directly on their published ports but not
  through `/v1` yet. Sign-in, the shell and the Asset Registry calls work; other
  data screens will show errors until those routes are added.
- **Email delivery does not work against Mailpit.** Notification posts to
  `POST {EMAIL_PROVIDER_URL}/v1/messages`, which is not Mailpit's API. Auth
  emails (confirmation, recovery) do go to Mailpit over SMTP; set a compatible
  provider URL in `.env` to exercise alert delivery.
- **No market-data provider is configured.** `MARKET_DATA_PROVIDER_URLS` is
  empty, so prices and FX stay empty until you point it at a provider.
- **Account needs reachable RPC endpoints.** `EVM_RPC_URL` and `SOLANA_RPC_URL`
  default to public endpoints in `.env`; replace them if you need real balances.
- **Realtime is not included.** Crypto Pocket Butler does not use it.
- **Supabase Storage runs the file backend** in the `storage-data` volume. The
  `identity-exports` bucket is created once Storage has migrated its schema.

## Reset

```bash
docker compose down -v      # removes the database and storage volumes
docker compose up --build
```

`docker compose down` alone keeps the data. Logs:

```bash
docker compose logs -f gateway web
```
