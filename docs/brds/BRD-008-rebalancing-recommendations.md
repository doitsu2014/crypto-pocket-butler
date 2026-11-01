# BRD-008 — Rebalancing Recommendations

| Field | Value |
|-------|-------|
| Document ID | BRD-008 |
| Version | 1.0 |
| Status | Draft — requires product, risk, and compliance sign-off |
| Owner | Product / Risk |
| Last updated | 2026-11-01 |
| Related documents | BRD-006, BRD-007, BRD-009, BRD-012 |

## 1. Purpose

Define how the product turns allocation drift into understandable, safe, and auditable rebalancing recommendations, and how users review and record their decisions. The product is decision support only: it does not execute trades in the MVP.

## 2. Scope

### In scope

- Detecting drift relative to target allocation.
- Applying user guardrails to proposed trades.
- Generating recommendations with rationale, risk level, and suggested trades.
- Recording recommendation status and user decisions.
- Recommendation history.
- Human-in-the-loop approval and rejection.

### Out of scope

- Trade execution, order placement, or wallet signing (post-MVP; see section 9).
- Tax-aware rebalancing and tax-loss harvesting (post-MVP).
- DCA scheduling and smart/ML optimisation (post-MVP).
- Cross-account transfer instructions (the recommendation states intent; execution is manual).
- Derivatives or leverage recommendations.

## 3. Actors

| Actor | Interaction |
|-------|-------------|
| Portfolio Owner | Requests, reviews, accepts, or rejects recommendations |
| Admin | Same as owner for own portfolios; may trigger batch generation (future) |
| OpenClaw agent | May generate narrative summaries of recommendations (BRD-009) |
| System | May generate recommendations on a schedule (future) |

## 4. Recommendation model

A recommendation must contain:

| Element | Description |
|---------|-------------|
| Portfolio | The portfolio it applies to |
| Created at | Timestamp of generation |
| Status | `pending`, `accepted`, `rejected`, `executed`, or `expired` |
| Risk level | Qualitative indicator (for example low/medium/high) with the basis for the rating |
| Rationale | Plain-language explanation of why the recommendation exists and what it achieves |
| Current vs target | Per-asset actual weight, target weight, and drift |
| Proposed trades | Buy/sell asset, estimated amount, and optional venue or account context |
| Guardrail evaluation | Which guardrails were checked and whether any constrained the proposal |
| Data freshness | Snapshot/price timestamp used for the calculation |

## 5. Generation logic (business-level)

1. Verify the portfolio has a target allocation; if not, do not generate a drift-based recommendation.
2. Compute current allocation and drift per asset.
3. Identify assets whose absolute drift exceeds the maximum-deviation guardrail.
4. Determine the trades needed to return the portfolio toward target.
5. Apply constraints:
   - Do not propose trades below the minimum trade size.
   - Do not propose trades that violate allowed-asset restrictions.
   - Do not propose trades that breach maximum trade size or other configured caps.
   - Do not propose actions requiring unavailable assets or accounts.
6. Assign a risk level based on size, number of trades, asset volatility, and concentration.
7. Produce a rationale that explains the drift, the proposed response, and the trade-offs.
8. Persist the recommendation with status `pending`.
9. Present it to the user for a decision.

## 6. Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-REB-01 | A user must be able to request a recommendation for a portfolio that has a target allocation. | Must |
| FR-REB-02 | The system must calculate drift against the target and identify assets beyond the configured deviation threshold. | Must |
| FR-REB-03 | The system must propose buy/sell trades that move the portfolio toward target. | Must |
| FR-REB-04 | Proposed trades must respect minimum trade size and maximum deviation guardrails. | Must |
| FR-REB-05 | Each recommendation must include a risk level, rationale, proposed trades, and the data timestamp. | Must |
| FR-REB-06 | A user must be able to accept or reject a pending recommendation. | Must |
| FR-REB-07 | The system must record the decision, decision time, and acting user. | Must |
| FR-REB-08 | The user must be able to view a portfolio's recommendation history with statuses. | Must |
| FR-REB-09 | The system must not execute any trade automatically in the MVP. | Must |
| FR-REB-10 | The system should prevent more than one pending recommendation for the same portfolio from being actioned ambiguously; newer generations should supersede or be clearly distinguishable. | Should |
| FR-REB-11 | The system should explain when no recommendation is generated (for example, drift within tolerance or no target set). | Should |
| FR-REB-12 | The system should support configuring deviation thresholds, minimum trade size, allowed assets, and maximum trade size. | Should |
| FR-REB-13 | The system should generate a scheduled recommendation for portfolios with configured targets. | Could |
| FR-REB-14 | The system should support tax-aware and gas-aware recommendations. | Won't (MVP) |
| FR-REB-15 | The system should support one-click execution through a DEX aggregator with guardrails and audit. | Won't (MVP) |

## 7. Business rules

| ID | Rule |
|----|------|
| BR-REB-01 | No recommendation without a defined target allocation. |
| BR-REB-02 | Drift is measured in percentage points and must exceed the configured threshold to generate a trade proposal (indicative default: 5 points). |
| BR-REB-03 | The sum of proposed trade values must not exceed the portfolio value. |
| BR-REB-04 | A trade below the minimum trade size must be omitted, even if it would improve allocation. |
| BR-REB-05 | Recommendations must never instruct the user to use withdrawal permissions or to share credentials. |
| BR-REB-06 | A recommendation is advisory; the product must state that the user is responsible for execution and for any tax consequences. |
| BR-REB-07 | Accepting or rejecting is a recorded decision, not an execution. Status `executed` is reserved for a future execution capability. |
| BR-REB-08 | Guardrails cannot be bypassed by a recommendation; if constraints prevent a full rebalance, the recommendation must say so. |
| BR-REB-09 | Recommendations must be reproducible from the recorded inputs (allocation, target, guardrails, prices, timestamp). |
| BR-REB-10 | Recommendations must not include assets that are unpriced or unmapped without clearly flagging the limitation. |
| BR-REB-11 | Any future execution capability must require explicit confirmation, allowlists, maximum order sizes, and an immutable audit trail. |

## 8. Data requirements

| ID | Data element | Description | Sensitivity |
|----|--------------|-------------|-------------|
| DR-REB-01 | Recommendation ID and portfolio ID | Identity and scope | Internal |
| DR-REB-02 | Status and timestamps | Lifecycle and audit | Internal |
| DR-REB-03 | Risk level and rationale | User-facing decision support | Internal |
| DR-REB-04 | Current/target/drift per asset | Basis for the proposal | Sensitive |
| DR-REB-05 | Proposed trades | Asset, side, amount, rationale | Sensitive |
| DR-REB-06 | Guardrail evaluation | Constraints applied and outcomes | Internal |
| DR-REB-07 | Decision record | User, decision, timestamp | Audit |
| DR-REB-08 | Data snapshot reference | Prices/allocation used | Internal |

## 9. Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-REB-01 | Given a portfolio with no target allocation, when a recommendation is requested, then the system explains that a target is required and generates nothing. |
| AC-REB-02 | Given a target of 50% BTC and actual 58%, with a 5-point threshold, when a recommendation is generated, then a sell-BTC proposal is included. |
| AC-REB-03 | Given drift of 2 points and a 5-point threshold, when requested, then no trade is proposed and the reason is shown. |
| AC-REB-04 | Given a trade below the minimum trade size, when generating, then that trade is omitted and the rationale explains why. |
| AC-REB-05 | Given a pending recommendation, when the user rejects it, then its status becomes `rejected` with the user and timestamp recorded. |
| AC-REB-06 | Given a pending recommendation, when the user accepts it, then its status becomes `accepted` and no trade is executed. |
| AC-REB-07 | Given a recommendation, when the user opens it, then the risk level, rationale, trades, guardrail evaluation, and data timestamp are visible. |
| AC-REB-08 | Given a portfolio with historical recommendations, when history is viewed, then all are listed with their statuses in reverse chronological order. |
| AC-REB-09 | Given any recommendation, when displayed, then it includes an advisory disclaimer and no instruction to share credentials or enable withdrawals. |

## 10. Future execution requirements (post-MVP)

If execution is introduced, the following must be defined and approved before release:

- Supported venues and DEX aggregators.
- Allowlisted assets and chains.
- Maximum order size and daily volume limits.
- Cooldown and confirmation requirements.
- Full audit log of quotes, approvals, transactions, and outcomes.
- Failure handling, partial fills, and reconciliation.
- Legal and regulatory review.

## 11. Assumptions, dependencies, constraints

- Allocation and pricing data come from BRD-005 and BRD-007.
- Guardrails are configured per portfolio under BRD-006.
- The recommendation engine is deterministic for a given set of inputs.
- The product is not a registered investment adviser; outputs are informational.

## 12. Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | What default maximum-deviation and minimum-trade-size values should ship? | Product |
| OQ-002 | Should recommendations be generated automatically on drift, or only on user request at launch? | Product |
| OQ-003 | How should conflicting recommendations across portfolios be presented? | Product |
| OQ-004 | What risk-rating methodology will be used and disclosed? | Risk |
| OQ-005 | What disclaimer language is required in each jurisdiction? | Compliance |
| OQ-006 | Should accepting a recommendation produce an exportable trade checklist? | Product |
