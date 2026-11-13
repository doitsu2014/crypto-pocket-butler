# Monitoring Design — u8-signup-gates-ui

**Upstream:** `security-design.md` (§5 telemetry decision, NFR22.2), `performance-design.md`
(§1 budget enforcement, §9 verification), `infrastructure-specification.md`, nfr-design
measurement-ownership split.

## The collection gap, stated first

Three honest facts frame everything below:

1. **The feature emits zero telemetry** — a design decision, not an omission (security-design
   §5, NFR22.2). There is therefore no client-side metric this unit could collect. The only
   browser-emitted signal that can mention the gate route is the shell's existing render-crash
   beacon (`web/src/telemetry.ts` → `POST /v1/telemetry/client-errors`, route/errorCode/
   correlationId/sessionId, no gate data). **That endpoint is not declared in the gateway's
   deny-by-default route table**, so today the beacon 404s and is dropped silently (it is
   best-effort by design). The CSP `report-to` endpoint (`/v1/telemetry/csp-reports`) is
   undeclared the same way. Both are gateway telemetry work, recorded here as platform gaps
   this unit neither fixes nor hides.
2. **No Prometheus exists in either stack.** The `web` image exposes nginx `stub_status` on
   :8081 (internal only) for an nginx-prometheus-exporter sidecar that is not declared — the
   same gap u2's monitoring design recorded. Every edge alert below is defined against a
   collection path that does not run yet.
3. **Server hops are measured by their owners.** G1/G2/G3 latency and error rates are gateway
   and Identity metrics (gateway, U1, U4, U6 records). This unit adds no server-side metric
   and duplicates none.

## Metrics & KPIs

| Metric | Source | Threshold | Why it matters |
|---|---|---|---|
| Route-chunk size (gzip) | **Build-time**: `yarn size` — size-limit `route-*.js` glob + `web/scripts/check-size.mjs` | ≤ 150 kB route chunk, ≤ 300 kB initial entry | The mechanical NFR2.1 gate; a build metric, not runtime — no config change needed (`*Page` naming) |
| nginx connections/requests | `web` `stub_status` :8081 → future exporter sidecar | — (trend) | Static-host health; existing source, uncollected |
| Gateway request 5xx share on G2/G3 routes | Gateway `/metrics` (route label owned by the gateway unit) | per the gateway unit's alerts | Server-side error rate of the gate calls |
| G1/G2/G3 latency (p95) | Gateway `/metrics` + Identity `/metrics` | budgets in the server units' NFR records | The entry < 2 s and submit < 2 s budgets (NFR2.2/2.3) are only measurable server-side — client-perceived latency is not collected (fact 1) |
| Render-crash beacons for the gate route | Shell beacon → gateway telemetry route (**once declared**) | any sustained rise | The only runtime signal that can name the route; dropped at the gateway 404 until the platform gap closes |

## Alerts

| Alert | Condition | Severity | Routes to |
|---|---|---|---|
| Route-chunk budget exceeded | `yarn size` fails | **Build-blocking** (pipeline stage, not a runtime alert) | the changing developer |
| G3 5xx / latency | gateway-side conditions owned by the gateway and U1 units | per those units' alert tables | gateway / Identity on-call |
| Static-host down | `web` health check failure during a rolling update | Swarm `failure_action: rollback` acts automatically; the platform's edge alerting (when Prometheus lands) pages | platform on-call |

There are **no unit-owned runtime alerts**, and the table says so instead of inventing them:
with zero feature telemetry (fact 1) and no collection stack (fact 2), nothing exists to arm one
against, and an alert nobody can act on is the thing the team's own rule deletes.

## SLIs / SLOs

| SLI | SLO target | Measurement window | Measured where |
|---|---|---|---|
| Entry resolution latency (server side of NFR2.2's p95 < 2 s) | p95 < 2 s | 30-day rolling | Gateway metrics on G1/G2 (gateway unit) |
| Gate submission success (allowed outcomes / non-5xx attempts) | per the platform 99.9% monthly availability floor (BRD-012 NFR-AVL-01) | 30-day rolling | Gateway metrics on G3 (gateway/U1 units) |
| Static asset availability | health check passing; failed updates roll back automatically | continuous | Swarm + `web` `/healthz` |
| Client-perceived entry latency | **Not collected** — recorded as a gap, not budgeted a number | — | would need the telemetry route (fact 1) |

## Logs & tracing

- **Edge access logs:** nginx's default format logs the URL and never request headers (the
  `web/nginx.conf` comment forbids adding `$http_*` to a log format). The gate routes are
  designed self-scoped (`/v1/me/...` shape, security-design §5), so gate URLs in edge logs
  carry no subject, hash, id, or email (NFR22.3) — the log format needs no change to stay
  clean, and the route shape is what keeps it that way. **Qualified by dependency 1:** today's
  G1 shape (`GET /v1/principals/{authUserId}`) does put an auth user id in the logged URL; the
  identifier-free property holds only once the read-only, self-scoped entry reads land
  (security-design §11 dep 1) — or an explicit deviation is recorded.
- **Tracing:** the gateway adds `x-correlation-id` to every `/v1` call; the gate screen displays
  it on error surfaces for support. Browser → gateway → Identity correlation rides that header;
  the feature adds no trace instrumentation of its own (fact 1).
- **Retention:** platform default; no unit-specific override.

## Dashboards

No new board. When the platform collection stack lands, gate traffic appears on the existing
gateway and `web` dashboards through the route labels those units own; a gate-specific panel
would need the telemetry route (fact 1) and is deliberately not specified here.
