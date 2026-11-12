# Monitoring Design — u9-support-console-ui

**Upstream:** `nfr-design/performance-design.md` (§8 measurement ownership), `nfr-design/
security-design.md` (§3 route budgets, §5 telemetry posture, §7 freshness), `nfr-design/
logical-components.md`, `infrastructure-specification.md`, nfr-design measurement-ownership split.

## The collection gap, stated first

Three facts frame everything below:

1. **The console emits no telemetry** — no telemetry module, no error beacon, no analytics, and no
   `console.*` of record contents (security-design §5); this unit adds none and removes nothing of
   the kind. There is no client-side signal naming the support screen, and none is invented.
2. **No Prometheus exists in either stack.** The `web` image exposes nginx `stub_status` on :8081
   (internal only) for an nginx-prometheus-exporter sidecar that is not declared — the gap u2, u8
   and u10 recorded. Every runtime metric below is defined against a collection path that does not
   run yet.
3. **The per-route measurement surface does not exist.** The owners' `/metrics` endpoints are
   real, but they do not expose what the budgets below would need:
   - **Gateway** (`services/gateway/http/src/metrics.rs`) renders status-class counters only —
     `http_requests_total{service,status_class}`, `http_errors_total{service}`,
     `service_up{service}`. No route label and no latency histogram. A limiter rejection **is**
     counted, but only as a status class: `record_metrics` is the outermost layer on the gateway
     router (`services/gateway/http/src/routes.rs:25-28`), so the `429` `rate_limit` returns lands
     in `http_requests_total{service="gateway",status_class="4xx"}` — there is no route label and
     no `429`-specific series, so a rejection cannot be counted per route or told apart from other
     client errors.
   - **Identity** (`services/identity/http/src/metrics.rs`) renders route-labelled status counters
     (`http_requests_total{service,status_class,route}`, labelled from `route_label` in
     `middleware.rs`) and operation histograms — `support_read_duration_seconds`,
     `support_write_duration_seconds` — but **no HTTP request latency histogram**, and the two
     support histograms carry **no route label**, so they bound the support surface rather than one
     call.
   - **This unit adds no `route_label` arm.** Unlike u10, whose console called four new admin paths,
     U9's only new path is S2, and its arm belongs to the delivery that creates the route (**U3**).
     U9's own change cannot make the read visible in `route_label` terms.

   So the latency targets below are **unmeasured** until the gateway gains a route label and a
   latency histogram and Identity gains one for its HTTP routes (plus a limiter counter for the
   `429` signal). Those are gateway/Identity platform changes owned outside this unit; they are
   recorded as gaps, not claimed as measurements.

## Metrics & KPIs

| Metric | Source | Threshold | Why it matters |
|---|---|---|---|
| Grant-request latency (p95) | Identity `support_write_duration_seconds` (support operations, **no route label**) | < 2 s (NFR2.1) | Bounds the support write surface; the one-attempt behaviour is the client half and is asserted in the MSW suite, not timed |
| Own-grant read latency (p95) | **No source today** — needs a route-labelled duration series, which is U3's to add with the route | < 2 s for ≤ 20 rows (NFR2.3) | Unmeasured target; the acknowledged gap in performance design §8 |
| Metadata read latency (p95) | Identity `support_read_duration_seconds` (support operations, **no route label**) | < 2 s perceived (NFR2.4); U3's fan-out stays within its own 500 ms budget | The series exists and bounds the surface; it does not isolate S3 |
| Per-route `429` count | Gateway `http_requests_total{status_class="4xx"}` does count limiter rejections, but with **no route label and no `429`-specific series** | — (trend) | Budgets are per token and per replica; noticing a lockout risk means reading the edge access log or the 4xx aggregate, not a per-route count |
| Gateway 5xx share | `http_errors_total{service}` — **service-wide, no route label** | per the gateway unit's alerts | Server-side error rate of all gateway traffic, support calls included but not isolated |
| nginx connections/requests | `web` `stub_status` :8081 → future exporter sidecar | — (trend) | Static-host health; existing source, uncollected |
| Console build and test gates | Build-time: `yarn --cwd support-console format:check / lint / build / test` | blocks the build | The console has **no `size` script** (unlike `web`), so the section's chunk size is bounded by review against `web`'s route-chunk budget, not by a mechanical gate — stated, not claimed. This unit shrinks the bundle (three components retired), it does not grow it |

## Alerts

| Alert | Condition | Severity | Routes to |
|---|---|---|---|
| Console gates failing | any `support-console` gate fails in the pipeline | **Build-blocking** (pipeline stage, not a runtime alert) | the changing developer |
| Support write/read 5xx rate | gateway `http_errors_total{service}` is service-wide; the threshold is the gateway unit's to set | per that unit | gateway / Identity on-call |
| Static-host down | `web` health check failure during a rolling update | Swarm `failure_action: rollback` acts automatically; platform edge alerting (when Prometheus lands) pages | platform on-call |

There are **no unit-owned runtime alerts**, and this table says so instead of inventing them: with no
console telemetry (fact 1) and no collection stack (fact 2), nothing exists to arm one against, and
an alert nobody can act on is the thing the team's own rule deletes.

## SLIs / SLOs

| SLI | SLO target | Measurement window | Measured where |
|---|---|---|---|
| Grant-request latency (server side) | p95 < 2 s | 30-day rolling | Identity `support_write_duration_seconds` once collected (fact 2); surface-wide, not per route |
| Own-grant read latency | p95 < 2 s for ≤ 20 rows — **unmeasured target** | 30-day rolling once a series exists | **No series** — U3 owns a route-labelled duration series if one is wanted |
| Metadata read latency | p95 < 2 s — **unmeasured target** | 30-day rolling once collected | Identity `support_read_duration_seconds` (surface-wide) |
| Support read/write availability (allowed outcomes / non-5xx attempts) | platform 99.9% monthly floor (BRD-012 NFR-AVL-01) | 30-day rolling | Gateway status-class counters, service-wide; Identity `http_requests_total{route}` for S3 today |
| Console/static asset availability | health check passing; a failed update rolls back automatically | continuous | Swarm + `web` `/healthz` |
| Client-perceived latency | **Not collected** — recorded as a gap, not budgeted a number | — | would need console telemetry (fact 1) |

## Logs & tracing

- **Edge access logs:** nginx's default `combined` format logs the request URL plus referer and
  user-agent — no request body, and no credential-bearing request header such as `authorization` or
  `cookie` (`web/nginx.conf`; the local stack's config makes the same default). U9's two
  case-bearing values stay out of that surface: the **reason** travels in the `POST /v1/support-grants`
  body, and S2's path is static. S3's path carries the **grant UUID**, recorded as an accepted
  residual in `infrastructure-specification.md` (opaque, already in U3's audit record, not personal
  data). The `X-Support-Reason` header is retired, and the nginx comment that still names it is
  corrected in the same change.
- **Tracing:** the gateway adds `x-correlation-id` to every `/v1` call and the console renders it on
  every error surface (`ProblemDetails` correlation id); browser → gateway → Identity correlation
  rides that header. The section adds no trace instrumentation of its own (fact 1).
- **Audit:** the create and the metadata read are audited by Identity (U3, C9); the console writes
  no audit record and renders only the statement that the read is recorded (NFR21.1). A `403`
  rendered as access-ended therefore never claims a read that was not audited.
- **Retention:** platform default; no unit-specific override.

## Dashboards

No new board, and no existing series to hang one on. Two things would have to change first: a
collection stack (fact 2), and the instrumentation gaps in fact 3 — the gateway has no route label,
and Identity's support histograms are not route-labelled, so support traffic shows as a surface
rather than as S1/S2/S3 rows. The S2 arm in `route_label` is U3's with the route. A console-side
panel would need telemetry the design deliberately does not emit (fact 1) and is therefore not
specified.