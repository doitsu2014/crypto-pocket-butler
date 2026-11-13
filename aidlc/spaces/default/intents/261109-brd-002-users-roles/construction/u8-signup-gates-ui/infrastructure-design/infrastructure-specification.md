# Infrastructure Specification — u8-signup-gates-ui

**Upstream:** `performance-design.md`, `security-design.md`, `logical-components.md`,
`components.md` (`SignupGatesUI`), `functional-spec.md` (revision 2), `contract-summary.md`
(C1/C2, C3, C6, C8). This unit's ui-kind NFR design produced no `scalability-design`,
`reliability-design`, or `observability-design` artifact — server-side scaling, reliability, and
observability belong to the services that own the server hops (gateway, U1/U4/U6) under the
measurement-ownership split recorded in `performance-design.md` and the nfr-design diary; the
client-side platform facts those artifacts would have carried (static-host sizing, rolling
updates, collection gaps) are covered in this document and `monitoring-design.md` instead.

**Unit kind:** ui. The unit ships as static assets inside the existing `web` image; it adds no
service, no container, no schema, and no stack change. This document records the infrastructure
the feature *runs on*, the one shared artifact it redeploys, and the route declarations that
must exist elsewhere before it works.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute model | **No new container or service.** The gate feature compiles into the existing SPA bundle and ships in the existing `web` image (`web/Dockerfile`: node:24-alpine builder → `nginxinc/nginx-unprivileged:1.27-alpine`, serving the customer SPA at `/` and the support console at `/support/`). The gate route chunk is one more hashed file under `/assets/` | The unit owns no deployable and no runtime process of its own |
| Networking — ingress | Browser → TLS edge → Swarm published ports: `web` 80 (nginx :8080, 2 replicas), `gateway` 443→8080 (2 replicas). The SPA calls `/v1` **same-origin** — `web/nginx.conf` deliberately carries no `/v1` proxy (the localhost stack's `nginx/default.conf` does), and the CSP `connect-src 'self'` requires the gateway to be same-origin through the CDN host | Existing edge topology, unchanged by this unit; the nginx.conf comment names the constraint any deploy config must respect |
| Networking — egress | **No new browser-external origin.** Country data and locale catalogues are bundled; terms/risk documents are link navigations, never fetches; the CSP in `web/nginx.conf` is unchanged and is the mechanical backstop | security-design §3; NFR9.1 |
| Storage | None. The static host is `read_only: true` with tmpfs for nginx scratch paths; the feature stores nothing client-side (memory-only cache and key holder) | NFR10.1; existing stack hardening |
| Environments | Production Swarm only; `deployment/localhost` compose for development (web built from the same Dockerfile, `VITE_GATEWAY_URL=/v1`) | Production is the only environment (team rule) |
| IaC approach | `infra/swarm-stack.yml`: **no change from this unit.** The gateway route-table entries the feature calls are gateway *code* (deny-by-default table), owned by the gateway/U1 units — see "What must exist before this works" | logical-components.md "Bridge to Infrastructure Design" |
| Resource sizing | `web` service unchanged: 2 replicas × 0.25 CPU / 128 MB. A 10–20 kB gzip route chunk does not move the sizing of a static host | Existing stack; the estimate's first real measurement is `yarn size` in Build and Test |
| Migration | None | No schema change |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| `web` nginx static host | static server / edge | Existing image and service; `/healthz` check (30 s interval), `stub_status` on :8081 internal only, `index.html` `no-store`, hashed `/assets/` immutable with 1-year cache | One image carries **both** the SPA and the support console (shared artifact, below) |
| API Gateway | single entry for `/v1` | Existing; `UPSTREAM_TIMEOUT_MS: 2000`, 120 req/min authenticated and 30 req/min anonymous rate limits, deny-by-default route table | The table today declares assets + identity routes; **no signup-gate route and no telemetry route exist yet** |
| Supabase Auth | third-party API | Existing shell sign-in. The CSP already permits two direct browser-external origins besides the gateway — Supabase Auth and the WalletConnect relay/verify hosts — and the gate feature contacts **neither**: its only runtime origin is the same-origin gateway | Both existing origins unchanged by this unit; no CSP change |
| Docker secrets / Supabase Vault | secrets | **None required.** The feature holds no secret; the `VITE_*` build args baked into the image are public client configuration, never secrets | web/Dockerfile comment; project secrets rule |
| Prometheus / nginx-prometheus-exporter | metrics | **Not declared in either stack.** `stub_status` :8081 is exposed for a future exporter sidecar that does not exist | Platform gap, recorded in `monitoring-design.md` — same gap u2 recorded |

## Shared Infrastructure

| Shared Resource | Owner Unit | Consumer Units | Access Boundary |
|---|---|---|---|
| `web` image + nginx host | platform (existing) | every SPA feature, u7, u8, and the support console | Static files only; feature isolation is the route chunk + the feature folder; a u8 release redeploys the console bundle too (both are built in one Dockerfile) — identical to any web release today |
| Gateway `/v1` route table | gateway | u8 (G1 read-only, G2, G3), u7, U1–U6 routes | Deny-by-default; additive per provider unit; undeclared paths 404 |
| `web/src/api/client.ts` + query client | shell | every SPA feature | Additive methods only; gate query keys namespaced `["signup-gates", <sessionUserId>, …]` (nfr-design) |
| i18n catalogues (`web/src/i18n`) | shell | u7, u8 | Additive `gates.*` namespace; `resolveLocale` untouched |
| Edge access logs (nginx) | platform | every route | Default format logs the URL, never request headers (nginx.conf comment); self-scoped gate routes keep URLs free of identifiers (NFR22.3) |

## What must exist before this works (all owned elsewhere)

| Item | Where | Owner |
|---|---|---|
| Read-only, non-provisioning, self-scoped principal/gate-state entry reads with a defined "no record" answer | gateway route table + Identity handler | gateway / U1 / U4 (security-design §11 dep 1) |
| `POST /v1/signup/gates` (G3) — new authenticated route carrying the four-field body, `Idempotency-Key`, and the C8 → C3 → C6 orchestration | gateway route table + Identity | gateway / U1 (dep 5); the existing unauthenticated pre-signup gateToken route stays as it is |
| G3/C6 skip-already-recorded semantics | Identity consent recording | U4 / U1 (dep 2) |
| `jurisdictionBlocked` source | Identity projection | U6 (dep 3) |
| Gateway IP derivation from the trusted proxy chain | gateway middleware | gateway (dep 4) |

Until the route entries land, the deny-by-default table 404s every gate call: entry resolution
fails safe into the full-page retry surface (never into the app), so a premature SPA deploy is
safe but unusable — the sequencing requirement is recorded in `cicd-pipeline.md`.

## Why no new container

The unit is presentation code inside an existing build. A new service, cache tier, or edge rule
would add operational surface with nothing to serve: the feature's transport rides the existing
gateway, its data modules are bundled at build time, and its state is memory-only. What is *not*
free is the route-declaration list above — five entries owned by the gateway and Identity units —
and this design records them as the infrastructure-relevant change they are, rather than folding
them into "no change".
