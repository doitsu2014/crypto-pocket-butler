# CI/CD Pipeline — u9-support-console-ui

**Upstream:** `infrastructure-specification.md`, `monitoring-design.md`, `nfr-design/
security-design.md` (§3 budgets, §11 blockers), `nfr-design/logical-components.md` (§1.3),
`nfr-design/performance-design.md` (§8), `functional-spec.md` (§8 S1–S3, §12), team memory
(production-only, rolling Swarm update, TDD, gates run by a person today).

## Where this unit sits in a pipeline that does not exist yet

There is **no CI platform** in this repository — no `.github/`, no GitLab, Jenkins or CircleCI
configuration. The canonical command list in `aidlc/spaces/default/memory/project.md` is run by a
person, and `scripts/check-boundaries.sh` and `scripts/smoke-identity.sh` are the two scripted
gates. This document describes the pipeline this unit would need, naming which stages exist as
commands today and which are still to be built; nothing below is claimed to run at merge.

## Stages

| # | Stage | Command(s) | Blocking | Notes for this unit |
|---|---|---|---|---|
| 1 | Format | `cargo fmt --all -- --check` | yes | Runs because the unit edits a Rust **comment** (`services/gateway/infrastructure/src/upstream.rs:88-90`); the file must stay rustfmt-clean like any other |
| 2 | Lint | `cargo clippy --workspace --all-targets -- -D warnings` | yes | Same reason. No new Rust code, so no new lints — a clippy failure here means the tree was already red |
| 3 | Boundaries | `bash scripts/check-boundaries.sh` | yes | Still runs; the comment edit cannot create an outward crate edge, and the gate proves it |
| 4 | Rust tests | `cargo test --workspace` after provisioning a disposable Postgres and exporting `TEST_DATABASE_URL` | yes | The `#[ignore]`d adapter and migration suites are a blocking gate (interview Q5), not a developer opt-in. This unit changes no Rust behaviour, so it is a regression guard, not a new-coverage stage |
| 4b | Coverage | `cargo llvm-cov` | target | 80% line coverage per Rust crate. A comment-only change moves no line of coverage; recorded so the stage is not silently skipped |
| 5 | Console format | `yarn --cwd support-console format:check` | yes | Prettier 3 through Yarn 4.18.1 (corepack) |
| 6 | Console lint | `yarn --cwd support-console lint` | yes | ESLint 9 flat config; `@typescript-eslint/no-explicit-any` is an error |
| 7 | Console tests | `yarn --cwd support-console test` | yes | Vitest 3 + Testing Library + jest-axe over the MSW handlers. Asserts the one-attempt rule (NFR2.1/NFR2.7), the `403` scoped clear (NFR10.4), the deadline path (NFR2.2) and axe on every rendered state (NFR25.5). The TDD rule is enforced here at review, not by the pipeline: the suite lands in the same change as the code it covers |
| 8 | Console build | `yarn --cwd support-console build` (`tsc -b && vite build`) | yes | TypeScript must compile and Vite must emit `dist/`. **There is no `size` script in `support-console`** (only `web` defines `yarn size`), so chunk size is a review check against `web`'s route-chunk budget; this unit shrinks the bundle, it does not grow it |
| 9 | Image build | `docker build` of `web/Dockerfile` | yes | The **only** image this unit changes: the console compiles into the same image as the customer SPA (`web/dist/support`). No other image is rebuilt by this unit; the Rust comment edit does not change `gateway` behaviour but the image is rebuilt with the next gateway release, not for this unit |
| 10 | Vulnerability scan | `cargo audit`, Trivy on the `web` image | **required, not configured** | BRD-012 NFR-SEC-14 requires it; no configuration or pipeline exists. This unit adds no dependency — the console's dependency set is unchanged except removals — so it inherits the gap rather than widening it. Recorded as a platform obligation |
| 11 | Deploy — server half first | The change that delivers `GET /v1/support-grants/mine`: gateway `Route` entry, Identity handler and `list_by_support_agent`, the `route_label` arm, **and** its `(support_agent_id, requested_at DESC)` index migration pair | yes | **Ordering is the load-bearing stage, and it has no executor today.** The path is in no `Route` entry and returns `404`; U3's work has already landed without it, so "U3's release" names something that will not happen. That change must be created first — by reopening U3 with this route or by raising a new unit for it (a Units Generation / delivery decision, recorded as a release blocker in `infrastructure-specification.md`). `nfr-requirements/tech-stack-decisions.md` T1 supersedes `functional-spec.md` §8 and §12 items 1 and 5, which still call S2 "this unit's addition". The console release must not precede it |
| 12 | Deploy — `web` | `docker service update` of the `web` service | yes | Existing service, 2 replicas, rolling `parallelism: 1`, `order: start-first`, `failure_action: rollback`, `monitor: 30s`, gated by the `/healthz` check. No `infra/swarm-stack.yml` change and no `docker stack deploy` from this unit — the stack is unchanged |
| 13 | Rollback | automatic per service | yes | A failed `/healthz` after the `web` update triggers `failure_action: rollback` for that service only. **No migration is shipped, so there is no down-migration step to run** — the case `infra/swarm-stack.yml`'s `rollback_config` exists for. Rolling the console back also rolls the SPA back, because one image carries both; that coupling is stated in `infrastructure-specification.md` under Shared Infrastructure |
| 14 | Smoke | `scripts/smoke-identity.sh` and the `web` `/healthz` | partial | `/healthz` proves the static host is serving. `scripts/smoke-identity.sh` covers `/healthz`, `/readyz` and two `/internal/authorize` decisions — it **cannot cover S2**: the seeded smoke user is an ordinary user, not a support agent, so the script has no token that S2 would accept. A support-path smoke test does not exist |

## Gates this unit cannot satisfy yet

- **A smoke test for S2 does not exist** and is not invented here. Proving the read end to end needs
  a seeded support-role user plus a pending or active grant, which is a test-fixture change in
  U3's footprint. Until then the console's own suite proves the client half against MSW, and the
  live path is exercised by hand under a real support session.
- **No feature flag gates the release.** The product has a flag capability (BRD-010 FR-ADM-18) but
  nothing gate-tests a deployment here, and this unit adds none: the section replaces a dead chain,
  so a flag would toggle between "broken" and "working".
- **No secrets and no build-time secret.** `web/Dockerfile` takes `VITE_*` values, which are public
  client configuration by construction; the unit adds no variable and injects no credential.

## What must exist before this unit can ship

| Item | Owner | Why it blocks |
|---|---|---|
| `GET /v1/support-grants/mine` in the gateway `Route` table (not shadowed by a broader `/v1/support-grants` pattern — the table is first-match), Identity's registration and `list_by_support_agent` with `MY_GRANTS_LIMIT = 20`, the `/v1/support-grants/mine` arm in `route_label` (`services/identity/http/src/middleware.rs:140`), and a `(support_agent_id, requested_at DESC)` index as an up/down migration pair | **not yet assigned** — reopen U3 with this route, or raise a new unit for it | Without it the read is `404` and two of the section's flows are dead; no open change delivers it, so this is the release blocker (infrastructure-specification.md "What must exist") |
| Contract entries for the `mine` path (C1/C2 and C5, including `requestedBy` on the bare-array grant row) **and** a C1/C2 entry for the shipped `GET /v1/support-grants/{grantId}/metadata` (S3), which appears in no external contract surface | the S2 delivery, same change as the route | The console cannot map a frozen shape that is not written down; the mapper's `grantId ?? id` rule depends on it, and the new panel's projection is equally uncontracted |
| The doc edits this unit owns — `support-console/README.md:29,33`, `web/nginx.conf:7-9`, the comment at `services/gateway/infrastructure/src/upstream.rs:88-90` | **U9** | The retired `X-Support-Reason` header is still named in three places; leaving them makes the docs lie |
| A CI platform, and within it stages 10 (vulnerability scan) and a support-path smoke fixture | platform / U3 | Recorded as gaps; neither is this unit's to build, and neither is silently assumed |

Deploy order in one line: **the change that delivers S2 (route, handler, `route_label` arm, index
migration), then the `web` image** — both through the existing rolling update with the `/healthz`
gate, against production Swarm only (there is no staging tier). The first half does not exist yet.