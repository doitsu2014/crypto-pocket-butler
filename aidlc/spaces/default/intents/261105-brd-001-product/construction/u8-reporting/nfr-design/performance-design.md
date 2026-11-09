# Performance Design — u8-reporting

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`, `scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`, `tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

- List/open: indexed `(user_id, generated_at desc)` and `(id, user_id)`; no source calls on read.
- Generation: C3 due list (≤2 s), C8 portfolios + valuation (p95 800 ms), C7 P&L (p95 400 ms),
  compose and store; 8 batches × 4 in flight = 32 concurrent; 10k × 2 s ÷ 32 ≈ 625 s.
- Email: C9 submit after store; one attempt with C9's own retries.

| Operation | Budget | Element |
|---|---|---|
| List | p95 ≤ 200 ms | Indexed read |
| Open | p95 ≤ 300 ms | Indexed read |
| Generate | p95 ≤ 2 s | C8 + C7 + compose |
| Daily 10k | ≤ 30 min | 32 concurrent |
| Email handoff | p95 ≤ 1 s | C9 call |

NFR references: NFR2.1–NFR2.5.
