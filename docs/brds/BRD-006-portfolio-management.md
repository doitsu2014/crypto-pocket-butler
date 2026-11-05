# BRD-006 — Portfolio Management

Draft v1.1 · Product · 2026-11-04. How users create portfolios, link accounts, set targets, and configure guardrails. A **portfolio** is the unit of valuation, drift, snapshots, orders, and reporting. v1.1 adds shared-account membership, target ranges, an unallocated bucket, templates, and manual rebalancing proposals.

**In scope:** create/edit/delete/archive portfolios · default designation · account membership (one account may appear in multiple portfolios) · target allocation (fixed or range) · unallocated bucket · guardrails · model portfolio templates · manual rebalancing proposal · ownership isolation · shared-account trade semantics.
**Out of scope:** automatic order generation or execution · tax-aware rules · attribution/benchmarks (BRD-020) · non-USD accounting (base currency handled by BRD-018) · sharing portfolios with other users (post-MVP).

## Portfolio model

| Element | Rule |
|---------|------|
| Identity | Name + optional description; names unique per user |
| Owner | Exactly one user |
| Members | Zero or more of the user's accounts; an account may be a member of multiple portfolios; the same account appears at most once per portfolio |
| Target allocation | Optional asset → percentage map; totals exactly 100% including an explicit `unallocated` bucket; each asset may have a tolerance band (min/max) around its single target |
| Guardrails | Max deviation (default 5pp), min trade size, allowed assets (Should), max trade size (Should), stablecoin minimum (Could) |
| Default flag | Exactly one default per user |
| State | Active or archived |
| Template | Optional model template used to seed targets; never generates orders |

### Shared-account semantics

A shared account is visible in every portfolio that contains it. The underlying holdings are counted once **within each portfolio**, so two portfolios may legitimately show the same asset. A trade changes the **account**, not a portfolio-specific copy; therefore every portfolio containing that account reflects the change immediately. Sell validation uses the account's available quantity minus quantities already committed by pending sell orders across all portfolios, preventing double-spend. A user who wants independent books must use separate accounts; the product explains this before sharing.

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-PORT-01 | Create a portfolio with name and optional description. | Must |
| FR-PORT-02 | Names unique per user; duplicates rejected. | Must |
| FR-PORT-03 | The first portfolio becomes the default. | Must |
| FR-PORT-04 | Redesignate the default; exactly one default per user. | Must |
| FR-PORT-05 | Edit name and description. | Must |
| FR-PORT-06 | Delete with confirmation, without deleting linked accounts. | Must |
| FR-PORT-07 | Add one or more own accounts as members. | Must |
| FR-PORT-08 | Reject adding another user's account. | Must |
| FR-PORT-09 | Reject duplicate membership of the same account in one portfolio. | Must |
| FR-PORT-10 | Remove an account from a portfolio. | Must |
| FR-PORT-11 | Define a target allocation with percentages per asset. | Must |
| FR-PORT-12 | Validate targets total 100% before saving, including `unallocated`. | Must |
| FR-PORT-13 | Set and edit guardrails (max deviation, min/max trade size, allowed assets). | Must |
| FR-PORT-14 | Show members, current value, drift, and shared-account overlap per portfolio. | Must |
| FR-PORT-15 | Archive a portfolio without deleting history. | Should |
| FR-PORT-16 | Duplicate a portfolio's configuration. | Could |
| FR-PORT-17 | Prevent access to portfolios the user doesn't own. | Must |
| FR-PORT-18 | Allow an account to be a member of multiple portfolios; warn that holdings and trades are shared. | Must |
| FR-PORT-19 | Show which portfolios share an account and the overlap in value. | Must |
| FR-PORT-20 | Support a tolerance band (min/max) around a single target weight, not an independent range allocation. | Should |
| FR-PORT-21 | Support an explicit `unallocated` target bucket. | Should |
| FR-PORT-22 | Offer model portfolio templates that seed targets without generating orders. | Should |
| FR-PORT-23 | Generate a manual rebalancing proposal with projected allocation and trade sizes; never place orders. | Should |
| FR-PORT-24 | Apply documented guardrail defaults when the user does not set them. | Must |
| FR-PORT-25 | Restore an archived portfolio without losing history. | Should |
| FR-PORT-26 | Support portfolio tags/views for grouping without duplicating holdings. | Could |
| FR-PORT-27 | Show a clear warning when a trade in a shared account affects other portfolios. | Must |

## Business rules

| ID | Rule |
|----|------|
| BR-PORT-01 | Names unique per user (case-insensitive recommended). |
| BR-PORT-02 | Exactly one default per user once one exists. |
| BR-PORT-03 | Only the owner's accounts may be members. |
| BR-PORT-04 | An account may be a member of multiple portfolios but appears at most once per portfolio. Shared holdings are counted once per portfolio, not once globally. |
| BR-PORT-05 | Targets sum to exactly 100%, including `unallocated`. |
| BR-PORT-06 | Guardrail deviation is measured in percentage points against target. |
| BR-PORT-07 | Deleting a portfolio removes its snapshots/allocations/orders, never its accounts or transactions. |
| BR-PORT-08 | An empty portfolio is valid and shows zero value. |
| BR-PORT-09 | Inactive accounts don't contribute to value or allocation. |
| BR-PORT-10 | No target allocation → drift is not calculated. |
| BR-PORT-11 | Guardrails may be stricter than defaults; disabling them is explicit and recorded. |
| BR-PORT-12 | A trade changes the shared account; every portfolio containing it reflects the change. |
| BR-PORT-13 | Sell validation is account-level across portfolios; a pending sell cannot be double-spent. |
| BR-PORT-14 | A template only seeds targets; it never creates orders or advice. |
| BR-PORT-15 | A rebalancing proposal is informational and user-initiated; it never executes. |
| BR-PORT-16 | A tolerance band is min ≤ target ≤ max; the single targets still sum to 100%. A range is not an independent allocation. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-PORT-01 | A new user's first portfolio is marked default. |
| AC-PORT-02 | Creating a portfolio named "Core" when "Core" exists is rejected. |
| AC-PORT-03 | Designating a second portfolio as default leaves exactly one default. |
| AC-PORT-04 | Adding another user's account is rejected. |
| AC-PORT-05 | Adding the same account twice to one portfolio is rejected with a clear message. |
| AC-PORT-06 | Targets of 40/30/20/10 are accepted. |
| AC-PORT-07 | Targets totalling 99% fail validation with a clear message. |
| AC-PORT-08 | A portfolio with no accounts shows zero value without error. |
| AC-PORT-09 | Inactive member accounts are excluded from valuation. |
| AC-PORT-10 | Deleting a portfolio leaves its accounts and transactions intact. |
| AC-PORT-11 | The same account in two portfolios appears once in each and is visibly marked as shared. |
| AC-PORT-12 | A trade in a shared account updates both portfolios and warns the user. |
| AC-PORT-13 | A pending sell in one portfolio reduces the quantity available to another portfolio's sell validation. |
| AC-PORT-14 | A tolerance band is accepted when min ≤ target ≤ max and the single targets total 100%. |
| AC-PORT-15 | A template seeds targets but creates no order. |
| AC-PORT-16 | A rebalancing proposal shows projected allocation and trade sizes without execution. |

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Resolved by D-004: shared-account membership is a launch requirement. | Product |
| OQ-002 | Protect the default portfolio from deletion, or auto-promote another? | Product |
| OQ-003 | Default guardrail values per plan? | Product |
| OQ-004 | Which target templates are safe without crossing into advice? | Product/Legal |
| OQ-005 | Are target ranges in scope for the first release? | Product |
| OQ-006 | How should a user separate independent books: separate accounts only, or a future book model? | Product |
