# Scalability Design — u6-portfolio

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`, `scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`, `tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

- API 2 replicas ceiling 4, pool 12; worker 1 replica, pool 8.
- 10k users, ≤ 20k portfolios, ≤ 50k accounts, ≤ 5 targets each.
- Snapshots hourly ⇒ ≤ 480k/day, two-year retention ≈ 350M rows; frozen inputs + lines stored once
  per snapshot in compressed JSONB (≈ 1–2 KB typical, bounded at 64 KB), pruned at two years.
- Snapshot uniqueness on `(portfolio_id, hour_bucket)` makes a retried run idempotent.
- Indexes: `(portfolio_id, as_of desc)`, `(portfolio_id)` membership, `(account_id)` membership,
  `(portfolio_id, asset_id)` overlays.
- Run: 500 portfolios per batch, 4 batches in flight (4 in-flight per batch), one leased run.

NFR references: NFR20.1–NFR20.6.
