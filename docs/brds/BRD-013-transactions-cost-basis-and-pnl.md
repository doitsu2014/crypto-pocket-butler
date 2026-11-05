# BRD-013 — Transactions, Cost Basis & P&L

Draft v1.0 · Product/Finance · 2026-11-04. The ledger behind the retail question: **how much did I put in, and what is it worth now?** Holdings snapshots answer "what do I hold"; this BRD answers "am I up or down, and why".

**In scope:** transaction ledger · import from exchange APIs, CSV, and manual entry · classification and deduplication · transfer pairing · cost basis (FIFO and average cost at launch) · net invested · realized/unrealized P&L · fees · base-currency and historical-FX conversion · reconciliation against synced holdings · tax-ready export (data only).
**Out of scope:** tax filing, tax advice, jurisdiction-specific tax calculation · tax-lot optimisation and specific-ID relief at launch · full on-chain transaction decoding at launch · NFT and DeFi position accounting · derivatives/margin · performance and benchmark analytics (BRD-020).

## Model

A **transaction** is one economic event in one account. Holdings (BRD-004) remain the source of truth for current quantity; the ledger explains how that quantity was acquired and what it cost.

| Field | Rule |
|-------|------|
| Identity | User, account, portfolio scope (where applicable), asset |
| Type | `buy`, `sell`, `deposit`, `withdrawal`, `internal_transfer`, `fee`, `reward`, `staking_income`, `airdrop`, `adjustment` |
| Quantity | Normalised decimal; positive; `sell`/`withdrawal` reduce the account |
| Unit price | Base-currency price per unit at execution, where known |
| Fiat value | Quantity × unit price, plus or minus fee; base currency |
| Fee | Separate field; never silently folded into price |
| Time | Execution timestamp with timezone; import timestamp kept separately |
| Venue | Exchange, chain, or `manual` |
| Source | `exchange_api`, `csv`, `manual`, `onchain` (post-MVP) |
| External ID | Exchange trade ID, transaction hash, or import row key |
| Status | `confirmed`, `pending`, `ignored`, `reconciled`, `mismatched` |
| Note | Optional user note; never contains credentials |

**Classification.** Deposits and withdrawals are external cash flows. Transfers between the user's own accounts are paired and are **not** P&L events. Rewards, staking income, and airdrops are income at receipt value; that value becomes their cost basis. Fees are always recorded.

**Cost basis.** FIFO is the launch default. Average cost is selectable per user. LIFO, HIFO, and specific-ID relief are post-MVP and require an audited recomputation when introduced.

**Definitions (base currency).**

| Metric | Definition |
|--------|------------|
| Net invested | External deposits − external withdrawals |
| Current cost basis | Remaining cost of assets still held |
| Unrealized P&L | Current market value − current cost basis |
| Realized P&L | Σ(proceeds − cost basis sold − sell fees) |
| Income | Rewards, staking, and airdrops at receipt value |
| Total P&L | Realized + unrealized + income − fees not already included |
| Simple return | Total P&L ÷ net invested, when net invested > 0 |

**Reconciliation.** For every asset and account, the ledger-derived quantity is compared with the latest synced holdings. `matched` means within the asset's display precision; `mismatched` is surfaced, never silently overwritten. The ledger is append-only; corrections are adjusting entries.

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-TXN-01 | Import transactions from the supported exchange APIs (BRD-004). | Must |
| FR-TXN-02 | Import CSV with a column-mapping preview and per-exchange templates. | Must |
| FR-TXN-03 | Add, edit, and delete transactions manually; corrections are recorded as adjusting entries, never silent edits. | Must |
| FR-TXN-04 | Deduplicate by source and external ID or transaction hash; imports are idempotent. | Must |
| FR-TXN-05 | Classify transactions into the model's types; user can correct a classification with an audit record. | Must |
| FR-TXN-06 | Pair transfers between the user's own accounts and exclude them from P&L. | Must |
| FR-TXN-07 | Compute remaining cost basis per asset under FIFO and average cost. | Must |
| FR-TXN-08 | Compute net invested, realized P&L, unrealized P&L, income, fees, and total P&L. | Must |
| FR-TXN-09 | Show P&L per asset, account, portfolio, and user, in the user's base currency. | Must |
| FR-TXN-10 | Convert using the historical FX rate at transaction time; show source and timestamp. | Must |
| FR-TXN-11 | Reconcile ledger quantity against synced holdings and show `matched`, `within tolerance`, `mismatched`, or `unavailable`. | Must |
| FR-TXN-12 | Show data coverage: percentage of current holdings with transaction history; label P&L as unavailable where history is missing. | Must |
| FR-TXN-13 | Import preview with per-row errors; partial imports succeed and unexpected rows are never silently dropped. | Must |
| FR-TXN-14 | Recompute cost basis when the method changes; show before/after and record actor and time. | Must |
| FR-TXN-15 | Export transactions and P&L to CSV and PDF. | Should |
| FR-TXN-16 | Provide a tax-ready export with jurisdiction-neutral columns; never calculate tax or give advice. | Should |
| FR-TXN-17 | Support fee handling as a separate field or embedded in price, configurable per import. | Must |
| FR-TXN-18 | Treat rewards, staking income, and airdrops as income at receipt value and use that value as cost basis. | Should |
| FR-TXN-19 | Flag oversells, negative quantities, and impossible timestamps; never clamp silently. | Must |
| FR-TXN-20 | Retain the ledger immutably for the retention period in BRD-017. | Must |
| FR-TXN-21 | Decode on-chain transactions for supported chains. | Could |
| FR-TXN-22 | Show tax-lot detail and lot-relief selection. | Could |

## Business rules

| ID | Rule |
|----|------|
| BR-TXN-01 | The ledger is append-only. Corrections are adjusting entries with actor, time, and reason. |
| BR-TXN-02 | Every transaction has an asset, quantity, type, source, and timestamp; fiat value is required where the source provides it. |
| BR-TXN-03 | Transfers between the user's own accounts are not P&L events. |
| BR-TXN-04 | Cost basis is never guessed. Missing history means P&L is `unavailable`, not zero. |
| BR-TXN-05 | The same external transaction imported twice is one transaction. |
| BR-TXN-06 | P&L is reported in the user's base currency; the FX rate and timestamp are disclosed. |
| BR-TXN-07 | Fees are always accounted for, whether separate or embedded. |
| BR-TXN-08 | Oversells and negative quantities are flagged for review, never silently corrected. |
| BR-TXN-09 | A cost-basis method change is an audited decision, not a display toggle. |
| BR-TXN-10 | A tax export is data, not tax advice or a tax filing. |
| BR-TXN-11 | Unpriced assets are excluded from P&L totals and labelled. |
| BR-TXN-12 | Reconciliation discrepancies are visible to the user; no silent overwrite of ledger or synced data. |
| BR-TXN-13 | The ledger never stores private keys, seed phrases, or exchange secrets. |
| BR-TXN-14 | Retention and deletion follow BRD-017; deletion is crypto-shredded or anonymised according to policy. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-TXN-01 | Importing 100 OKX trades produces 100 unique transactions; re-importing produces no duplicates. |
| AC-TXN-02 | Buy 1 BTC at $50,000 with a $10 fee; price $60,000 → unrealized P&L is $9,990 under FIFO. |
| AC-TXN-03 | Selling half the position computes realized P&L from the remaining lots and reduces cost basis correctly. |
| AC-TXN-04 | Moving BTC between two own wallets creates no realized P&L. |
| AC-TXN-05 | Missing pre-import history shows P&L as `unavailable` with a clear explanation. |
| AC-TXN-06 | Base currency EUR applies the transaction-date FX rate and shows its source and timestamp. |
| AC-TXN-07 | A ledger quantity that differs from synced holdings is shown as `mismatched`. |
| AC-TXN-08 | Changing FIFO to average cost recomputes P&L and leaves an audit record. |
| AC-TXN-09 | CSV export contains asset, type, quantity, price, fee, timestamp, source, and base-currency value. |
| AC-TXN-10 | An oversell is flagged and the user can correct it with an adjusting entry. |

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Resolved by D-007: FIFO default and average cost selectable at launch. | Product/Finance |
| OQ-002 | Which jurisdictions need specific-ID or HIFO at launch? | Compliance |
| OQ-003 | Scope of on-chain decoding at launch: native transfers only, or token transfers too? | Data |
| OQ-004 | Treatment of staking rewards, rebasing tokens, and airdrops across jurisdictions? | Finance/Compliance |
| OQ-005 | Which tax-export partners (Koinly/CoinTracker) and column schemas? | Product |
| OQ-006 | Historical FX source and licensing? | Data/Legal |
