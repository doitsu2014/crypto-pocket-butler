# BRD-006 — Portfolio Management

Draft v1.0 · Product · 2026-11-01. How users create portfolios, link accounts, set targets, and configure guardrails. A **portfolio** is the unit of valuation, drift, snapshots, and orders.

**In scope:** create/edit/delete portfolios · default designation · link/unlink accounts · target allocation per asset · guardrails · ownership isolation.
**Out of scope:** shared/multi-user portfolios · model portfolios/templates/automatic order generation · tax-aware rules · attribution/benchmarks · non-USD currencies.

## Portfolio model

| Element | Rule |
|---------|------|
| Identity | Name + optional description; names unique per user |
| Owner | Exactly one user |
| Linked accounts | Zero or more of the user's accounts; **each account links to one portfolio at a time** |
| Target allocation | Optional asset → percentage map; totals exactly 100%; unlisted assets get no target |
| Guardrails | Max deviation (indicative 5pp), min trade size, allowed assets (Should), max trade size (Should), stablecoin minimum (TBD) |
| Default flag | Exactly one default per user |
| State | Active or archived |

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-PORT-01 | Create a portfolio with name and optional description. | Must |
| FR-PORT-02 | Names unique per user; duplicates rejected. | Must |
| FR-PORT-03 | The first portfolio becomes the default. | Must |
| FR-PORT-04 | Redesignate the default; exactly one default per user. | Must |
| FR-PORT-05 | Edit name and description. | Must |
| FR-PORT-06 | Delete with confirmation, without deleting linked accounts. | Must |
| FR-PORT-07 | Link one or more own accounts. | Must |
| FR-PORT-08 | Reject linking another user's account. | Must |
| FR-PORT-09 | Reject duplicate links and links of an account already linked to another portfolio. | Must |
| FR-PORT-10 | Unlink an account. | Must |
| FR-PORT-11 | Define a target allocation with percentages per asset. | Must |
| FR-PORT-12 | Validate targets total 100% before saving. | Must |
| FR-PORT-13 | Set and edit guardrails (max deviation, min trade size). | Must |
| FR-PORT-14 | Show linked accounts, current value, and drift per portfolio. | Must |
| FR-PORT-15 | Archive a portfolio without deleting history. | Could |
| FR-PORT-16 | Duplicate a portfolio's configuration. | Could |
| FR-PORT-17 | Prevent access to portfolios the user doesn't own. | Must |

## Business rules

| ID | Rule |
|----|------|
| BR-PORT-01 | Names unique per user (case-insensitive recommended). |
| BR-PORT-02 | Exactly one default per user once one exists. |
| BR-PORT-03 | Only the owner's accounts may be linked. |
| BR-PORT-04 | A portfolio links multiple accounts; an account links to **one** portfolio at a time — re-linking elsewhere is rejected. |
| BR-PORT-05 | Targets sum to exactly 100%. |
| BR-PORT-06 | Guardrail deviation is measured in percentage points against target. |
| BR-PORT-07 | Deleting a portfolio removes its snapshots/allocations/orders, never its accounts. |
| BR-PORT-08 | An empty portfolio is valid and shows zero value. |
| BR-PORT-09 | Inactive accounts don't contribute to value or allocation. |
| BR-PORT-10 | No target allocation → drift is not calculated. |
| BR-PORT-11 | Guardrails may be stricter than defaults; disabling them is explicit and recorded. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-PORT-01 | A new user's first portfolio is marked default. |
| AC-PORT-02 | Creating a portfolio named "Core" when "Core" exists is rejected. |
| AC-PORT-03 | Designating a second portfolio as default leaves exactly one default. |
| AC-PORT-04 | Linking another user's account is rejected. |
| AC-PORT-05 | Re-linking an account (same or different portfolio) is rejected with a clear message. |
| AC-PORT-06 | Targets of 40/30/20/10 are accepted. |
| AC-PORT-07 | Targets totalling 99% fail validation with a clear message. |
| AC-PORT-08 | A portfolio with no accounts shows zero value without error. |
| AC-PORT-09 | Inactive linked accounts are excluded from valuation. |
| AC-PORT-10 | Deleting a portfolio leaves its accounts and other portfolios intact. |

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Protect the default portfolio from deletion, or auto-promote another? | Product |
| OQ-002 | Default guardrail values? | Product |
| OQ-003 | Explicit "other/unallocated" target bucket? | Product |
| OQ-004 | Archiving at launch? | Product |
| OQ-005 | Target ranges (min/max) instead of single percentages? | Product |
