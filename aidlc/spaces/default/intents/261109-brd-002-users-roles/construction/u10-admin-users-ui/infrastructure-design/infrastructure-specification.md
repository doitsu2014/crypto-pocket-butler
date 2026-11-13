# Infrastructure Specification — u10-admin-users-ui

**Upstream:** `nfr-design/performance-design.md`, `nfr-design/security-design.md`,
`nfr-design/logical-components.md`, `components.md` (`AdminUsersUI`), `functional-spec.md`
(§2 shell gating, §3 screens, §4 W1–W5, §8 routes), `contract-summary.md` (C1/C2, C3, C5, C7).
This unit's ui-kind NFR design produced no `scalability-design`, `reliability-design`, or
`observability-design` artifact — server-side scaling, reliability, and observability belong to
the services that own the server hops (gateway, Identity) under the measurement-ownership split in
`performance-design.md` §8 and the nfr-design diary; the platform facts those artifacts would have
carried are covered in this document and `monitoring-design.md` instead.

**Unit kind:** ui. The console section ships as static assets inside the existing `web` image; the
unit adds no service, no container, no schema, no secret and no stack change. It *does* touch
running code in two existing services (gateway route entries, Identity's admin lookup widening and
grant-list read), so this document records both the platform the feature runs on and the route
declarations that must exist elsewhere before it works.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute model | **No new container or service.** The section compiles into the existing support-console bundle; `web/Dockerfile` builds the customer SPA into `web/dist/` and then the console into `web/dist/support`, so one `web` image carries both. The section is one more lazy, hashed chunk under `/support/assets/` | The unit owns no deployable and no runtime process of its own; `AdminUsersSection` is `React.lazy` like `IncidentConsole` (logical-components §1.1) |
| Networking — ingress | Browser → TLS edge → Swarm published ports: `web` 80 (nginx :8080, 2 replicas) and `gateway` 443→8080 (2 replicas). Every U10 data call is a `/v1` path issued same-origin from `client.ts`; the console's CSP in `web/nginx.conf` is `connect-src 'self' https://*.supabase.co` | Existing edge topology, unchanged. `web/nginx.conf` serves `/support/` but **proxies no `/v1` path**, so a production edge in front of `web` must route `/v1` to the gateway; this repository does not define that production edge (security-design D-S1, dependency 7). The local stack's `deployment/localhost/nginx/default.conf` is the only file that both serves the console and proxies `/v1/` |
| Networking — egress | **No new browser-external origin.** The console's pre-existing Google Fonts load is unchanged; this unit adds no CDN data, analytics, telemetry or font, and the CSP is untouched | security-design §1 (NFR9.1) |
| Storage | None. The static host is `read_only: true` with tmpfs for nginx scratch paths; the looked-up record lives only in the in-memory query cache and is removed on unmount and on 401/403 | NFR10.1, NFR22.5; existing stack hardening |
| Environments | Production Swarm only; `deployment/localhost` compose for development (same `web/Dockerfile`, `VITE_GATEWAY_URL=/v1`) | Production is the only environment (team rule); the local stack is developer convenience, not a tier |
| IaC approach | `infra/swarm-stack.yml`: **no change from this unit.** The five route declarations are gateway *code* (the deny-by-default `Route` table), and the grant list read is Identity code — see "What must exist before this works" | logical-components §6 "Bridge to Infrastructure Design" |
| Resource sizing | `web` unchanged: 2 replicas × 0.25 CPU / 128 MB. `gateway` unchanged: 2 replicas × 0.5 CPU / 512 MB. `identity-api` unchanged: 3 replicas × 0.5 CPU, start-first. One admin-screen chunk and one extra provider read per lookup do not move any of them | Existing stack; the lookup's provider share is bounded at 3 s by Identity's own adapter (performance-design §5) |
| Migration | None. `0007_support_access` and the grant table are U3's; this unit adds a read method and requires a status index owned by U3 | logical-components §5; security-design §12 dep 4 |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| `web` nginx static host | static server / console edge | Existing image and service; 2 replicas, `read_only` + tmpfs, `/healthz` check (30 s interval), `stub_status` :8081 internal only, `index.html` `no-store`, hashed `/support/assets/` immutable with a 1-year cache, `stop_grace_period` 15 s | Serves the console at `/support/` and `/support/assets/` (lines 78–111); logs the request URL by default and sets no `access_log off` there |
| API Gateway | single entry for `/v1` | Existing; `UPSTREAM_TIMEOUT_MS: 2000`, `GATEWAY_RATE_LIMIT_PER_MINUTE: 120`, anonymous 30, deny-by-default route table, per-token `check_route` limiter (in-process per replica) | This unit declares four new `Route` entries, replaces the address-carrying U1 lookup, and sets a budget on the shipped role-change entry (security-design §3); the global figures are untouched |
| `identity-api` | admin lookup, role change, disable/restore, grant list | Existing; 3 replicas, `read_only`, `/healthz` 10 s check, `IDENTITY_API_POOL: 12`, internal network only (no published port) | The lookup gains one Supabase Auth provider read (`RoleReader::read_role`, connect 1 s / total 3 s, one attempt); the grant list adds one bounded read on Identity's own DSN |
| Supabase Postgres | Identity's schema | Existing; `identity_database_url` secret, one schema and one role per service, no cross-schema grants | This unit adds no table and no migration; it requires a status index on `support_grants` (U3) |
| Supabase Auth | third-party API | Existing; Identity reaches it with `identity_supabase_service_role` (JWKS for token validation, admin API for role read/write). The **console never calls it** for U10 — sign-in only, unchanged | The new provider read reuses the existing adapter and its timeouts; no new credential, no new origin |
| Docker secrets / Supabase Vault | secrets | **None required.** The console image's `VITE_*` build args are public client configuration, never secrets; the section adds no secret | web/Dockerfile comment; project secrets rule |
| Prometheus / nginx-prometheus-exporter | metrics | **Not declared in either stack.** `stub_status` :8081 is exposed for a sidecar that does not exist | Platform gap recorded in `monitoring-design.md` — the same gap u2 and u8 recorded |
| Message queue / cache / CDN / search | — | **None added.** Service-to-service is synchronous HTTP; the console caches in memory only | Project rule: no message queues |

## Shared Infrastructure

| Shared Resource | Owner Unit | Consumer Units | Access Boundary |
|---|---|---|---|
| `web` image + nginx host | platform (existing) | every SPA feature, u7, u8, u10, and the support console | Static files only; a U10 release redeploys the console bundle **and** the customer SPA (both are built in one Dockerfile) — identical to any `web` release today |
| Gateway `/v1` route table | gateway | u10 declares four new entries (`POST /v1/admin/users/lookup`, `POST …/disable`, `POST …/restore`, `GET /v1/support-grants`), **removes** the U1 `GET /v1/admin/users` declaration (`route.rs:130`, whose looked-up address travelled in the query string) together with its Identity `admin_lookup` registration, and **sets `max_requests_per_minute: 20` on the existing** `PUT …/role` entry (`route.rs:133`); every other U1–U8 declaration is unchanged | Deny-by-default; undeclared paths `404`; all five U10 paths are `requires_admin: true`; role change, disable and restore carry 20/min, the lookup and grant list 60/min (NFR11.1–11.2), per token and per gateway replica. The table is **first-match** (`RouteTable::match_route`) and the budget is read from that first entry (`AuthorizeAndRoute::route_budget`), so the PUT budget must be set on the shipped declaration — an appended second `PUT …/role` entry would be shadowed and its 20/min would never apply |
| Identity's `support_grants` table and repository | U3 (schema and store) | u10 (one list read), U3's own grant flow | Identity's own Postgres role and DSN; u10 adds `list_by_status` and no write, no rule, no migration |
| Supabase Auth admin API (role read/write) | Identity (U2's `RoleWriter`, u10's `RoleReader`) | u10 (read at lookup), U2 (write) | One adapter, one service-role secret, same timeouts; the browser never reaches it |
| Console query client + `client.ts` | shell | every console section | Additive functions and one optional `deadlineMs`; query keys namespaced `["admin"]`, removed on unmount |
| Edge access logs | platform | every route | nginx's default `combined` format logs the request URL **plus the referer and user-agent** — never a request body, and never a credential-bearing header such as `authorization` or `cookie`. D-S1 keeps the looked-up address out of the URL, the logged field that would otherwise carry it; the internal user UUID remains in the three mutation paths, recorded as a residual (security-design §1) |

## What must exist before this works (all owned elsewhere)

| Item | Where | Owner |
|---|---|---|
| `POST /v1/admin/users/lookup` route entry **replacing** the existing `GET /v1/admin/users` declaration and its Identity `admin_lookup` registration (both removed in the same change), plus Identity's admin DTO widening and `RoleReader::read_role` (provider read before the success audit, failure → `503`) | gateway `route.rs`; Identity `http/src/routes.rs`, `application`/`http`/`infrastructure` | gateway / U2 territory (security-design §12 deps 1–2) |
| `max_requests_per_minute: 20` set on the **existing** `PUT /v1/admin/users/{userId}/role` entry over the existing `change_role_for_gateway` | gateway `route.rs` | gateway / U2 |
| `POST /v1/admin/users/{userId}/disable` and `…/restore` entries + registration of the existing use cases on the JWT-authenticated `/v1` router | gateway `route.rs`; Identity `http/src/routes.rs` | gateway / U5 (security-design §12 dep 3) |
| `GET /v1/support-grants?status=pending_approval` handler + `SupportGrantRepository::list_by_status` | gateway `route.rs`; Identity `application`/`http` | gateway / U3 (security-design §12 deps 3–4) |
| `route_label` arms for the four `/v1/admin/users…` paths in `services/identity/http/src/middleware.rs` — the label map is a static match list, so without them the admin traffic is recorded under `route="other"` | Identity `http/src/middleware.rs` | this unit, in the Code Generation change (u7 precedent) |
| Status index on `support_grants` for the list read | U3's migration set | U3 (**required**, not conditional) |
| D-S1's contract ripple (functional-spec, frontend-components, contract-summary, C4, `docs/architecture/README.md`) **and the gateway `Route` entry it names** — the removal of `GET /v1/admin/users` is part of the ripple, not a separate change | record artifacts; gateway `route.rs` | this unit, in the Code Generation change |

Until those route entries land, the deny-by-default table answers `404` (and Identity `403` for a
non-admin) to every U10 call; the section renders its inline error states, so a premature console
deploy is **fail-safe but unusable**. The sequencing consequence is recorded in `cicd-pipeline.md`.
No `infra/swarm-stack.yml` change is required from this unit at any point.

## Why no new container

The unit is presentation code inside an existing build plus five route-table changes (four new
declarations, one replacement, one budget on a shipped entry) and one read in services that
already exist. A new service, cache tier or edge rule would add operational surface
with nothing to serve: the transport rides the existing gateway, the record is memory-only, and the
grant list is a bounded query on a table Identity already owns. What is *not* free is the list in
the table above — the gateway route-table changes, Identity's `route_label` arms and widening,
and U3's index — and this design
records them as the infrastructure-relevant change they are rather than folding them into "no
change".