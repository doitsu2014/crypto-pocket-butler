# Infrastructure Specification — u9-support-console-ui

**Upstream:** `nfr-design/performance-design.md`, `nfr-design/security-design.md`,
`nfr-design/logical-components.md`, `components.md` (`SupportConsoleUI`), `functional-spec.md`
(§2 journeys, §3 screens, §8 S1–S3, §10 errors, §12 open items), `contract-summary.md` (C1/C2,
C5, C9).
This unit's ui-kind NFR design produced no `scalability-design`, `reliability-design`, or
`observability-design` artifact — server-side scaling, reliability and observability belong to the
services that own the server hops (gateway, Identity) under the measurement-ownership split in
`performance-design.md` §8; the platform facts those artifacts would have carried are covered here
and in `monitoring-design.md`.

**Unit kind:** ui. The section ships as static assets inside the existing `web` image and this unit
provisions **no new service, container, schema, secret, volume or stack change**. It takes feature
code out of the console (three components reworked, one added, five retired) and changes two files
outside that package, **both comment-only**: `web/nginx.conf:7-9` and
`services/gateway/infrastructure/src/upstream.rs:88-90`, each of which still names the retired
`X-Support-Reason` header. Its one server-side dependency, `GET /v1/support-grants/mine` (S2), is
**U3's route and method**, not this unit's — recorded below rather than folded into "no change",
and flagged there as undelivered: U3's work has landed without it.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute model | **No new container or service.** The section compiles into the existing support-console bundle; `web/Dockerfile` builds the customer SPA into `web/dist/` and the console into `web/dist/support`, so one `web` image carries both. The section **shrinks**: `AccountSearch`, `AccountMetadataPanel` and `useSupportGrant` are removed | The unit owns no deployable. It is presentation code replacing a dead route chain (logical-components §1.1) |
| Networking — ingress | Browser → TLS edge → Swarm published ports: `web` 80 (nginx :8080, 2 replicas) and `gateway` 443→8080 (2 replicas). Every U9 data call is a `/v1` path issued same-origin from `client.ts`; the console CSP in `web/nginx.conf` is `connect-src 'self' https://*.supabase.co` and is unchanged | Existing edge topology. `web/nginx.conf` serves `/support/` (lines 84–106) but **proxies no `/v1` path**; only `deployment/localhost/nginx/default.conf` both serves the console and proxies `/v1/`. The production `/v1` edge is still undefined in this repository — it is the u10 open item, unchanged by this unit |
| Networking — egress | **No new browser-external origin.** The console's pre-existing Google Fonts load (`support-console/index.html:12-15`) is unchanged; this unit adds no CDN data, analytics, telemetry or font, and the CSP is untouched | security-design §1 (NFR9.1) |
| Storage | None. The static host is `read_only: true` with tmpfs for nginx scratch; the grant and the projection live only in the in-memory query cache and are dropped on access end | NFR10.1, NFR22.5; existing stack hardening |
| Environments | Production Swarm only; `deployment/localhost` compose for development (same `web/Dockerfile`, `VITE_GATEWAY_URL=/v1`) | Production is the only environment (team rule); the local stack is developer convenience, not a tier |
| IaC approach | `infra/swarm-stack.yml`: **no change from this unit.** The only route this unit consumes (S2) is a gateway *code* entry owned by U3; the unit itself adds no declaration | logical-components §1.3, §6 "Bridge to Infrastructure Design" |
| Resource sizing | `web` unchanged: 2 replicas × 0.25 CPU / 128 MB. `gateway` unchanged: 2 replicas × 0.5 CPU / 512 MB. `identity-api` unchanged: 3 replicas × 0.5 CPU / 512 MB, start-first. A shrinking console chunk and one bounded 20-row read do not move any of them | `infra/swarm-stack.yml`; the read is a single indexed query on Identity's own DSN |
| Migration | **None from this unit**, but the read is not indexed as first written. S2 reads the existing `support_access_grants` table under Identity's own schema and role and adds no column, table or index for U9 itself. The only index leading on `support_agent_id` is the **partial unique** `support_grants_live_pair_idx (support_agent_id, user_id) WHERE status IN ('pending_approval', 'active')` (`services/identity/migrations/0007_support_access.up.sql:45-46`), which a status-free `WHERE support_agent_id = $1 ORDER BY requested_at DESC, id LIMIT 20` cannot use — so without a `(support_agent_id, requested_at DESC)` index that read scans the table. The index ships with the S2 delivery as a versioned up/down migration pair (see "What must exist"); it is not this unit's migration | security-design §11 item 1; logical-components §5 |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| `web` nginx static host | static server / console edge | Existing image and service; 2 replicas, `read_only` + tmpfs, `/healthz` check, `stub_status` :8081 internal only, `index.html` `no-store`, hashed `/support/assets/` immutable with a 1-year cache, `stop_grace_period` 15 s | Serves the console at `/support/` and `/support/assets/`; logs the request URL by default and sets no `access_log off` there. This unit edits the log-format **comment** at lines 7–9 (it still names the retired `X-Support-Reason` header); the format itself is unchanged |
| API Gateway | single entry for `/v1` | Existing; `UPSTREAM_TIMEOUT_MS: 2000`, `GATEWAY_RATE_LIMIT_PER_MINUTE: 120`, anonymous 30, deny-by-default route table, per-token `check_route` limiter (in-process, per replica) | U9 declares **no route**: S1 (`POST /v1/support-grants`, 20/min) and S3 (`GET /v1/support-grants/{grantId}/metadata`, 60/min) already exist at `route.rs:154` and `:158-160`. S2's entry is U3's (see "What must exist"). The unit edits one **comment** in `infrastructure/src/upstream.rs:88-90` |
| `identity-api` | grant create, own-grant read (S2, U3), metadata read (S3) | Existing; 3 replicas, `read_only`, `/healthz` 10 s check, `IDENTITY_API_POOL: 12`, internal network only (no published port), `ACCOUNT_URL`/`PORTFOLIO_URL`/`AUDIT_URL` already set for the metadata fan-out | U9 changes no Identity code. S2 is U3's handler and repository method; S3 already exists and fans out to Account and Portfolio within its own 500 ms budget |
| Supabase Postgres | Identity's schema | Existing; one schema and one role per service, no cross-schema grants | No table change and no migration from this unit. S2 is a `SELECT … FROM support_access_grants WHERE support_agent_id = $1 ORDER BY requested_at DESC, id LIMIT $2` on that existing table, executed under Identity's role. It needs a `(support_agent_id, requested_at DESC)` index to avoid a table scan — the shipped `support_grants_read_idx` leads on `user_id` and `support_grants_live_pair_idx` is partial — and that index is the S2 delivery's migration pair (Deployment, "Migration") |
| Supabase Auth | sign-in and JWKS | Existing; JWKS validation on every service route; the browser calls it only for sign-in | Unchanged. U9 adds no auth call and no refresh loop (NFR2.6) |
| Docker secrets / Supabase Vault | secrets | **None required.** The console image's `VITE_*` build args are public client configuration, never secrets; the section adds no secret and no environment variable | web/Dockerfile comment; project secrets rule |
| Prometheus / nginx-prometheus-exporter | metrics | **Not declared in either stack.** `stub_status` :8081 is exposed for a sidecar that does not exist | Platform gap recorded in `monitoring-design.md` — the same gap u2, u8 and u10 recorded |
| Message queue / cache / CDN / search | — | **None added.** Service-to-service is synchronous HTTP; the console caches in memory only | Project rule: no message queues |

## Shared Infrastructure

| Shared Resource | Owner Unit | Consumer Units | Access Boundary |
|---|---|---|---|
| `web` image + nginx host | platform (existing) | every SPA feature, u7, u8, u9, u10 | Static files only; a U9 release redeploys the console bundle **and** the customer SPA (one Dockerfile, one image). The console chunk shrinks; the SPA's own assets are unchanged |
| Gateway `/v1` route table | gateway | u9 consumes S1 and S3 unchanged and **consumes S2, which U3 declares** (`GET /v1/support-grants/mine`, no per-route budget — it rides the shared per-caller-address budget, `requires_admin: false`) | Deny-by-default; an undeclared path is `404` before a service is reached. The table is **first-match**, so S2's declaration must not be shadowed by a broader `/v1/support-grants` pattern that matches first — U3 places it accordingly. The gateway has no role capability — a `Route` is `method`, `pattern`, `service`, `requires_admin` and `max_requests_per_minute`, nothing more (`services/gateway/domain/src/route.rs:52-64`) — so the support-role refusal is Identity's on every path |
| Identity's `support_grants` table and repository | U3 (schema and store) | u9 (S2 read only, through U3's route), u10 (pending list), U3's own grant flow | Identity's own Postgres role and DSN; U9 adds no query of its own — it calls U3's route |
| Console query client + `client.ts` | shell | every console section | Two additive functions (`listMyGrants`, `fetchSupportMetadata`) and one mapper rule (`grantId ?? id`); three client functions and one header retired. Query keys namespaced `["support", …]`; the scoped clear matches that prefix only |
| Edge access logs | platform | every route | nginx's default `combined` format logs the request URL plus referer and user-agent — never a request body, and never a credential-bearing header such as `authorization` or `cookie`. S1 carries the reason in its **body** and S2's path is static, so neither puts case data in the logged URL. S3 carries the **grant UUID** in its path (`/v1/support-grants/{grantId}/metadata`) — an opaque identifier, already in U3's audit record and not personal data; recorded as an accepted residual here rather than quietly left out |

## What must exist before this works (owned elsewhere)

| Item | Where | Owner |
|---|---|---|
| `GET /v1/support-grants/mine` — gateway `Route` entry (no per-route budget), Identity registration, `SupportGrantRepository::list_by_support_agent(&UserId, u32)` with `MY_GRANTS_LIMIT = 20`, a bare `Vec<PendingGrantResponse>` answer, and the `/v1/support-grants/mine` arm in Identity's `route_label` (`services/identity/http/src/middleware.rs:140` — without it S2 is counted as a revoke) | gateway `route.rs`; Identity `application`/`infrastructure`/`http` | **No open change delivers it, and that is the blocker.** U3's work has landed without this route, so calling it "U3's release" was wrong: nothing in the tree or in any pending unit will add the path. Delivering it needs one of two decisions — **reopen U3** with this route, or **raise a new unit** for it; that choice belongs to Units Generation / delivery planning, and this design records it as a release-blocking decision rather than folding server code into a ui unit. `nfr-requirements/tech-stack-decisions.md` T1 supersedes `functional-spec.md` §8 and §12 items 1 and 5, which still call S2 "this unit's addition"
| A `(support_agent_id, requested_at DESC)` index on `support_access_grants`, as a versioned `<NNNN>_<name>.{up,down}.sql` pair | `services/identity/migrations/` | the S2 delivery, in the same change as the route — the shipped indexes cannot serve a status-free, recency-ordered read (Deployment, "Migration") |
| Contract entries for the `mine` path — `contract-summary.md` C1/C2 and C5, including `requestedBy` on the bare-array grant row — **and** a C1/C2 entry for the already-shipped `GET /v1/support-grants/{grantId}/metadata` (S3), which exists at `services/gateway/domain/src/route.rs:158-162` and `services/identity/http/src/routes.rs:70` but appears in no external contract surface, so the projection the new panel maps is as unwritten as the `mine` shape | record artifacts | the S2 delivery, in the same change as the route |
| Documentation edits this unit owns — `support-console/README.md:29,33`, `web/nginx.conf:7-9`, and the comment at `services/gateway/infrastructure/src/upstream.rs:88-90` | console package; `web`; gateway | **U9**, in the Code Generation change (comment-only outside the package) |
| A console-wide localisation unit for NFR26.2 (deferred, not met) | `unit-of-work.md` | tracked as decision `ab46b27b`; no owner unit exists yet |

Until S2 lands, the console's reload recovery and its **Check status** control have no server half,
and the deny-by-default table answers `404` to that path; the section's other flows (create, banner
from the `201`, metadata read under a live grant) are unaffected. Because no change currently
delivers S2, that wait has no end date: the route is a **release blocker** for this unit's Code
Generation, not a sequencing preference, and the deploy ordering it imposes is recorded in
`cicd-pipeline.md`. **No `infra/swarm-stack.yml` change is required from this unit at any point**,
and this unit ships no migration — the index pair above belongs to the change that delivers S2.

## Why no new container

The unit is presentation code inside an existing build: it reworks three components, adds one,
retires a component chain that calls routes that do not exist, and makes one bounded read through a
route U3 owns. A new service, cache tier or edge rule would add operational surface with nothing to
serve — the transport rides the existing gateway, the grant and projection are memory-only, and the
read is a single indexed query on a table Identity already owns. What is *not* free is the
dependency in the table above: S2's route, method and contract are real infrastructure-relevant
work, and this design records them as such while stating plainly that they are another unit's to
deliver.