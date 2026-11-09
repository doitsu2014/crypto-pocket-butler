# Observability Requirements — u1-walking-skeleton

| ID | Requirement | Target | Parent |
|---|---|---|---|
| NFR17.1 | Structured logs with a correlation ID on every request | 100% of gateway and registry logs carry the correlation ID | NFR17 |
| NFR17.2 | Health and readiness endpoints | every replica exposes both | NFR17 |
| NFR17.3 | Metrics | request latency histogram, error rate, cache hit ratio, registry read count | NFR17 |
| NFR17.4 | Traces | every request produces a trace spanning gateway and registry | NFR17 |
| NFR17.5 | Alerts | availability burn below 99.9%, error rate above 1%, missed heartbeat — all below the SLO | NFR17 |

## Alert thresholds (set below the SLO)

| Alert | Condition | Why below the SLO |
|---|---|---|
| Availability burn | error budget burn above the 30-day 99.9% allowance at a 5-minute window | leaves time to act before the monthly SLO is lost |
| Error rate | 5xx above 1% for 5 minutes | a sustained 1% error rate is well below a 99.9% availability breach at request level |
| Missed heartbeat | scheduler heartbeat older than 5 minutes | detects a silent stop before jobs visibly fail |

## Dashboards

- Gateway: request rate, latency, error rate, rate-limit rejections, JWKS validation failures.
- Registry: resolution rate, cache hit ratio, quarantine count, registry change rate.
