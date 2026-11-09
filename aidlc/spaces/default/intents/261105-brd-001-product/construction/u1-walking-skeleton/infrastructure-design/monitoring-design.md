# Monitoring Design — u1-walking-skeleton

Implements the strategy in `nfr-design/observability-design.md` on the self-hosted stack.

## Metrics & KPIs

| Metric | Source | Threshold | Why it matters |
|---|---|---|---|
| Gateway request rate | Prometheus scrape | — | capacity signal |
| Gateway latency p95 | histogram | >20 ms overhead | NFR2.2 |
| Registry latency p95 | histogram | >200 ms | NFR2.1 |
| 5xx rate | request counter | >0.1% over 5 min | availability |
| Rate-limit rejections | counter | spike | abuse |
| JWT validation failures | counter | spike | attack or misconfiguration |
| JWKS cache age | gauge | >8 min | 2-minute lead before fail-closed |
| Cache hit ratio | counter ratio | <80% | invalidation churn |
| Registry change version | gauge | stalled bump | write path issue |
| Quarantine count | gauge | >10 000 per account | abuse or registry gap |
| Audit emission failures | counter | any sustained | audit trail risk |

## Alerts

| Alert | Condition | Severity | Routes to |
|---|---|---|---|
| Availability burn | burn rate >2 over 1 h **and** >1 over 6 h | P1 | on-call |
| Error rate breach indicator | 5xx >0.1% over 5 min | P1 | on-call |
| Registry latency | p95 >200 ms for 10 min | P2 | on-call |
| JWKS staleness | cache age >8 min | P1 | on-call |
| Cache hit ratio | <80% for 15 min | P3 | team channel |
| Quarantine growth | 10 000 rows in one account | P2 | team channel |
| Audit failures | sustained emission failures | P2 | on-call |

## SLIs / SLOs

| SLI | SLO target | Measurement window |
|---|---|---|
| Availability | 99.9% | 30-day rolling |
| Gateway latency p95 | <20 ms overhead | 30-day rolling |
| Registry latency p95 | <200 ms | 30-day rolling |
| Error rate | <0.1% 5xx | 30-day rolling |

## Logs and tracing

- **Logs:** Loki; JSON lines with timestamp, level, service, correlation ID, route, status,
  duration. No tokens, credentials, or user wallet addresses. 30-day retention at launch.
- **Traces:** Tempo; W3C trace context; one trace per request; 100% head sampling at launch.
  Expected span volume at 300 RPS is roughly 600 spans/s (gateway + registry); retention 7 days.
  Tail sampling replaces head sampling when volume exceeds 1 000 spans/s.
- **Dashboards:** gateway (rate, latency, errors, rate limits, JWKS age) and registry
  (resolutions, cache hit ratio, quarantine, change rate, audit failures).
