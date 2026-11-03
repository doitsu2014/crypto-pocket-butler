# BRD-008 — Orders & Trades

Draft v2.1 · Product · 2026-11-03. On the portfolio view the user sees valuation and allocation, clicks **Buy** or **Sell** on an asset, and an **order** is generated. When the order is executed on chain or on an exchange it becomes a **trade**, and the trade's effect on the portfolio allocation is reflected **immediately**. Orders and trades are different things: the order is the instruction, the trade is the execution. This replaces drift-based rebalancing recommendations (retired IDs in §Retired).

**In scope:** creating buy/sell orders from the portfolio view · order lifecycle (pending → filled / cancelled) · trades as the execution records · immediate allocation effect of trades · order and trade history.
**Out of scope:** drift-based or automatic order generation (recommendations removed) · the product placing orders on exchanges or chains in the MVP — exchange credentials stay read-only · DCA scheduling · derivatives/leverage.

## Model

**Order** = portfolio, asset, side (buy/sell), quantity or USD amount, status, timestamps, optional note. The user's instruction.

**Trade** = the executed transaction: asset, side, quantity, venue (chain or exchange), execution time. The order's result once executed — and the thing that moves the portfolio.

- **Order lifecycle:** `pending` → `filled` (by a trade) or `cancelled`. A cancelled order never trades.
- **Immediate effect:** a trade's quantity change is reflected in the portfolio allocation immediately — it does not wait for the next scheduled holdings refresh.
- **Truth:** holdings sync (BRD-004) remains the source of truth; trades give the immediate view and reconcile against the next sync.
- One order → one trade in the MVP (partial fills are post-MVP).

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-ORD-01 | From the portfolio view, create a buy or sell order for an asset with a quantity or USD amount. | Must |
| FR-ORD-02 | A new order starts `pending` with portfolio, asset, side, amount, and timestamps. | Must |
| FR-ORD-03 | The user can cancel a pending order. | Must |
| FR-ORD-04 | The user can record a trade that fills an order: venue (chain or exchange), quantity, and execution time. | Must |
| FR-ORD-05 | When a trade is recorded, the portfolio allocation reflects its effect immediately. | Must |
| FR-ORD-06 | View a portfolio's order and trade history with statuses, newest first. | Must |
| FR-ORD-07 | Orders are user-initiated only; the system never generates orders or trades automatically in the MVP. | Must |
| FR-ORD-08 | Before creating an order, show its projected effect on allocation. | Should |
| FR-ORD-09 | Validate order amounts (positive; sells within the portfolio's current quantity of the asset). | Must |
| FR-ORD-10 | Audit order creation, cancellation, and trade recording (actor, time, target). | Must |

## Business rules

| ID | Rule |
|----|------|
| BR-ORD-01 | Orders are user-initiated only — no drift-based generation (recommendations retired). |
| BR-ORD-02 | An order is an instruction; only its trade changes the portfolio. A pending or cancelled order never affects allocation. |
| BR-ORD-03 | A trade's effect on allocation is immediate; holdings sync reconciles it. |
| BR-ORD-04 | An order and its trade belong to exactly one portfolio (and its owner). |
| BR-ORD-05 | Sells are bounded by the portfolio's current quantity of the asset. |
| BR-ORD-06 | The product never places orders in the MVP; exchange credentials stay read-only. |
| BR-ORD-07 | Guardrails (BRD-006) constrain order creation: minimum and maximum order size, allowed assets. |
| BR-ORD-08 | Cancelling an order or recording a trade is a recorded decision; statuses never change silently. |
| BR-ORD-09 | Order and trade records are reproducible: created state, decision, and execution facts are all retained. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-ORD-01 | Clicking Buy on BTC with a USD amount creates a `pending` order visible in history. |
| AC-ORD-02 | Cancelling a pending order sets `cancelled`; no trade exists; allocation is unchanged. |
| AC-ORD-03 | Recording the trade that fills an order updates the portfolio allocation immediately — e.g. BTC 10% → 12% without waiting for the next refresh. |
| AC-ORD-04 | Order and trade history lists all records with statuses, newest first. |
| AC-ORD-05 | Drift beyond a threshold never creates an order by itself. |
| AC-ORD-06 | A sell exceeding the portfolio's quantity of the asset is rejected with a clear error. |
| AC-ORD-07 | Order create/cancel and trade recording events appear in the audit log. |

## Notes

- Allocation and pricing data come from BRD-005/007; guardrails from BRD-006.
- Post-MVP: direct order placement through exchange APIs and on-chain signing, partial fills, and sync-detected trades from activity outside the app — with allowlists, size caps, confirmations, and full audit (see BRD-012).

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Trade recording: manual user entry only, or detection via account sync in the MVP? | Product |
| OQ-002 | Should a recorded trade's immediate allocation effect persist as a snapshot? | Product |
| OQ-003 | Default guardrail values for min/max order size? | Product |
| OQ-004 | Should briefs (BRD-009) list trades, orders, or both? | Product |

## Retired

| ID | Original | Retired | Reason |
|----|----------|---------|--------|
| FR-REB-01…15, BR-REB-01…11, AC-REB-01…09 | Drift-based rebalancing recommendations: generation, accept/reject decisions, rationale, risk levels, guardrail evaluation | v2.0, 2026-11-03 | Product pivot: users place buy/sell orders directly from the portfolio view; executed orders affect allocation immediately. Replaced by FR-ORD-01…10. |
