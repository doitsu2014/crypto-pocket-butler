# Performance Requirements — u11-job-orchestration

Sources: `functional-design/functional-spec.md` (WF1 tick, WF2 call budget) and
`functional-design/rules.md`.

| ID | Requirement | Measure |
|---|---|---|
| NFR2.1 | A tick schedules all due work and returns within 30 s, excluding outbound calls | p95 of `job_orchestration_tick_duration_seconds` (histogram, NFR17.2) ≤ 30 s |
| NFR2.2 | At most 32 outbound run calls in flight per instance | Configuration constant asserted by a test |
| NFR2.3 | A per-job call is bounded by its `timeoutSeconds` (1–3,600) plus a 30 s lease margin | HTTP client timeout; a timeout records `failed` |
| NFR2.4 | Operator list reads answer within 300 ms at p95 | `GET /v1/jobs` and `/runs` request metrics |
| NFR2.5 | The heartbeat age stays under 90 s while the tick loop is healthy | `job_orchestration_heartbeat_age_seconds` |
| NFR2.6 | The due query answers within 50 ms at p95 with 8.1M retained run rows | 20,300 windows/day × 400 days ≈ 8.1M rows; indexes `(job, scheduled_for desc)`, `(status, next_attempt_at)`; EXPLAIN check in CI |
| NFR2.7 | A backfill or manual-run request answers within 500 ms at p95 | A `every:1` 7-day backfill queues 10,080 pending rows in one summary insert; the response is counts plus the covered-window count, never the full list |
