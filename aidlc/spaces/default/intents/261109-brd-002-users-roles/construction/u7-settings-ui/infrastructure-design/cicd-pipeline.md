# CI/CD Pipeline — u7-settings-ui

**Upstream:** `infrastructure-specification.md` (this unit — three rebuilds, no stack change),
`monitoring-design.md` (the build-time size gate), `nfr-design/performance-design.md` (§1 route
budget, §9 verification), `nfr-design/security-design.md` (§4 transport, §10 rate limiting),
`nfr-requirements/tech-stack-decisions.md` (T1–T6).

## The honest starting point

**There is no CI platform in this repository.** No `.github/`, no GitLab, Jenkins or CircleCI
configuration exists, and the team chose to leave the platform open and record CI as an intention.
The stages below are therefore the pipeline **as specified**, every one of them a command a person
runs today from the repository root. The authoritative command list lives in
`aidlc/spaces/default/memory/project.md`; this file does not redefine it, it sequences it and says
which stages block a release.

Unlike the frontend-only gate unit that precedes this one, **this unit touches Rust**: it adds
handlers to `services/identity-*/` and route entries to `services/gateway-*/`. Both gate families
apply, and the sequencing below is built around that.

## Stages

| # | Stage | Commands | Gate | Notes |
|---|---|---|---|---|
| 1 | Rust format | `cargo fmt --all -- --check` | **blocking** | Cheap, first, so a formatting failure costs seconds |
| 2 | Rust lint | `cargo clippy --workspace --all-targets -- -D warnings` | **blocking** | Includes the new Identity handlers and gateway `Route` entries |
| 3 | Architecture boundaries | `bash scripts/check-boundaries.sh` | **blocking** | Mechanical: parses `cargo metadata --no-deps`, fails on any outward crate edge. The new code is projections and handlers in `http`/`application`; the risk this catches is a projection helper reaching into `infrastructure` |
| 4 | Rust tests (unit + in-process) | `cargo test --workspace` | **blocking** | Covers the new self-scoped routes and handlers, the profile projection and the `route_label` arms added for them, plus the cases they exercise in the **existing** deletion lifecycle (`services/identity/domain/src/lifecycle.rs`, `deletion.rs`) and cancel path — this unit does not rewrite that state machine. HTTP boundary tests assert status codes and RFC 7807 bodies |
| 5 | Rust tests (adapter + migration, `#[ignore]`d) | provision a disposable Postgres, export `TEST_DATABASE_URL` for its DSN, then `cargo test --workspace -- --include-ignored` | **blocking** | Team rule: every run provisions a disposable database, so an adapter regression fails the change rather than being a developer opt-in. This unit adds **no migration**, so stage 5 is about the existing Identity and gateway repository suites still passing against the new statements; the up/down pair rule is not exercised here |
| 6 | SPA install | `yarn install --immutable` in `web/` | blocking | Yarn 4.18.1 through corepack; a lockfile drift fails |
| 7 | SPA lint + format | `yarn lint`, `yarn format:check` in `web/` | blocking | ESLint 9 flat config, Prettier 3 |
| 8 | SPA tests | `yarn test` in `web/` | blocking | Vitest 3 with Testing Library, jest-axe on every rendered state, MSW for D0/P1/P2/D1/D2/D3/X2–X6 including the delayed and hung handlers that prove the deadline surfaces |
| 9 | SPA build | `yarn build` in `web/` | blocking | `tsc -b && vite build`; produces the hashed route chunk |
| 10 | SPA size budget | `yarn size` in `web/` | **blocking** | size-limit: 300 kB initial, 150 kB route chunk, 150 kB stylesheet, all gzip — this is the mechanical NFR2.1 gate, and it runs against the new `*Page` chunk via the existing route glob |
| 11 | Support console | `yarn lint`, `yarn format:check`, `yarn test`, `yarn build` in `support-console/` | blocking | **Not changed by this unit**, but the `web` image builds both, so a broken console breaks the same release artifact — the stage runs rather than being assumed green |
| 12 | Image build | build `identity-api`, `gateway`, `web` images | blocking | Three existing Dockerfiles; no new one. Image digests are what stage 13 references |
| 13 | Vulnerability scan | `cargo audit`; Trivy against the three images | **specified, not wired** | BRD-012 NFR-SEC-14 requires dependency and image scanning before release. No configuration exists in the repository, so this stage is an intention with an owner, not a running gate — the honest status is stated here rather than implied by its presence in the table |
| 14 | Deploy | Swarm service update, in the order below | blocking on health check | `infra/swarm-stack.yml` as it stands |
| 15 | Smoke | `curl /healthz` per updated service; `scripts/smoke-identity.sh <base-url>` where a C4 token is available | advisory today | The smoke script exists and writes no file; the stack reserves `*_smoke_token` secrets for a release job that is not in the repository |

Stages 1–5 are the Rust half and 6–11 the frontend half; they can run in that order in one job or
as two parallel jobs, because nothing in the frontend depends on the Rust stage output. Stage 12
waits for both.

## Test-automation integration

- **TDD is the ordering rule, not a stage.** Each test lands in the same change as the code it
  covers; the pipeline only proves the tests exist and pass. Nothing here can detect write-order,
  and the design does not pretend otherwise.
- **The deleted calls are tested by absence.** The `client.ts` removals (`/users/me`,
  `/preferences`, `/consent`, `/sessions`) mean the MSW handlers those tests used are gone too; the
  suite proves the settings routes are the ones the SPA calls, so a stale call resurfacing is a
  failing test rather than a runtime 404.
- **Gateway entries are tested at the gateway's own HTTP boundary** — a request to each of the eight
  paths is asserted to reach a handler rather than 404 at the deny-by-default table, and D3's
  5/min budget is asserted to answer `429` on the sixth request in a window. This is the only way
  the eight declarations in `infrastructure-specification.md` "What must exist" are proven shipped.
- **The pinned no-record `404`** is asserted on both sides: Identity renders
  `https://crypto-pocket-butler.example/problems/no-record`, and the client's `nullOnNoRecord`
  accepts only that type, so any other `404` stays an error.
- **No end-to-end browser journey suite exists** and this unit adds none. The MSW-backed suite plus
  the gateway boundary tests are the coverage, and the gap is recorded rather than papered over.

## Deployment strategy

Rolling update, already configured — this unit changes no deployment policy:

| Service | Order | Replicas | Update policy | Health gate |
|---|---|---|---|---|
| `identity-api` | `start-first` | 3 | `parallelism: 1`, `monitor: 30s`, `failure_action: rollback` | `/healthz` on the new task (`http://127.0.0.1:8083/healthz`). `/readyz` exists on the service but is **not** wired into the Swarm healthcheck, so the rolling update gates on liveness only — the stack change that would add it is not this unit's |
| `gateway` | `start-first` | 2 | same | `/healthz` |
| `web` | `start-first` | 2 | same, `stop_grace_period: 15s` | `/healthz` on nginx |

**Release order within the rollout: `identity-api` and `gateway` before `web`.** A `web` task whose
bundle calls `/v1/me/account-status` before the gateway table declares it receives a `404`; the entry
surface renders that as the full-page retry with a correlation id. That is a visible, recoverable
failure rather than a wrong state — which is why the order is stated here as a stage requirement and
not left to whichever service Swarm updates first (they update independently).

No blue-green, no canary, no staging tier: production is the only environment (team rule), and the
rolling update with a health gate is the strategy. `deployment/localhost` is a developer convenience
that runs the same three images on one machine; it is not a promotion step and nothing deploys
through it.

## Rollback

| Step | Action | Notes |
|---|---|---|
| 1 | Let Swarm roll back | `failure_action: rollback` restores the previous task definition for a service whose new task fails its health check, with the `rollback_config` pause. No human action is needed for the health-check case |
| 2 | Roll back a bad release manually | `docker service update --rollback <service>` per service, `web` first if the failure is presentation-side, `gateway`/`identity-api` only if the server side is at fault |
| 3 | Schema step | **None.** This unit ships no migration, so there is no `.down.sql` to apply and no ordering question between a migrate job and a service update |
| 4 | Verify | `/healthz` per rolled-back service; a retried settings route returns the previous answer |

A `gateway` rollback removes the eight route entries, so the SPA's settings calls `404` until the
matching `web` image is also rolled back — the two roll back together, which the fail-safe surface
above makes survivable in the interim. Nothing in the rollback path loses user data: preferences and
deletion requests are server rows, and the client cache is memory-only.

## Environment promotion

There is none. A change lands on the single production Swarm or it does not land; the local compose
stack exists so a developer can exercise the path first, and the same build/lint/test commands run
in both cases. The only promotion-like sequencing is the intra-release service order above.

## Secrets in CI/CD

- **This unit adds no credential** (`infrastructure-specification.md`, Infrastructure Services). The
  pipeline needs none beyond what the repository already uses: the registry credential for the image
  push, and — for stage 5 — a disposable database DSN it creates itself.
- **`SMOKE_*` / `C4_SMOKE_TOKEN` are test material only.** `scripts/smoke-identity.sh` documents them
  as a throwaway token minted by the same mechanism the callers use, never a production credential.
  The stack reserves `*_smoke_token` secrets for a release job that is not in the repository.
- **No secret is ever in a build stage's output.** Rust and SPA builds read no credential; the
  `web` image bakes static assets only, and `identity-api` reads its credentials at run time from
  Docker secrets and Supabase Vault, as it does today. A stage that would need one fails rather than
  reading an environment variable into the build.

## What must exist before this unit can ship

| Item | Stage that proves it |
|---|---|
| Six Identity routes serving (D0, P1, P2, D1, D2, D3) | 4, 5 (HTTP boundary and adapter suites) |
| Eight gateway route-table entries, including D3 at 5/min | 4 (gateway boundary test: route resolves; sixth call `429`) |
| `route_label` arms for the six settings routes | 4 (an Identity middleware test asserts each new path labels itself rather than `other`) |
| D1's pinned `https://crypto-pocket-butler.example/problems/no-record` | 8 (client) and 4 (server rendering), plus the Contract Design row it depends on |
| Route chunk within the 150 kB budget | 10 |
| Both SPA packages green | 7–11 |
| Clean Architecture edges intact | 3 |

The remaining external prerequisite — contract-summary rows for the six routes and the two export
entries in C1/C2 — is a documentation dependency that no stage here can enforce; it is recorded in
`security-design.md` §14.4 and `infrastructure-specification.md` so Code Generation cannot assume it.