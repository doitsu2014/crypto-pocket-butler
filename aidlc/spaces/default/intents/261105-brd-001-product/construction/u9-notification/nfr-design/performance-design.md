# Performance Design — u9-notification

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`, `scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`, `tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

- Submit: one dedupe check + insert; a mandatory alert also sets a durable `wake_requested_at` flag
  on the singleton delivery row.
- Delivery: the single delivery worker polls that flag (and the queue) every 5 s, processes batches
  of 200 with provider concurrency 20 and a 5 s per-call deadline, and clears the flag when drained.
  The provider tier is sized at 240 calls/min (the launch contract), which concurrency 20 × 5 s can
  saturate; mandatory handoff ≤ 30 s comes from the 5 s poll, not from an in-process signal.
- Budget check: 1,000 email deliveries / 240 calls per minute = 250 s ≤ 300 s. If a run has more
  than 1,200 email-eligible alerts it is split across two runs and the excess is reported, rather
  than exceeding the provider limit.
- Preferences: indexed `(user_id, type, channel)`; one row per pair.
- Feed: keyset on `(user_id, created_at desc, id)`.

| Operation | Budget | Element |
|---|---|---|
| Submit | p95 ≤ 100 ms | Indexed dedupe + insert |
| Preferences | p95 ≤ 150 ms | Indexed read/write |
| Delivery 1,000 | ≤ 5 min | 200/batch |
| Feed | p95 ≤ 200 ms | Keyset |
| Mandatory handoff | p95 ≤ 30 s | Wake on submit |

NFR references: NFR2.1–NFR2.5.
