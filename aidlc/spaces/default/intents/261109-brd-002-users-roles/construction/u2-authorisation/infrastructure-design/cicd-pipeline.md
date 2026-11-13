# CI/CD Pipeline — u2-authorisation

**Upstream:** `infrastructure-specification.md`, `monitoring-design.md`, `reliability-design.md`, `team.md` (Deployment), `project.md` (canonical commands).

## The honest starting point

The repository has **no CI platform** — no `.github/`, GitLab, Jenkins, or CircleCI configuration.
The team chose to leave the platform open and record CI as an intention, so today these gates run
when a person runs them. This pipeline is the sequence a pipeline *would* run, and the unit's code
plan records the same commands as its local definition of done.

**Two of the gates below are red on this tree today, and the pipeline says so rather than
pretending otherwise:**

- **Stage 3 (boundaries)** fails on a pre-existing edge — `reporting-domain` imports `serde` /
  `serde_json` — which this unit did not introduce. Until that crate edge is removed, a blocking
  stage 3 stops every run at the gate. Owner: a separate change to `services/reporting`; this
  unit neither fixes nor hides it.
- **Stage 6 (coverage)** is measured but the 80% per-crate floor is not met by pre-existing
  crates (the last full run reported 58.39% overall for the identity crates). The floor is a
  mandate, so the stage **blocks** for the crates this unit touches and reports the rest.

## Stages

| # | Stage | Command | Gate |
|---|---|---|---|
| 1 | Format | `cargo fmt --all -- --check` | Blocks: formatting drift |
| 2 | Lint | `cargo clippy --workspace --all-targets -- -D warnings` | Blocks: any warning |
| 3 | Boundaries | `bash scripts/check-boundaries.sh` | Blocks — **currently red** on `reporting-domain` → `serde`/`serde_json`, a pre-existing edge owned by a separate change |
| 4 | Unit + integration tests | `cargo test --workspace` | Blocks: any failing test |
| 5 | Adapter tests | disposable Postgres, `TEST_DATABASE_URL=… cargo test --workspace -- --ignored` | Blocks: any adapter or migration failure |
| 6 | Coverage | `cargo llvm-cov --workspace` | **Blocks below 80% per crate for the crates this unit changes** (`identity-domain`, `identity-application`, `identity-http`); reports the rest and records the pre-existing shortfall |
| 7 | Frontends | `yarn --cwd web test`, `lint`, `build`, `format:check`; the same for `support-console` | Blocks: any frontend failure |
| 8 | Image build | Identity service image, tagged with the commit SHA | Blocks: build failure |
| 9 | Vulnerability scan | `cargo audit`, Trivy on the image | Blocks: high/critical findings (required by BRD-012 NFR-SEC-14; **not yet configured** — the stage is specified, not wired) |
| 10 | Migrate | `identity-migrate` against the target DSN, once, before the service update | Blocks: migration failure (this unit ships no migration) |
| 11 | Deploy | Swarm rolling update: `parallelism: 1`, `failure_action: rollback`, `monitor: 30s`, health check on `/healthz` | Blocks: a failing health check triggers the automatic rollback |
| 12 | Rollback | The stack's `failure_action: rollback` restores the previous task set | Works today for the task set. **The automatic down-migration step is not built** — no release job exists and nothing applies a `.down.sql`; this unit ships no schema change, so the gap does not affect it, but it must not be described as working |
| 13 | Smoke | `scripts/smoke-identity.sh` — **to be added by this unit's code plan** | Blocks: the script exits non-zero |

### Stage 13 — what the smoke script is

A small shell script the unit adds, invoked by a person or a future pipeline as
`scripts/smoke-identity.sh <base-url>`. Given `base-url` (the Identity service inside the
cluster), a `C4_SMOKE_TOKEN` environment variable (a throwaway service token minted by the same
mechanism the caller uses), and a `SMOKE_USER_JWT` (a test user's token from the local stack), it:

1. `GET /healthz` and requires 200;
2. `GET /readyz` and requires 200, or a body naming the status source as degraded;
3. `POST /internal/authorize` with the operation `principal:read`, the caller's own user id as
   `resourceOwnerId`, and both tokens, requiring 200 with `allowed: true`;
4. the same call with a foreign owner id, requiring 404 with `not_owner`.

It exits non-zero on the first mismatch. No smoke script exists in the repository today; this
one is part of the unit's build, not an assumption about the platform.

## Deployment strategy and rollback

- **Rolling**, as configured: Identity API services roll `start-first`, so a new task starts before
  the old one stops and the health check gates the swap.
- **Rollback trigger:** a failed post-update health check, implemented by `failure_action:
  rollback` and a `rollback_config` that pauses on failure.
- **Schema:** this unit ships no migration, so its rollback needs no down step. The automatic
  down-migration step for releases that *do* carry a schema change is **not built** (no release
  job exists) and is recorded as an open platform gap, not claimed here.
- **Feature flags:** none. A misbehaving decision is corrected by a redeploy; a flag would add a
  second code path in front of every request for no operational gain.

## Secrets in CI/CD

- The C4 per-service tokens live in each service's `*_service_tokens` Docker secret, materialised
  from Supabase Vault in production — never in the pipeline configuration, never in the
  repository.
- The Identity service's Supabase admin credential follows the same secret path.
- A test run mints a throwaway token through the local stack; that stack is explicitly not a
  production credential model.

## What must exist before this unit can ship

1. The C4 contract and C4 model update (subject and role from the JWT; `callerSubject` and
   `roleClaim` rejected; operation enum; 503; the two new reasons).
2. The caller wiring in `infra/swarm-stack.yml`: the two Identity service-token entries, the
   missing `account-api` `IDENTITY_URL`, and the Portfolio port corrected from 8089 to 8083.
3. The Prometheus service and scrape job (see `monitoring-design.md`) before any alert is armed.
4. `cargo audit` and Trivy wired into the pipeline the team eventually chooses.
