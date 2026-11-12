# Infrastructure Specification — u7-settings-ui

**Upstream:** `nfr-design/performance-design.md`, `nfr-design/security-design.md`,
`nfr-design/logical-components.md`, `inception/domain-design/components.md` (the `Identity`
container and its settings surface), `functional-spec.md` (§2 routing, §3 screens, §8 API
integration points), `contract-summary.md` (C1/C2, C3).
This unit's ui-kind NFR design produced no `scalability-design`, `reliability-design`, or
`observability-design` artifact — server-side scaling, reliability and observability belong to the
services that own the hops — so the platform facts those artifacts would have carried are recorded
here and in `monitoring-design.md`.

**Unit kind:** ui **with server additions**. Unlike a pure frontend unit, this one changes three
existing deployables: the SPA bundle (`web`), the Identity API (`identity-api`, six routes), and
the API Gateway (`gateway`, route-table entries). It adds **no new service, no container, no
schema, no migration, and no stack change** — the three images already exist and already run.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute model | **No new container or service.** Three existing images are rebuilt: `web` (nginx static host, the settings and account-status screens), `identity-api` (the D0/P1/P2/D1/D2 handlers), `gateway` (the route-table entries). The settings route chunk is one more hashed file under `/assets/` | Each addition lives in a process that already runs; nothing new needs a scheduler, a port, or a scaling policy |
| Networking — ingress | Browser → TLS edge → Swarm: `web` 80 (nginx :8080, 2 replicas), `gateway` 443→8080 (2 replicas), `identity-api` internal only (:8083, 3 replicas, no published port). The SPA calls `/v1` same-origin — `web/nginx.conf` carries no `/v1` proxy and the CSP's `connect-src 'self'` requires the gateway behind the same origin | Existing edge topology, unchanged; security-design §4 adds no origin and no CSP entry |
| Networking — egress | **No new browser-external origin** (security-design §4). Server-side, `identity-api` already reaches Supabase Auth/Storage and its sibling services on the internal network; the new handlers use the existing repository and use-case paths and call nothing new | NFR9.1 |
| Storage | No client-side storage (NFR10.1): the query cache is memory-only, preferences persist server-side. `web` and `identity-api` are both `read_only: true` with tmpfs scratch paths; the export job's object storage is the existing `identity-exports` bucket | Existing stack hardening; no new mount, no new bucket |
| Environments | Production Swarm only; `deployment/localhost` compose for development, where the same three images run on one machine | Production is the only environment (team rule) |
| IaC approach | `infra/swarm-stack.yml`: **no change from this unit.** Everything this unit ships is application code — the six routes in Identity's router, the entries in the gateway's deny-by-default table, the SPA bundle. No new secret, no new network, no new service, no new volume | logical-components.md "Bridge to Infrastructure Design" |
| Resource sizing | Unchanged: `web` 2 × 0.25 CPU / 128 MB, `identity-api` 3 × 0.5 CPU / 512 MB, `gateway` 2 × 0.5 CPU / 512 MB. The added load is four personal-settings reads and writes per signed-in visit — orders of magnitude below what those limits already carry | Existing stack; the deletion trigger is rate-limited to 5/min per token (security-design §10), so it cannot become a spike source. **Per-replica caveat:** the per-route bucket is an in-memory fixed window inside each gateway process (`services/gateway/http/src/rate_limit.rs`) and `gateway` runs 2 replicas, so the effective ceiling is up to two windows per token and a task restart resets the window — a bound on abuse, not an exact quota |
| Database | **No migration.** D0/P1/D2 read and write tables `users`, `user_preferences`, `deletion_requests` and the consent tables that already exist; the only new SQL is in the existing repositories' existing statements plus the projections the P1/D1 reads select | The unit changes no schema and no role, so NFR23 stays the service units' (traceability records it N/A) |
| Release order | `identity-api` (and `gateway`) must serve the new routes **before** the `web` bundle that calls them is live. Within one release the stack updates each service independently; a `web` task that reaches a gateway without the entries gets a 404, which the entry surface renders as the full-page retry — fail-safe, not corrupting | Sequencing rule; `cicd-pipeline.md` states it as a stage order |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| `web` nginx static host | static server / edge | Existing image and service; `/healthz` (30 s interval), `read_only` + tmpfs, `index.html` `no-store`, hashed `/assets/` immutable for a year | One image carries the SPA **and** the support console (shared artifact below) |
| API Gateway | single entry for `/v1` | Existing service: `UPSTREAM_TIMEOUT_MS: 2000`, 120 req/min authenticated and 30 req/min anonymous default limits, deny-by-default route table, `Gateway_RATE_LIMIT_PER_MINUTE` untouched | **The table declares none of the settings routes today** — the six entries and the 5/min budget on D3 are this unit's change to the gateway's code, not its stack |
| `identity-api` | settings reads and writes | Existing: 3 replicas, `IDENTITY_API_POOL: 12`, JWT validation against Supabase JWKS, `SERVICE_TOKENS_FILE` for internal callers, `/metrics` | The new routes ride the existing router, pool and middleware; the deletion and profile use cases already exist |
| Supabase Postgres | primary store | Existing; one schema and one role for Identity (`identity_app`), append-only audit through its own role | No new table, no new grant, no migration |
| Supabase Auth | identity provider | Existing shell sign-in; `identity-api` validates tokens against the JWKS | Unchanged; the SPA contacts it only for sign-in (NFR9.1) |
| Docker secrets / Supabase Vault | secrets | **None required by this unit.** No new credential: D2/D3 need only the caller's JWT, and the export control uses the existing Identity token service | Project secrets rule; security-design §3 |
| Prometheus / exporters | metrics | **Not declared in either stack** — each service exposes `/metrics` and no Prometheus is deployed; `infra/prometheus/` holds alert rules but no scrape config runs | Platform gap, recorded in `monitoring-design.md`; the same gap the other unit records |

## Shared Infrastructure

| Shared Resource | Owner Unit | Consumer Units | Access Boundary |
|---|---|---|---|
| `web` image + nginx host | platform (existing) | every SPA feature, u7, u8, support console | Static files only; a u7 release redeploys the console bundle too, because one Dockerfile builds both |
| `identity-api` image | platform / Identity units | u1, u3, u7 and every service that reads Identity over C3 | **Additive routes and handlers only**; `UserStatus::denies_use` and `PrincipalService` are untouched (security-design §2), so no existing caller's behaviour changes |
| Gateway `/v1` route table | gateway | u7 (six entries), u8, U1–U6, the support routes | Deny-by-default; additive per provider unit; undeclared paths 404 |
| `web/src/api/client.ts` + query client | shell | every SPA feature | Additive methods plus the removal of calls no shell screen uses; keys namespaced `["me", …]`; retry policy not overridden (T6) |
| `ConfirmDialog`, `StatusPill` (`web/src/components/`) | shell | other features, U8/U9/U10 | `ConfirmDialog` gains focus-trap behaviour additively; `StatusPill` is presentational |
| i18n catalogues (`web/src/i18n`) | shell | u7, u8, U9/U10 | Additive `settings.*` namespace; `resolveLocale` untouched |
| Edge access logs (nginx) | platform | every route | Default format logs the URL and never request headers; the self-scoped `/v1/me/...` shape keeps user identifiers out of those URLs (security-design §5) |

## What must exist before this works

| Item | Where | Owner | State |
|---|---|---|---|
| D0 `GET /v1/me/account-status`, P1/P2 `/v1/me/profile`, D1/D2 `/v1/me/deletion-request` — self-scoped, caller from the stored record | Identity router + handlers | **this unit** | Code this unit writes |
| Gateway entries for the six settings routes — D0 `GET /v1/me/account-status`, P1/P2 `/v1/me/profile`, D1/D2 `/v1/me/deletion-request`, D3 `POST /v1/privacy/deletions` (5/min); five paths, six declarations | gateway route table | **this unit** | Code this unit writes; the paths are absent from the table today |
| Gateway entries for `POST /v1/exports` and `GET /v1/exports/{exportId}` | gateway route table | **this unit** (the privacy panel's export control needs them) | Absent today; Identity already serves both |
| Route labels for the six settings routes in `services/identity/http/src/middleware.rs` `route_label` | identity-api | **this unit** (same crate as the handlers) | `route_label` is a static match list; without an arm, every new route is counted as `route="other"` and the settings SLIs below cannot be computed |
| D1's no-record `404` carrying `https://crypto-pocket-butler.example/problems/no-record` | Identity problem rendering | **this unit**, against the **contract** | The pinned type is a Contract Design addition (security-design §14.4); the client treats any other `404` as an error |
| Contract summary rows for the six routes and the two export entries | `contract-summary.md` C1/C2 | Contract Design (outside this unit) | Recorded dependency; the functional spec carries the delta |

Five of those six rows are this unit's own code, so the sequencing risk is lower than u8's: the
only external prerequisite is the contract summary and the sentinel's agreement, and a miss there
shows up as an error surface rather than a silent wrong state.

**Eight gateway declarations, not nine.** The unit needs exactly the six settings declarations above
plus the two export entries — the list functional-spec §8 records. `GET /v1/privacy/deletions/{deletionId}`
is **not** among them: Identity already serves it and it resolves through the stored-record lookup,
but no screen in this unit calls it (the delete and restore screens read D1, and the privacy panel
polls `GET /v1/exports/{exportId}`), so it is not a declaration this unit adds.

## Why no new container

The feature is presentation code plus routes on services that already run. A new service, cache
tier or edge rule would add operational surface with nothing to serve: the settings data lives in
Identity's existing schema, the reads are single-row lookups behind an existing pool, and the
client holds nothing but a memory-only cache. What is *not* free is the gateway entry list above —
eight declarations in a deny-by-default table — and this design records them as the
infrastructure-relevant change they are rather than folding them into "no change".