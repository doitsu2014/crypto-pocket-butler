# Monitoring Design — u10-admin-users-ui

**Upstream:** `nfr-design/performance-design.md` (§8 measurement ownership), `nfr-design/
security-design.md` (§3 route budgets, §5 telemetry posture, §7 provider read),
`nfr-design/logical-components.md`, `infrastructure-specification.md`, nfr-design
measurement-ownership split.

## The collection gap, stated first

Three facts frame everything below:

1. **The console emits no telemetry** — it has no telemetry module, no error beacon, no
   analytics and no `console.*` of record contents (security-design §5), and this unit adds none.
   There is therefore no client-side signal naming the admin screen, and none is invented: a
   screen that leaks nothing it renders is the design, not an omission to instrument away.
2. **No Prometheus exists in either stack.** The `web` image exposes nginx `stub_status` on
   :8081 (internal only) for an nginx-prometheus-exporter sidecar that is not declared — the same
   gap u2's and u8's monitoring designs recorded. Every runtime metric below is defined against a
   collection path that does not run yet.
3. **The per-route measurement surface does not exist yet.** The owners' `/metrics` endpoints are
   real, but they do not expose what the budgets below would need:
   - **Gateway** (`services/gateway/http/src/metrics.rs`) renders status-class counters only —
     `http_requests_total{service,status_class}`, `http_errors_total{service}`, `service_up
     {service}`. There is **no route label and no latency histogram**, and a `check_route`
     rate-limit rejection increments nothing of its own, so a `429` is invisible in metrics.
   - **Identity** (`services/identity/http/src/metrics.rs`) renders route-labelled status counters
     (`http_requests_total{service,status_class,route}`; the label comes from `route_label` in
     `middleware.rs`) and operation histograms (`support_read_duration_seconds`,
     `support_write_duration_seconds`, `authorisation_decision_duration_seconds`), but **no HTTP
     request latency histogram**. `route_label` is a static match list: the four U10 admin paths
     fall under `route="other"` unless arms are added — **that arm list is this unit's code
     addition**, in the same change as the routes, and it makes the admin calls *visible*, not
     *timed*.
   - With no Prometheus (fact 2), none of it is collected anyway.

   So the latency targets below are **unmeasured** until the gateway gains a route label and a
   latency histogram and Identity gains one for its HTTP routes (plus a limiter counter for the
   `429` signal). Those are gateway/Identity platform changes owned outside this unit; they are
   recorded as gaps, not claimed as measurements. What this unit owns is the `route_label` arms and
   the status counters that flow from them.

## Metrics & KPIs

| Metric | Source | Threshold | Why it matters |
|---|---|---|---|
| Lookup latency (p95) | **No source today** — needs an HTTP latency histogram on gateway and Identity (fact 3) | < 2 s (NFR2.1), provider share ≤ 1 s | Unmeasured target until that histogram exists; the budget is what the first measurement will judge |
| Provider role-read latency and failures | Identity `SupabaseRoleWriter`/`RoleReader` adapter timing | 3 s ceiling (connect 1 s); any `503` rate is the signal | The adapter has no timing metric today; the lookup's `5xx` count is the only proxy, and it is service-wide |
| Role-change / disable / restore latency (p95) | **No source today** — same histogram gap; Identity counts them as `http_requests_total{route}` once the arms land | < 2 s (NFR2.3/NFR2.4) | Status counts, not latency; the budget is unmeasured |
| Pending-grant list latency (p95) | Identity `http_requests_total{route="support.grants"}` (status only; `support_read_duration_seconds` covers U3's support-operation reads, not HTTP latency) | < 2 s for ≤ 100 rows (NFR2.5) — **no automated gate**, first measured after deployment | Status counts show the list is being called and answering; the latency budget has no series yet |
| Per-route `429` count | **No metric source** — `check_route` rejections are not counted; only the edge access log shows them | — (trend) | Budgets are per token, per replica; noticing a lockout risk today means reading the edge log |
| Gateway 5xx share | `http_errors_total{service}` — **service-wide, no route label** | per the gateway unit's alerts | Server-side error rate of all gateway traffic, admin calls included but not isolated |
| nginx connections/requests | `web` `stub_status` :8081 → future exporter sidecar | — (trend) | Static-host health; existing source, uncollected |
| Console build and test gates | Build-time: `yarn --cwd support-console test/lint/build/format:check` | blocks the build | The console has **no `size` script** (unlike `web`), so the section's chunk size is bounded only by review against `web`'s route-chunk budget, not by a mechanical gate — stated, not claimed |

## Alerts

| Alert | Condition | Severity | Routes to |
|---|---|---|---|
| Console gates failing | any `support-console` gate fails in the pipeline | **Build-blocking** (pipeline stage, not a runtime alert) | the changing developer |
| Admin lookup `503` rate | sustained provider-failure `5xx` on the gateway — service-wide, so the threshold is the gateway unit's to set | per that unit | Identity on-call |
| Mutation 5xx / latency | gateway-side conditions owned by the gateway unit; latency cannot be alerted on until the histogram exists (fact 3) | per that unit | gateway / Identity on-call |
| Static-host down | `web` health check failure during a rolling update | Swarm `failure_action: rollback` acts automatically; the platform edge alerting (when Prometheus lands) pages | platform on-call |

There are **no unit-owned runtime alerts**, and this table says so instead of inventing them:
with no console telemetry (fact 1) and no collection stack (fact 2), nothing exists to arm one
against, and an alert nobody can act on is the thing the team's own rule deletes.

## SLIs / SLOs

| SLI | SLO target | Measurement window | Measured where |
|---|---|---|---|
| Admin lookup latency (server side) | p95 < 2 s | 30-day rolling | **Unmeasured** — no HTTP latency histogram on gateway or Identity (fact 3) |
| Admin mutation success (allowed outcomes / non-5xx attempts) | per the platform 99.9% monthly availability floor (BRD-012 NFR-AVL-01) | 30-day rolling | Gateway status-class counters, service-wide; Identity `http_requests_total{route}` once the arms land |
| Pending-grant list latency | p95 < 2 s for ≤ 100 rows — **unmeasured target** | 30-day rolling once a series exists | Identity status counters only; no latency series, no gate |
| Console/static asset availability | health check passing; failed updates roll back automatically | continuous | Swarm + `web` `/healthz` |
| Client-perceived latency | **Not collected** — recorded as a gap, not budgeted a number | — | would need console telemetry (fact 1) |

## Logs & tracing

- **Edge access logs:** nginx's default `combined` format logs the request URL **plus the referer
  and user-agent** — no request body, and no credential-bearing request header such as
  `authorization` or `cookie` (`web/nginx.conf`; the local stack's config makes the same default).
  D-S1 is what keeps the looked-up address out of that surface: the address travels in a POST
  body, so no U10 URL carries it, and the URL is the logged field that would otherwise hold it. The
  three mutation paths still carry the **internal user UUID**, recorded as an accepted residual in
  security-design §1.
- **Tracing:** the gateway adds `x-correlation-id` to every `/v1` call and the console renders it
  on every error surface; browser → gateway → Identity correlation rides that header. The section
  adds no trace instrumentation of its own (fact 1).
- **Audit:** the lookup's own audit event (`identity.admin.user_lookup`, allowed and refused) and
  the mutation audit records are server-side (Identity, C9) and unchanged; a provider failure
  records `Failed` with its own context, so the trail never shows a successful lookup that
  answered `503` (security-design §7).
- **Retention:** platform default; no unit-specific override.

## Dashboards

No new board, and no existing series to hang one on. Two things would have to change first: a
collection stack (fact 2), and the instrumentation gaps in fact 3 — the gateway has no route
label, so admin traffic would show service-wide; Identity would count it under `route="other"`
until the `route_label` arms this unit ships. A console-side panel would need telemetry the design
deliberately does not emit (fact 1) and is therefore not specified.