# Monitoring Design — u2-authorisation

**Upstream:** `observability-design.md`, `infrastructure-specification.md`, `reliability-design.md`.

## The collection gap, stated first

The observability stack (Prometheus, Alertmanager, Loki, Tempo, Grafana) is documented as the
target, but the repository declares **no Prometheus service and no scrape configuration**:
`infra/prometheus/` holds only `job-orchestration-rules.yml`, and neither `infra/swarm-stack.yml`
nor `deployment/localhost/docker-compose.yml` declares Prometheus. So the metrics below are
emitted by the Identity service at `/metrics` and **collected only once the platform work lands**.
This unit records the scrape job it needs rather than assuming collection:

```yaml
# scrape job to add with the Prometheus service (platform work, not this unit)
- job_name: identity-api
  metrics_path: /metrics
  static_configs:
    - targets: ["identity-api:8083"]
```

That gap is an open item, not a hidden assumption: until it is closed, every alert below is
defined but not armed.

## Metrics & KPIs

| Metric | Source | Threshold | Why it matters |
|---|---|---|---|
| `authorisation_decisions_total` | Identity `/metrics` (new, this unit) | — (rate by class and outcome) | The unit's throughput and denial mix |
| `authorisation_decision_duration_seconds` | Identity `/metrics` (new) | p95 above the budget for that `path` **and** `outcome` | Guards the latency targets |
| `authorisation_status_lookup_errors_total` | Identity `/metrics` (new) | > 1% of decisions over 5 min | The status source is the only read dependency |
| `authorisation_audit_write_failures_total` | Identity `/metrics` (new) | any non-zero over 5 min | Denials are failing closed |
| `authorisation_role_changes_total` | Identity `/metrics` (new) | > 5% failures over 30 min | Privileged-path health |
| `authorisation_cache_entries` | Identity `/metrics` (new) | approaching 10 000 | Cache near its cap |
| `http_requests_total{service,status_class}` | Identity `/metrics` (**existing**) | 5xx share on the C4 route | The route's own health |
| `identity_request_errors_total{service}` | Identity `/metrics` (**existing**) | any rise | Error-rate floor for the service |

**The route-level alert needs a label that does not exist yet.** The existing counters carry
`service` and `status_class` but no route, so the C4 route cannot be isolated from `/v1`
traffic. The unit's code plan therefore adds a `route` label (value `internal.authorize`) to the
emitted request counter; until that lands, the C4 alert uses the service-wide 5xx share and says
so.

## Alerts

| Alert | Condition | Severity | Routes to |
|---|---|---|---|
| Audit write failures | any non-zero over 5 minutes | P1 (page) | Identity on-call; check the audit service, denials are returning 503 |
| Status lookup errors | > 1% over 5 minutes | P1 (page) | Identity on-call; check Postgres and the pool |
| Decision latency — warning | per `path` and `outcome` pair above the budget in `performance-design.md` for 10 minutes | P2 (ticket) | Identity team |
| Decision latency — escalation | per `path` and `outcome` pair above 2× that budget for 10 minutes | P1 (page) | Identity on-call |
| Role-change failures | > 5% over 30 minutes | P2 (ticket) | Identity team; check the Supabase Auth admin API |
| Cache entries near cap | `authorisation_cache_entries` > 9 000 for 10 minutes | P3 (dashboard) | Memory pressure on the replica; misses are the normal path |
| C4 route 5xx | 5xx share on the route label > 1% over 5 minutes (service-wide until the label lands) | P2 (ticket) | Identity team |

There is **no cache-age alert**: entries older than the 30-second TTL are never served, so an age
gauge above 30 s cannot occur — an alert on it could never fire, and an alert nobody can act on
is the thing the team's own rule deletes.

## SLIs / SLOs

| SLI | SLO target | Measurement window |
|---|---|---|
| User-visible decision success (allowed, or denied with its audit entry recorded) | 99.9% | 30-day rolling |
| Designed 503s (`audit_unavailable`, cold-cache status outage) | Reported, not budgeted into availability | 30-day rolling |
| Latency compliance (inside the budget for its `path` and `outcome`) | 99% | 30-day rolling |
| Denial audit completeness | 100% (a miss is an incident) | 30-day rolling |
| Role-change audit completeness (intent and outcome) | 100% | 30-day rolling |

## Logs & tracing

- **Aggregation:** structured JSON to stdout, collected by the existing Loki pipeline; every line
  carries `service`, `trace_id`, `correlation_id`, `operation_class`, `outcome`. No token,
  subject plaintext, credential, secret, or full wallet address.
- **Tracing:** the decision is a child span of the caller's request (Tempo); the C4 route opens a
  server span linked to the caller's trace. A role change is one span with the intent write, the
  provider write, and the outcome write as its steps.
- **Retention:** the platform default; no unit-specific override.

## Dashboards

One board on the existing Grafana: decisions by class and outcome; denial reasons; latency by
`path` and `outcome` with each budget line; status-lookup errors; audit-write failures; role
changes; cache entries. Panels link to the runbooks above.
