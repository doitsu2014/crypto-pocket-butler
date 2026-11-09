# Scalability Design — u7-ledger

Inputs: `nfr-requirements/scalability-requirements.md`.

- API: 2 stateless replicas, ceiling 4; pool 12 each.
- Worker: 1 replica for reconcile; pool 8.
- Volume: ≤ 1 M transactions/month; ≤ 50k per user; ≤ 500k open lots.
- Indexes: `(user_id, executed_at desc, id)` for listing, `(user_id, asset_id, acquired_at)` for
  FIFO, unique `(canonical_key)` and `(external_key)`.
- Partitioning: `transactions` and `pnl_snapshots` partition by user hash at 24 M rows.
- Retention: two years then anonymise; snapshots two years.
- Reconcile: 500-user batches, one leased run.

NFR references: NFR20.1–NFR20.6.
