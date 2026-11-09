# Performance Requirements — u8-reporting

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`, and
`inception/requirements-analysis/requirements.md`.

| ID | Requirement | Target |
|---|---|---|
| NFR2.1 | Brief list | p95 ≤ 200 ms |
| NFR2.2 | Open one brief | p95 ≤ 300 ms |
| NFR2.3 | Generate one brief (C7 + C8 + compose) | p95 ≤ 2 s |
| NFR2.4 | Daily run for 10k users | ≤ 30 minutes: 10,000 × 2 s ÷ 32 concurrent (8 batches × 4 in flight) ≈ 625 s, plus C3 due-users ≤ 2 s |
| NFR2.5 | Email handoff to C9 | p95 ≤ 1 s after generation |
