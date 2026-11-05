# BRD-008 — Orders & Trades

Draft v2.2 · Product · 2026-11-04. On the portfolio view the user sees valuation and allocation, clicks **Buy** or **Sell** on an asset, and an **order intent** is generated. When the order is executed on chain or on an exchange it becomes a **trade**, and the trade's effect on the portfolio allocation is reflected **immediately**. v2.2 clarifies intent-versus-execution, reconciliation, shared-account effects, and deep links.

**In scope:** creating buy/sell order intents from the portfolio view · order lifecycle (pending → filled / cancelled) · trades as execution records · immediate allocation effect of trades · reconciliation against holdings sync · shared-account trade effects · order and trade history · deep links to the user's venue.
**Out of scope:** drift-based or automatic order generation · the product placing orders on exchanges or chains in the MVP · partial fills at launch · DCA scheduling · derivatives/leverage.

## Model

**Order** = portfolio, asset, side (buy/sell), quantity or base-currency amount, status, timestamps, optional note. The user's instruction. It never changes holdings by itself.

**Trade** = the executed transaction: asset, side, quantity, venue (chain or exchange), execution time, optional external reference. The order's result once executed — and the thing that moves the account.

- **Order lifecycle:** `pending` → `filled` (by a trade) or `cancelled`. A cancelled order never trades.
- **Immediate effect:** a trade changes the underlying account's quantity immediately; every portfolio containing that account reflects the change. It does not wait for the next scheduled holdings refresh.
- **Truth:** holdings sync (BRD-004) remains the source of truth; trades give the immediate view and reconcile against the next sync.
- **Reconciliation states:** `pending_reconciliation`, `matched`, `mismatched`, `stale`. A trade not matched within the configured window is flagged, never silently discarded.
- **Shared accounts:** a trade in an account shared by multiple portfolios affects all of them; the user is warned before recording.
- One order → one trade in the MVP (partial fills are post-MVP).

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-ORD-01 | From the portfolio view, create a buy or sell order intent for an asset with a quantity or base-currency amount. | Must |
| FR-ORD-02 | A new order starts `pending` with portfolio, asset, side, amount, and timestamps. | Must |
| FR-ORD-03 | The user can cancel a pending order. | Must |
| FR-ORD-04 | The user can record a trade that fills an order: venue (chain or exchange), quantity, execution time, and optional external reference. | Must |
| FR-ORD-05 | When a trade is recorded, the portfolio allocation reflects its effect immediately. | Must |
| FR-ORD-06 | View a portfolio's order and trade history with statuses, newest first. | Must |
| FR-ORD-07 | Orders are user-initiated only; the system never generates orders or trades automatically in the MVP. | Must |
| FR-ORD-08 | Before creating an order, show its projected effect on allocation. | Must |
| FR-ORD-09 | Validate order amounts (positive; sells within the account's available quantity across all portfolios). | Must |
| FR-ORD-10 | Audit order creation, cancellation, and trade recording (actor, time, target). | Must |
| FR-ORD-11 | Show a trade's reconciliation state and the next sync's result. | Must |
| FR-ORD-12 | Flag a trade that does not match the next sync within the configured window. | Must |
| FR-ORD-13 | Detect duplicate trade recording by external reference or matching fields. | Must |
| FR-ORD-14 | Offer a deep link to the user's exchange or wallet for execution. | Should |
| FR-ORD-15 | Support partial fills. | Could |
| FR-ORD-16 | Place orders directly through exchange APIs or on-chain signing. | Could (post-MVP) |
| FR-ORD-17 | Warn when a trade affects an account shared by other portfolios. | Must |
| FR-ORD-18 | Record trade quantity, unit price where known, fees, and base-currency value. | Must |
| FR-ORD-19 | Export order and trade history to CSV. | Should |
| FR-ORD-20 | Never present a deep link or preview as execution by the product. | Must |

## Business rules

| ID | Rule |
|----|------|
| BR-ORD-01 | Orders are user-initiated only — no drift-based generation. |
| BR-ORD-02 | An order is an instruction; only its trade changes the account. A pending or cancelled order never affects allocation. |
| BR-ORD-03 | A trade's effect is immediate; holdings sync reconciles it. |
| BR-ORD-04 | An order and its trade belong to exactly one portfolio and its owner; the trade changes the underlying account. |
| BR-ORD-05 | Sells are bounded by the account's available quantity across all portfolios, minus pending sell commitments. |
| BR-ORD-06 | The product never places orders in the MVP; exchange credentials stay read-only. |
| BR-ORD-07 | Guardrails (BRD-006) constrain order creation: minimum and maximum order size, allowed assets. |
| BR-ORD-08 | Cancelling an order or recording a trade is a recorded decision; statuses never change silently. |
| BR-ORD-09 | Order and trade records are reproducible: created state, decision, and execution facts are all retained. |
| BR-ORD-10 | A trade in a shared account affects every portfolio containing it; the user is warned. |
| BR-ORD-11 | Duplicate trades are rejected or merged; the same external reference cannot create two trades. |
| BR-ORD-12 | An unmatched trade is visible as `mismatched`; it is never silently deleted. |
| BR-ORD-13 | A deep link is a convenience, not execution; the product states this clearly. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-ORD-01 | Clicking Buy on BTC with a base-currency amount creates a `pending` order visible in history. |
| AC-ORD-02 | Cancelling a pending order sets `cancelled`; no trade exists; allocation is unchanged. |
| AC-ORD-03 | Recording the trade that fills an order updates the portfolio allocation immediately — e.g., BTC 10% → 12% without waiting for the next refresh. |
| AC-ORD-04 | Order and trade history lists all records with statuses, newest first. |
| AC-ORD-05 | Drift beyond a threshold never creates an order by itself. |
| AC-ORD-06 | A sell exceeding the account's available quantity is rejected with a clear error. |
| AC-ORD-07 | Order create/cancel and trade recording events appear in the audit log. |
| AC-ORD-08 | A trade not matched by the next sync is shown as `mismatched`. |
| AC-ORD-09 | Recording the same external reference twice is rejected as a duplicate. |
| AC-ORD-10 | A trade in a shared account warns the user and updates every containing portfolio. |
| AC-ORD-11 | A deep link opens the venue without claiming the product executed the trade. |

## Notes

- Allocation and pricing data come from BRD-005/007; guardrails from BRD-006; transactions and cost basis from BRD-013.
- Post-MVP: direct order placement through exchange APIs and on-chain signing, partial fills, and sync-detected trades from activity outside the app — with allowlists, size caps, confirmations, and full audit (BRD-012).
- A manual trade journal is not a substitute for execution; the UX must say so.

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Trade recording: manual entry only, or detection via account sync in the MVP? | Product |
| OQ-002 | Should a recorded trade's immediate allocation effect persist as a snapshot? | Product |
| OQ-003 | Default guardrail values for min/max order size? | Product |
| OQ-004 | Should briefs list trades, orders, or both? | Product |
| OQ-005 | Reconciliation window before a trade is flagged `mismatched`? | Product/Operations |
| OQ-006 | Which venues support reliable deep links at launch? | Product |
