# BRD-006 — Portfolio Management

| Field | Value |
|-------|-------|
| Document ID | BRD-006 |
| Version | 1.0 |
| Status | Draft — requires product sign-off |
| Owner | Product |
| Last updated | 2026-11-01 |
| Related documents | BRD-002, BRD-004, BRD-007, BRD-008 |

## 1. Purpose

Define how users create and manage portfolios, link accounts to them, set target allocations, and configure guardrails. A **portfolio** is the user's strategic grouping of accounts and the unit against which allocation, drift, snapshots, and recommendations are calculated.

## 2. Scope

### In scope

- Creating, editing, and deleting portfolios.
- Designating a default portfolio.
- Linking and unlinking accounts.
- Setting target allocation weights per asset.
- Configuring guardrails (deviation thresholds, minimum trade size, allowed assets).
- Portfolio-level ownership and isolation rules.

### Out of scope

- Shared or multi-user portfolios (post-MVP; see BRD-002).
- Model portfolios, templates, or automatic rebalancing.
- Tax-aware or jurisdiction-specific portfolio rules (post-MVP).
- Performance attribution and benchmarking (post-MVP).
- Multiple base currencies (MVP is USD).

## 3. Actors

| Actor | Interaction |
|-------|-------------|
| Portfolio Owner | Creates and configures own portfolios |
| Admin | Same as owner, for own portfolios |
| System jobs | Read portfolios to create snapshots and recommendations |

## 4. Portfolio model

A portfolio contains:

| Element | Description |
|---------|-------------|
| Identity | Name and optional description |
| Owner | Exactly one user in the MVP |
| Linked accounts | Zero or more accounts owned by the same user |
| Target allocation | Optional map of asset symbol → target percentage |
| Guardrails | Optional constraints applied to recommendations |
| Default flag | At most one default portfolio per user |
| State | Active or archived (archiving is optional in the MVP) |

### 4.1 Target allocation

- Expressed as percentages per asset symbol.
- Must total exactly 100% when defined.
- Assets not listed are treated as "other/unallocated" and are not given a target.
- Changes require no re-approval, but affect the next allocation calculation and recommendation.

### 4.2 Guardrails

| Guardrail | Meaning | MVP status |
|-----------|---------|------------|
| Maximum deviation | Drift threshold that triggers a recommendation/alert (indicative 5 percentage points) | In scope |
| Minimum trade size | Trades below this value are not recommended | In scope |
| Allowed assets | Restrict recommendations to an approved asset set | Should |
| Stablecoin minimum | Minimum stablecoin allocation to preserve liquidity | TBD |
| Futures/derivatives cap | Maximum derivatives exposure | TBD / post-MVP |
| Maximum trade size | Cap per recommended trade | Should |

## 5. Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-PORT-01 | A user must be able to create a portfolio with a name and optional description. | Must |
| FR-PORT-02 | Portfolio names must be unique per user; duplicate names must be rejected. | Must |
| FR-PORT-03 | The first portfolio a user creates must become their default portfolio. | Must |
| FR-PORT-04 | A user must be able to designate any of their portfolios as the default; exactly one default must exist per user. | Must |
| FR-PORT-05 | A user must be able to edit a portfolio's name and description. | Must |
| FR-PORT-06 | A user must be able to delete a portfolio, with confirmation, without deleting linked accounts. | Must |
| FR-PORT-07 | A user must be able to link one or more own accounts to a portfolio. | Must |
| FR-PORT-08 | The system must reject linking an account owned by another user. | Must |
| FR-PORT-09 | The system must reject linking the same account to the same portfolio twice. | Must |
| FR-PORT-10 | A user must be able to unlink an account from a portfolio. | Must |
| FR-PORT-11 | A user must be able to define a target allocation with percentages per asset. | Must |
| FR-PORT-12 | The system must validate that target allocation percentages total 100% before saving. | Must |
| FR-PORT-13 | A user must be able to set and edit guardrails including maximum deviation and minimum trade size. | Must |
| FR-PORT-14 | The system must show, for each portfolio, its linked accounts, current value, and drift versus target. | Must |
| FR-PORT-15 | The system should support archiving a portfolio without deleting its history. | Could |
| FR-PORT-16 | The system should support duplicating a portfolio's configuration into a new portfolio. | Could |
| FR-PORT-17 | The system must prevent a user from accessing portfolios they do not own. | Must |

## 6. Business rules

| ID | Rule |
|----|------|
| BR-PORT-01 | Portfolio names are unique per user (case-insensitive comparison recommended). |
| BR-PORT-02 | Exactly one default portfolio exists per user once they have at least one portfolio. |
| BR-PORT-03 | Only accounts owned by the portfolio owner may be linked. |
| BR-PORT-04 | An account may be linked to multiple portfolios belonging to the same user. |
| BR-PORT-05 | Target allocation weights, when defined, must sum to 100% (tolerance: exact, with clear rounding guidance in the UI). |
| BR-PORT-06 | Guardrail deviation is measured in percentage points against the target weight. |
| BR-PORT-07 | Deleting a portfolio removes its snapshots, allocations, and recommendations, but must not delete its accounts. |
| BR-PORT-08 | An empty portfolio is valid and shows zero value rather than an error. |
| BR-PORT-09 | Inactive accounts do not contribute to portfolio value or allocation. |
| BR-PORT-10 | A portfolio with no target allocation cannot generate drift-based recommendations until a target is defined. |
| BR-PORT-11 | Guardrails may be stricter than defaults but may not be disabled silently; disabling must be explicit and recorded. |

## 7. Data requirements

| ID | Data element | Description | Sensitivity |
|----|--------------|-------------|-------------|
| DR-PORT-01 | Portfolio ID | Stable identifier | Internal |
| DR-PORT-02 | Owner user ID | Owner | Internal |
| DR-PORT-03 | Name and description | User-facing identity | Personal |
| DR-PORT-04 | Default flag | Whether this is the user's default portfolio | Internal |
| DR-PORT-05 | Linked account IDs | Many-to-many relationship | Internal |
| DR-PORT-06 | Target allocation | Asset → percentage map | Sensitive |
| DR-PORT-07 | Guardrails | Thresholds and restrictions | Sensitive |
| DR-PORT-08 | Created/updated timestamps | Audit information | Internal |

## 8. Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-PORT-01 | Given a new user with no portfolios, when they create their first portfolio, then it is marked as default. |
| AC-PORT-02 | Given a user with a portfolio named "Core", when they create another named "Core", then the request is rejected. |
| AC-PORT-03 | Given a user with two portfolios, when they designate the second as default, then the first is no longer default and exactly one default exists. |
| AC-PORT-04 | Given an account owned by another user, when linking is attempted, then the request is rejected. |
| AC-PORT-05 | Given an account already linked to a portfolio, when linking again, then the duplicate is rejected. |
| AC-PORT-06 | Given target weights of 40%, 30%, 20%, and 10%, when saved, then the configuration is accepted. |
| AC-PORT-07 | Given target weights totalling 99%, when saved, then validation fails with a clear message. |
| AC-PORT-08 | Given a portfolio with no linked accounts, when viewed, then it shows zero value without error. |
| AC-PORT-09 | Given a portfolio with inactive linked accounts, when valued, then those accounts are excluded. |
| AC-PORT-10 | Given a deleted portfolio, when deletion completes, then its accounts still exist and the user's other portfolios are unaffected. |

## 9. Assumptions, dependencies, constraints

- Accounts are created and managed under BRD-004.
- Valuation and drift calculation rules are defined in BRD-007.
- Recommendation generation consumes target allocation and guardrails per BRD-008.
- The MVP does not support portfolio-level permissions or sharing.

## 10. Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Should the default portfolio be protected from deletion, or should another be promoted automatically? | Product |
| OQ-002 | What are the default guardrail values (maximum deviation, minimum trade size)? | Product |
| OQ-003 | Should "other/unallocated" be an explicit target bucket? | Product |
| OQ-004 | Is archiving required at launch? | Product |
| OQ-005 | Should target allocation support ranges (minimum/maximum) instead of a single percentage? | Product |
