# Performance Design — u6-portfolio

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`, `scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`, `tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

- Read path: latest snapshot ≤ 60 s old is served directly; otherwise compute from C5 holdings + C6
  quotes/FX (C5 200 ms, C6 150 ms, one attempt).
- Intent/target preview: in-memory projection over the latest snapshot's lines (labelled with the
  snapshot asOf when it is older than 60 s); never a fresh C5+C6 compute, so the 300 ms budget holds.
- The valuation applies TradeOverlay deltas to C5 holdings on both the read and snapshot paths.
- Recorded trade: C7 write (300 ms) then recompute and overlay insert.
- Valuation run: 500 portfolios per batch, 4 in flight; snapshot written once per portfolio per hour.
- Indexes: `(portfolio_id, as_of desc)` snapshots, `(user_id)` portfolios, `(account_id)` membership.

| Operation | Budget | Element |
|---|---|---|
| Read (snapshot) | p95 ≤ 300 ms | Snapshot row |
| Read (fresh) | p95 ≤ 800 ms | C5 + C6 + compute |
| History 1 year | p95 ≤ 2 s | Indexed range |
| Preview | p95 ≤ 300 ms | In-memory |
| Recorded trade | p95 ≤ 900 ms | C7 + recompute |
| Run 10k | ≤ 15 min | 500/batch, 4 in flight |

NFR references: NFR2.1–NFR2.6.
