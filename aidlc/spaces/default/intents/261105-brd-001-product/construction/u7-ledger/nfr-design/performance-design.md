# Performance Design — u7-ledger

Inputs: `nfr-requirements/performance-requirements.md`, `functional-design/functional-spec.md`.

- P&L: a stored `pnl_snapshots` total per user is served only when `computed_at` is later than the
  user's last ledger change (transaction or correction); otherwise it is recomputed before serving.
  Per-asset figures are computed on demand from open lots for the C7 response.
- Lots: `(user_id, asset_id, acquired_at)` index; FIFO consumption is one ordered scan per asset.
- Transaction list: keyset pagination on `(user_id, executed_at desc, id)`; one page of 50.
- Corrections: rebuild only the affected asset's lots, not the whole user.
- Reconcile: batched by transaction volume (≤ 5,000 transactions per batch), ordered by user hash,
  one run, so the 10-minute budget holds at the 50k-per-user maximum.

| Operation | Budget | Element |
|---|---|---|
| getPnl | p95 ≤ 400 ms | Snapshot or lot scan |
| List page | p95 ≤ 200 ms | Keyset index |
| Recorded trade | p95 ≤ 500 ms | Insert + affected asset |
| Correction | p95 ≤ 800 ms | Affected-asset rebuild |
| Reconcile 50k | ≤ 10 min | 500-user batches |

NFR references: NFR2.1–NFR2.6.
