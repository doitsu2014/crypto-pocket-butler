# Monitoring Design — u7-settings-ui

**Upstream:** `nfr-design/security-design.md` (§5 telemetry, §10 rate limiting, §11 audit),
`nfr-design/performance-design.md` (§1 budget map, §9 verification), `infrastructure-specification.md`
(this unit), `nfr-requirements/performance-requirements.md` (measurement-ownership split).

## The collection gap, stated first

1. **No Prometheus runs in either stack.** Every service exposes `/metrics`; `infra/prometheus/`
   holds alert rules but nothing scrapes them, and no exporter sidecar is declared. Every runtime
   alert below is therefore defined against a path that does not run yet — the same gap the other
   units' monitoring designs record. Naming it is the honest alternative to pretending the metrics
   above are already watched.
2. **The settings feature emits no telemetry of its own** (security-design §5): no analytics
   event, no client-side latency sample, no error payload. The only browser-emitted signal that can
   mention a settings route is the shell's existing render-crash beacon
   (`web/src/telemetry.ts` → `POST /v1/telemetry/client-errors`, carrying route, error code,
   correlation id and session id and no settings data). **That endpoint is not in the gateway's
   route table**, so today the beacon 404s and is dropped — a platform gap this unit neither fixes
   nor hides.
3. **Server hops are measured by their owners — but the route label is not automatic.** Identity's
   `http_requests_total{…,route}` is labelled by `route_label` in
   `services/identity/http/src/middleware.rs`, and that function is a **static match list** whose
   fallback arm is `"other"`. The new paths (`/v1/me/account-status`, `/v1/me/profile`,
   `/v1/me/deletion-request`, `/v1/privacy/deletions`) would all be counted as `route="other"`
   without an arm added for them, which would make every per-route series below uncomputable.
   **This unit therefore adds those arms as part of its change** — it is the same crate as the
   handlers — and the row is listed in `infrastructure-specification.md` "What must exist before
   this works". No new metric family is added, and none is duplicated. The gateway labels by
   service and status class only: it has no route label and no latency histogram.

## Metrics & KPIs

| Metric | Source | Threshold | Why it matters |
|---|---|---|---|
| Settings route traffic and status classes | `identity-api` `/metrics`: `http_requests_total{service,status_class,route}` — the new route labels appear **once this unit adds their arms to `route_label`** (collection gap 3) | — (trend) | Shows the settings surface is being used and what it answers |
| Identity 5xx rate | `identity-api` `/metrics`: `http_requests_total{status_class="5xx", route}` once the route-label arms exist (collection gap 3); `identity_request_errors_total{service}` is service-wide only and cannot be scoped to settings | any sustained rise above the platform error budget | A 5xx here is a save or a deletion request that failed |
| Deletion lifecycle outcomes | `identity-api` `/metrics`: `identity_deletion_status_total{status}` — the existing counter, whose only status labels are `requested`, `in_progress`, `completed` and `failed` | an unexpected `failed` share; an `in_progress` count that does not drain | The irreversible path's own signal |
| Deletion cancellations | `identity-api` `/metrics`: `deletion_requests_total{event}` — cancellations land here as `event="cancelled"`, not in the status counter | a cancellation surge | The restore path's own signal |
| Prepared-principal and lifecycle answers | `identity-api` `/metrics`: `identity_principal_status_total{status}` | — (trend) | Status distribution behind the blocked and restore screens |
| Gateway failures reaching `/v1` | `gateway` `/metrics`: `http_errors_total{service}` and `http_requests_total{service,status_class}` — **service-wide, no route label** | per the gateway unit's alert table | Covers a gateway-side failure that never reaches Identity; cannot isolate the settings paths and does not claim to |
| `429` from the per-route limiter | **not derivable from the metrics today** — the gateway counts by status class, so a D3 429 lands inside `status_class="4xx"`, and the limiter has no counter of its own | — | The evidence for a scripted trigger is the edge access log, where a stream of 429s on one path is visible; a dedicated counter is a gateway-unit change, not this unit's |
| Route-chunk size (gzip) | **Build time**: `yarn size` — size-limit `route-*.js` glob + `web/scripts/check-size.mjs` | ≤ 150 kB route chunk; 300 kB initial; 150 kB stylesheet | The mechanical NFR2.1 gate, measured in the build, not at runtime |
| nginx requests/connections | `web` `stub_status` :8081 | — (trend) | Static-host health; existing source, uncollected |

**Deliberately not collected: latency.** NFR2.2–NFR2.6's p95 targets have **no runtime source**,
and there is no server-side substitute either. The gateway's `/metrics` exposes status-class and
5xx counters plus `service_up` — no latency histogram and no route label — and Identity's
histograms cover the authorisation decision, the consent-state read and the support read/write
only; no D0 route duration exists. Those targets are verified the way the requirements already say:
request-count and state assertions in the MSW-backed suite (`performance-design.md` §9) and a
devtools profiling pass, which is what `performance-requirements.md` records. This design states the
absence rather than inventing a histogram nobody scrapes or a latency SLI the series cannot answer.

## Alerts

| Alert | Condition | Severity | Routes to |
|---|---|---|---|
| Route-chunk budget exceeded | `yarn size` fails | **Build-blocking** — a pipeline stage, not a runtime alert | the changing developer |
| Identity 5xx on the settings routes | `http_requests_total{status_class="5xx", route="me.profile"\|"me.account-status"\|"me.deletion.request"\|…}` rising while that route's traffic is steady — derivable only after this unit adds the route-label arms (`identity_request_errors_total` alone is service-wide) | P2 | Identity on-call |
| Deletion requests failing | `identity_deletion_status_total{status="failed"}` rising, or `identity_deletion_status_total{status="in_progress"}` not draining | P1 | Identity on-call; an irreversible action that is not progressing is the case a person cannot fix alone |
| Sustained `429` from the per-route limiter | a steady stream of 429s on `POST /v1/privacy/deletions`, read from the **edge access log** (no metric isolates them — see Metrics & KPIs) | P3 (investigate) | platform on-call |
| Static host down | `web` health check failing during a rolling update | Swarm's `failure_action: rollback` acts automatically; the edge alert (once Prometheus lands) pages | platform on-call |

There are **no unit-owned runtime alerts** beyond those: with no client telemetry and no collection
stack, anything more specific would be an alert nobody can act on, which the team's own on-call
rule deletes.

## SLIs / SLOs

| SLI | SLO target | Measurement window | Measured where |
|---|---|---|---|
| Settings read success (2xx or a defined 4xx answer / attempts) | within the platform's 99.9% monthly availability floor (BRD-012 NFR-AVL-01) | 30-day rolling | `identity-api` metrics, route labels |
| Settings save success (P2, X2, X5/X6) | same floor | 30-day rolling | `identity-api` metrics |
| Deletion request acceptance (202 or a defined 409/410 / attempts) | same floor | 30-day rolling | `identity-api` `identity_deletion_status_total` |
| Restore within the recovery window (restores completed / requests that reached their window) | a product SLO, not an infrastructure one — **not asserted here**; the counters are the evidence, and the target is a product decision | — | `identity_deletion_status_total` |
| Static asset availability | health check passing; a failed update rolls back automatically | continuous | Swarm + `web` `/healthz` |

## Logs & tracing

- **Edge access logs:** nginx logs the URL and never request headers. Every settings route is
  self-scoped (`/v1/me/...`, `/v1/privacy/deletions`), so those URLs carry no user id, email or
  subject (security-design §5) — the log format needs no change, and the route shape is what keeps
  it clean. The one id-bearing path, `GET /v1/exports/{exportId}`, carries an opaque job id the
  server ownership-checks.
- **Service logs:** Identity's existing structured logging applies; the new handlers log the same
  way as the routes they sit beside. **No credentials, tokens or `ProblemDetails.detail` in a log**
  (the unit renders only title, status and correlation id on screen, and the same rule holds for
  its logs).
- **Tracing:** the gateway adds `x-correlation-id` to every `/v1` call and the settings error
  surfaces display it, so a person can quote a correlation id that ties their failed save to the
  gateway, Identity and audit rows. The feature adds no instrumentation of its own.
- **Retention:** platform default; no unit-specific override.

## Dashboards

No new board. When a collection stack lands, the settings routes appear on the existing gateway and
Identity dashboards through the route labels those services already emit; a settings-specific panel
would need client telemetry (fact 2) and is deliberately not specified.