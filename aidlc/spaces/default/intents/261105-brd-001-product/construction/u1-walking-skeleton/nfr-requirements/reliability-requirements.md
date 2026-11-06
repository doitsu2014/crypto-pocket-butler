# Reliability Requirements — u1-walking-skeleton

| ID | Requirement | Target | Parent |
|---|---|---|---|
| NFR1.1 | Gateway and registry availability | ≥99.9% monthly, measured at the edge | NFR1 |
| NFR18.1 | Recovery point / recovery time for the registry schema | RPO ≤24 h, RTO ≤4 h | NFR18 |
| NFR18.2 | Migration safety | versioned up/down pairs; a down migration restores the previous schema | NFR18 |
| NFR1.2 | Graceful degradation | registry unavailable → callers keep last-known identity and label it stale | NFR1 |
| NFR1.3 | Health checks gate every deployment | readiness reflects a working schema connection | NFR1 |

## Failure modes

| Component | Failure | Behaviour |
|---|---|---|
| Registry | database unavailable | 503 with a problem response; callers keep last-known identity |
| Gateway | JWKS endpoint unavailable | cached keys serve until a bounded staleness window, then fail closed with 503 |
| Gateway | replica loss | remaining replicas serve; rolling update health-gated |
| Cache | cold start | resolution falls through to the schema; latency rises but stays inside NFR2.1 |
