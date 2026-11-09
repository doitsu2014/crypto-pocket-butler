# Logical Components — u7-ledger

Inputs: `nfr-requirements/*`, `functional-design/functional-spec.md`.

| Logical component | Responsibility | Scaling | Failure domain |
|---|---|---|---|
| Ledger API | C7 reads, correction and recorded-trade writes, privacy export/anonymise | 2 replicas | One task |
| Ingest | C5 pages, canonical dedupe, storage | Per run | A failed page retries |
| Lot engine | FIFO create/consume, transfer basis move, corrections rebuild | In-process | Affects one asset |
| P&L | Snapshot + per-asset compute, C5 holdings, C6 marks and FX, coverage | In-process | Read fails labelled |
| Reconcile worker | Nightly rebuild and coverage refresh, lease | 1 replica | Missed run visible |
| `ledger` schema | transactions, corrections, transfer pairs, lots, snapshots | Supabase | Service-owned |
| C5/C4/C6/C10 clients | Account rows and holdings, asset resolve, current prices and FX, audit | Per call | Bounded retry |

## Boundaries

- No credential or address stored; no cross-schema read.
- Transactions are append-only; corrections are rows.
- No message queue; reconcile is a C11 run.

## Blast radius

- A failed API task affects its replica.
- A bad correction affects one asset's lots until corrected again.
- A failed audit write is paged and retried; the correction remains.

NFR references: NFR20.1, NFR9.3, NFR1.4.
