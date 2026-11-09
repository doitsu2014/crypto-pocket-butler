# Performance Requirements — u6-portfolio

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`, and
`inception/requirements-analysis/requirements.md`.

| ID | Requirement | Target |
|---|---|---|
| NFR2.1 | Portfolio read served from the latest snapshot when it is ≤ 60 s old | p95 ≤ 300 ms |
| NFR2.2 | Valuation computed fresh (C5 + C6) | p95 ≤ 800 ms |
| NFR2.3 | Snapshot history, one year | p95 ≤ 2 s |
| NFR2.4 | Target/intent preview | p95 ≤ 300 ms |
| NFR2.5 | Recorded trade (Ledger write + recompute) | p95 ≤ 900 ms |
| NFR2.6 | Valuation run for 10k portfolios | ≤ 15 minutes |
