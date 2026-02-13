# 10 — Core Split: Infrastructure Data vs Portfolios

You want to split the core into two major parts:

## Phase A — Infrastructure Data (foundation)

Purpose: maintain a clean, reusable **data layer** that everything else (accounts, portfolios, allocation) can build on.

### A1) Market reference data (cron-collected)

- Collect and store:
  - prices (per asset)
  - top 100 coins list (rank + metadata)
  - contract addresses per chain for these coins (EVM, etc.)
- This becomes the canonical dataset for:
  - symbol normalization
  - portfolio allocation and valuation

### A2) User → Accounts (data sources)

Accounts represent where the user has holdings.

- **EVM account**:
  - user provides address
  - user selects which chains are enabled (from supported chain list)
  - ingest stores **holdings + quantities only** (no pricing required at this stage)
- **OKX account**:
  - read-only connector
  - ingest stores **holdings + quantities only** (no pricing required at this stage)

Output of Infra phase:
- clean holdings data by account
- clean market reference data

## Phase B — Portfolios (constructed views)

Purpose: portfolios are a computed view based on selected accounts.

### B1) Construct portfolio allocation (manual action)

- Portfolio has selected accounts (wallets/exchanges)
- When user clicks **Construct**, system computes:
  - portfolio holdings aggregation
  - portfolio allocation (weights)
  - valuation using price reference dataset

- **Persist** the result as a stored Portfolio Allocation record (so UI can show last constructed at and snapshots can reference it).

### B2) Snapshots

Snapshots can be created:
- automatically **EOD**
- manually when user clicks **Construct snapshot**

Snapshot includes:
- total value (USD)
- composition (holdings + value)
- metadata (trigger type: EOD vs manual)
