# Performance Requirements — u10-analytics

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`, and
`inception/requirements-analysis/requirements.md`.

| ID | Requirement | Target |
|---|---|---|
| NFR2.1 | `POST /v1/events` end to end (C3 consent call included) | C3 assumed p95 ≤ 40 ms, timeout 80 ms, one attempt; overall p95 ≤ 120 ms, p99 ≤ 300 ms |
| NFR2.2 | `GET /v1/flags/{key}` | p95 ≤ 30 ms |
| NFR2.3 | Nightly aggregation over ≤ 200 k events | ≤ 2 minutes |
| NFR2.4 | Aggregate read (one metric, one period) | p95 ≤ 150 ms |
| NFR2.5 | Erase by authUserId | ≤ 30 s for a user with ≤ 100 k events |

The ingest path makes one C3 call for analytics-class events; the flag path is a single indexed
lookup plus an in-memory hash.
