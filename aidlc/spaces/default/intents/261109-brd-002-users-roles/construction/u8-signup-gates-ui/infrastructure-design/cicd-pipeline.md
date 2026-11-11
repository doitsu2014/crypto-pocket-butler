# CI/CD Pipeline — u8-signup-gates-ui

**Upstream:** `infrastructure-specification.md`, `monitoring-design.md`, `performance-design.md`
(§9 verification), `team.md` (Deployment), `project.md` (canonical commands).

## The honest starting point

The repository has **no CI platform** — no `.github/`, GitLab, Jenkins, or CircleCI
configuration; the team chose to leave the platform open and record CI as an intention, so today
these gates run when a person runs them. This pipeline is the sequence a pipeline *would* run,
and the unit's code plan records the same commands as its local definition of done. Only the
frontend gates apply: the unit touches no Rust code.

One shared artifact shapes the whole pipeline: `web/Dockerfile` builds the customer SPA **and**
the support console into one image, so every stage below that passes also redeploys the console.
That coupling is existing platform behaviour, identical for any `web` change, and is recorded
rather than worked around.

## Stages

| # | Stage | Command | Gate |
|---|---|---|---|
| 1 | Format | `yarn --cwd web format:check` | Blocks: Prettier drift |
| 2 | Lint | `yarn --cwd web lint` | Blocks: any error (`@typescript-eslint/no-explicit-any` is an error) |
| 3 | Type + build | `yarn --cwd web build` (`tsc -b && vite build`) | Blocks: type or build failure |
| 4 | Unit tests | `yarn --cwd web test` (Vitest + Testing Library + jest-axe) | Blocks: any failure — includes the deadline/retry/request-count suites and the axe passes for every rendered state (performance-design §9, NFR25.4). Gateway mocking: `web/` has **no msw dependency today** — its existing tests use the hand-rolled fetch stub in `web/src/test/gateway.ts` (`vi.stubGlobal("fetch", …)`). The nfr-design chose MSW-style handlers for the gate suite (logical-components T1), so the unit's code plan records **adding `msw` as a `web` devDependency** as a reviewed dependency change (dev-only, cannot reach the production bundle — security-design §3); until it lands, the existing fetch stub is the harness of record |
| 4b | Support-console gates | `yarn --cwd support-console test`, `lint`, `build`, `format:check` | Blocks: any failure. The console's gates are unchanged by this unit, but the shared `web` image (stage 6) also carries the console bundle — a console regression would ship inside it, so its canonical gates run in the same pipeline pass |
| 5 | Size budget | `yarn --cwd web size` (size-limit + `scripts/check-size.mjs`) | Blocks: NFR2.1's mechanical gate — 300 kB initial, 150 kB per route chunk, 150 kB stylesheet, gzip. No config change; the `*Page` module name keeps the gate chunk inside the `route-*.js` glob |
| 6 | Image build | `docker build -f web/Dockerfile -t <registry>/web:<commit-sha> .` from the repo root | Blocks: build failure. `VITE_*` args are public client configuration baked at build time — never secrets |
| 7 | Vulnerability scan | dependency audit + Trivy on the `web` image | Blocks: high/critical findings (BRD-012 NFR-SEC-14). **Not yet configured** — specified, not wired; the same gap u2's pipeline recorded |
| 8 | Deploy | Swarm rolling update of `web`: `parallelism: 1`, `order: start-first`, `failure_action: rollback`, `monitor: 30s`; `/healthz` gates the swap | Blocks: a failing health check triggers the automatic rollback |
| 9 | Rollback | The stack's `failure_action: rollback` restores the previous task set | Complete for this unit: no migration, no down step, no persistent state — the static bundle is the whole deployment |
| 10 | Smoke | `/healthz` returns 200 — the stated release-completion signal (team memory) | The stack reserves `*_smoke_token` secrets for a release job that does not exist; **this unit adds no smoke script**: an end-to-end gate-flow smoke needs G2/G3 to exist (below) and belongs to the gateway/U1 release check, not to a static bundle |

## Deployment strategy and rollback

- **Rolling, `start-first`**, as configured for `web`: a new task starts and passes `/healthz`
  before the old one stops, so the swap is zero-downtime for the static host.
- **Mid-roll chunk requests:** `index.html` is `no-store` and hashed assets are immutable with a
  1-year cache, so a client picks up the new shell at once. A session already open can still
  request a *previous* build's route chunk after the old task is gone — a 404 the router sees as
  a chunk-load failure. The **route-level error boundary this unit ships** (performance-design
  §3) renders that as the full-page retry inside the gate route instead of blanking the app; the
  boundary is the deployment-timing safety net as well as the offline one.
- **Rollback trigger:** a failed post-update health check — the only stated trigger, implemented
  by `failure_action: rollback`. No schema change exists, so the unbuilt automatic
  down-migration release job is irrelevant to this unit and is not claimed here.
- **Feature flags:** none. The route guard and the gateway's deny-by-default table are the
  switches that matter, and both are server-owned; a flag inside the bundle would add a second
  code path for no operational gain.

## Secrets in CI/CD

None. The feature holds no secret, the `web` service declares no secrets, and the Supabase URL
and anon key baked via `VITE_*` build args are public client configuration (web/Dockerfile
comment; project secrets rule). The pipeline needs registry-push credentials only — the same
credential every image stage uses.

## What must exist before this unit can ship

1. **Gateway route-table entries** — the read-only entry lookup (G1 mode or its replacement) and
   `GET /v1/signup/gate-state` (G2), `POST /v1/signup/gates` (G3): without them the
   deny-by-default table 404s every gate call and every signed-in visitor lands on the full-page
   retry — fail-safe, but unusable (security-design §11 deps 1 and 5).
2. **G3's server behaviour** — the C8 → C3 → C6 ordering, `Idempotency-Key` handling, and
   skip-already-recorded semantics (deps 2 and 5; U1/U4/U6).
3. **`jurisdictionBlocked` source and gateway IP derivation** pinned (deps 3 and 4).
4. **Vulnerability scanning wired** into whichever platform the team chooses (stage 7).

Sequencing consequence: the SPA bundle may deploy *before* those land only if the gate route is
not yet linked into the shipped routing (it fails safe either way); the functional release of
the gate flow ships **after** items 1–3 are in production. No `infra/swarm-stack.yml` change is
required from this unit at any point.
