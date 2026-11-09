# Performance Requirements — u7-ledger

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`, and
`inception/requirements-analysis/requirements.md`.

| ID | Requirement | Target |
|---|---|---|
| NFR2.1 | `getPnl` for a user with ≤ 50k transactions | p95 ≤ 400 ms (cached snapshot ≤ 50 ms) |
| NFR2.2 | Transaction list, one page of 50 | p95 ≤ 200 ms |
| NFR2.3 | Recorded-trade write + P&L recompute | p95 ≤ 500 ms |
| NFR2.4 | Correction write + affected-lot rebuild | p95 ≤ 800 ms |
| NFR2.5 | Nightly reconcile over 50k transactions | ≤ 10 minutes |
| NFR2.6 | Cost-basis lot query for one asset | p95 ≤ 30 ms |

Per-asset P&L is computed from indexed lots; the stored total is served on the hot path.
