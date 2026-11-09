# Performance Requirements — u1-walking-skeleton

Inherits the inception NFRs; each detailed requirement carries its parent ID. The targets apply
to the workflows in `functional-design/functional-spec.md` (asset resolution WF1, gateway path
WF4) and the rules in `functional-design/rules.md`.

| ID | Requirement | Target | Parent | Verification |
|---|---|---|---|---|
| NFR2.1 | Asset-resolution request latency, server-side | p95 <200 ms | NFR2 | Load test at launch capacity, p95 measured at the gateway |
| NFR2.2 | Gateway pass-through overhead | <20 ms p95 added to the downstream call | NFR2 | Gateway latency histogram minus downstream latency |
| NFR6.1 | Asset resolution within the first-valuation path | does not consume more than 200 ms of the two-minute first-value budget | NFR6 | End-to-end onboarding test |
| NFR3.1 | Registry reads under repeated load | cache hit ratio ≥90% for asset resolution at steady state | NFR2 | Cache metrics |

## Notes

- Asset resolution is read-heavy and changes rarely; the in-process cache (TTL 60 s, explicit
  invalidation on registry change) carries the steady-state load.
- No endpoint in this Unit performs price collection; that budget belongs to Market Data.
