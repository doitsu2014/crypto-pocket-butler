# CI/CD Pipeline — u10-admin-users-ui

**Upstream:** `infrastructure-specification.md`, `monitoring-design.md`,
`nfr-design/performance-design.md` (§6 `Retry-After`, §8 measurement), `nfr-design/
security-design.md` (§3 route budgets), `team.md` (Deployment, Testing Posture), `project.md`
(canonical commands).

## The honest starting point

The repository has **no CI platform** — no `.github/`, GitLab, Jenkins or CircleCI configuration.
The team chose to leave the platform open and record CI as an intention, so today these gates run
when a person runs them. This pipeline is the sequence a pipeline *would* run, and the unit's code
plan records the same commands as its local definition of done.

Unlike the UI-only units, this change touches **three build outputs from one commit**: the
`gateway` image (the deny-by-default table: four new entries, one removal, one budget on an
existing entry), the `identity-api` image (admin
DTO widening, `RoleReader`, disable/restore `/v1` registration, grant-list handler), and the `web`
image (the console section, built into `web/dist/support`). Each is gated below. The `web` image
also carries the customer SPA, whose source this unit does not change; its own size budget is
therefore unaffected, and the console has no size script to run (monitoring-design.md, metrics
table).

## Stages

| # | Stage | Command | Gate |
|---|---|---|---|
| 1 | Rust format | `cargo fmt --all -- --check` | Blocks: rustfmt drift |
| 2 | Rust lint | `cargo clippy --workspace --all-targets -- -D warnings` | Blocks: any warning |
| 3 | Boundaries | `bash scripts/check-boundaries.sh` | Blocks: any outward crate edge — the mechanical Clean Architecture gate the new `RoleReader` port and Identity DTO changes must respect |
| 4 | Rust tests | `cargo test --workspace` | Blocks: any failure, including the HTTP-boundary suites for the new admin handlers and the lookup's audit-ordering test (security-design §7) |
| 4b | Adapter and migration suites | `cargo test --workspace -- --ignored` with `TEST_DATABASE_URL` pointing at a **disposable Postgres** | Blocks: any failure (team rule — the `#[ignore]` suites are a blocking gate, not a developer opt-in). Until a CI platform exists, this is the same local command a developer runs |
| 5 | Console format | `yarn --cwd support-console format:check` | Blocks: Prettier drift |
| 6 | Console lint | `yarn --cwd support-console lint` | Blocks: any error (`@typescript-eslint/no-explicit-any` is an error) |
| 7 | Console type + build | `yarn --cwd support-console build` (`tsc -b && vite build`) | Blocks: type or build failure — the check that a `getSupportGrant`-style deletion would have failed |
| 8 | Console tests | `yarn --cwd support-console test` (Vitest + Testing Library + jest-axe) | Blocks: any failure — the deadline, one-attempt, no-refetch-on-focus, 429 `Retry-After`, 401/403 cache-clear and axe suites (performance-design §2–§9). `msw@^2.7.0` is **already** a devDependency, so the NFR design's handler choice needs no dependency change |
| 9 | Image builds | `docker build -f web/Dockerfile …`, `…/gateway/Dockerfile`, `…/identity/Dockerfile.api` (Identity ships `Dockerfile.api`, `Dockerfile.migrate`, `Dockerfile.worker` and no plain `Dockerfile`), each tagged `<registry>/<service>:<commit-sha>` | Blocks: any build failure. `VITE_*` args are public client configuration, never secrets |
| 10 | Vulnerability scan | dependency audit + Trivy on all three images | Blocks: high/critical findings (BRD-012 NFR-SEC-14). **Not yet configured** — specified, not wired; the same gap earlier unit pipelines recorded |
| 11 | Deploy — Identity first | Swarm rolling update of `identity-api`: `parallelism: 1`, `order: start-first`, `failure_action: rollback`, `monitor: 30s`; `/healthz` (10 s check) gates the swap | Blocks: a failing health check triggers the automatic rollback |
| 12 | Deploy — gateway, then console | Same rolling policy for `gateway` (published 443) and `web` (published 80) | Blocks: same automatic rollback |
| 13 | Rollback | `failure_action: rollback` restores the previous task set per service; the previous image tags are the manual path | Complete for this unit: no migration of its own, no persistent state |
| 14 | Smoke | `/healthz` 200 on each rolled service — the stated release-completion signal (team memory) | The repository ships `scripts/smoke-identity.sh` (`C4_SMOKE_TOKEN`, `SMOKE_USER_JWT`, `SMOKE_USER_ID`), which covers the C4 decision and the identity read with a **test user's** JWT. It cannot cover the admin routes as it stands: they need an admin-role token and no admin user is seeded in the stack's provisioning, so extending it needs that seed first — recorded as this unit's one open smoke item, not invented |

## Deployment ordering and rollback

- **Identity before gateway before console.** The gateway's deny-by-default table only *forwards*;
  if its entries land before Identity's handlers exist, the paths answer Identity's `404`/`405`
  rather than failing closed at the edge. The console ships last, so an admin never sees a section
  whose calls are not yet routable. Because each service rolls `start-first` independently, the
  intermediate window is safe in every order — old clients see no new routes (they do not call
  them) and new clients fail into their inline error states — but the stated order avoids a
  pointless error window.
- **Mid-roll console behaviour:** `index.html` is `no-store` and hashed chunks are immutable, so a
  client picks up the new shell on next load. A session already open can request a previous
  build's lazy section chunk after the old task is gone; the shell's existing `Suspense` boundary
  covers the first load, and a chunk-load failure stays inside the section body (logical-components
  §2) — the header, nav and banner remain. No new error boundary is specified, because the console
  routes by hash sections rather than routes (unlike `web`'s router).
- **Rollback trigger:** a failed post-update health check — the only stated trigger, implemented by
  `failure_action: rollback`. Re-deploying the previous `identity-api`, `gateway` and `web` tags is
  the manual path.
- **Schema:** this unit ships **no migration**. The status index on `support_grants` is U3's and
  moves with U3's `0007` up/down pair; if U3's release is rolled back, the index rolls back with
  it, and this unit's list read degrades in performance (bounded to 100 rows) rather than in
  correctness.
- **Feature flags:** none. The section is gated by the shell's `adminOnly` nav and, decisively, by
  the server: a `support` session gets `403` on its first call (security-design §3). A flag inside
  the bundle would add a second code path with no operational gain; the route entries themselves
  are the switch.

## Secrets in CI/CD

None from this unit. The console holds no secret; the `web` service declares none; the
`VITE_SUPABASE_URL`/anon-key build args are public client configuration (web/Dockerfile comment;
project secrets rule). Identity's provider access uses its existing
`identity_supabase_service_role` secret — unchanged, not re-provisioned. The pipeline needs
registry-push credentials only, the same credential every image stage uses.

## What must exist before this unit can ship

1. **Identity's admin surface** — the widened lookup with `RoleReader::read_role` ordered before
   the success audit, the `/v1` registrations of disable/restore, and
   `SupportGrantRepository::list_by_status` (security-design §12 deps 2–3).
2. **Gateway route-table changes** — four new declarations with their per-token budgets, the
   removal of the address-carrying `GET /v1/admin/users`, and the 20/min budget set on the
   existing `PUT …/role` entry (security-design §12 deps 1, 3).
3. **U3's status index** on `support_grants` (dep 4, **required**).
4. **D-S1's contract ripple** landed in the same change (dep 1): functional-spec, frontend-components,
   contract-summary, the C4 model and `docs/architecture/README.md`.
5. **Vulnerability scanning wired** into whichever platform the team chooses (stage 10).

Sequencing consequence: the console bundle may deploy *before* items 1–2 only if the nav entry is
not yet shipped (it fails safe either way); the functional release of the admin screen ships
**after** items 1–2 and should not precede item 3 in the same environment. No
`infra/swarm-stack.yml` change is required from this unit at any point.