# Observability Design — u1-walking-skeleton

## SLIs and SLOs

| SLI | Definition | SLO |
|---|---|---|
| Availability | successful requests ÷ total requests, measured at the edge | 99.9% over a 30-day window |
| Gateway latency | p95 of gateway request duration | <20 ms overhead |
| Registry latency | p95 of asset-resolution duration | <200 ms |
| Error rate | 5xx ÷ total requests | <0.1% (the request-level implication of 99.9% availability) |

## Alerting rules (all below the SLO breach)

| Alert | SLI + threshold + window | Rationale |
|---|---|---|
| Availability burn | burn rate >2 over 1 h **and** >1 over 6 h (multi-window) | fires while there is budget left to act |
| Error rate | 5xx >0.1% over 5 min | matches the request-error rate implied by 99.9% availability |
| Registry latency | p95 >200 ms for 10 min | below the SLO's breach pace |
| Cache hit ratio | <80% for 15 min | early signal of invalidation churn |
| JWKS staleness | cached keys older than 8 min | 2-minute lead before the 10-minute fail-closed window |

The scheduler heartbeat alert belongs to the Job Orchestration unit; it is not raised by this
Unit, which has no scheduler.

## Metrics

- Gateway: request rate, latency histogram, status classes, rate-limit rejections, JWT validation
  failures, JWKS age.
- Registry: resolution rate, latency histogram, cache hit ratio, cache size, registry change
  version, quarantine count, audit emission failures.

## Structured logging

- JSON lines with timestamp, level, service, correlation ID, route, status, and duration.
- Never tokens, credentials, or user wallet addresses.
- Registry administration logs name the actor and the internal registry target.

## Distributed tracing

- W3C trace context propagated gateway → registry → audit.
- One trace per request, **100% sampling at launch** because volume is low; tail sampling is
  introduced when volume rises.
- The trace ID is emitted with the structured log line and the problem response.

## Dashboards

- Gateway: request rate, latency, errors, rate limits, JWKS age.
- Registry: resolutions, cache hit ratio, quarantine, change rate, audit failures.
