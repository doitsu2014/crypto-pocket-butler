# BRD-009 — Reporting & Agent Integration

| Field | Value |
|-------|-------|
| Document ID | BRD-009 |
| Version | 1.0 |
| Status | Draft — requires product and operations sign-off |
| Owner | Product / Operations |
| Last updated | 2026-11-01 |
| Related documents | BRD-005, BRD-007, BRD-008, BRD-012 |

## 1. Purpose

Define the automated reporting capability: the **OpenClaw agent** produces a periodic portfolio brief and publishes it to **Notion**. This removes manual reporting effort and gives users a durable, shareable narrative alongside the dashboard. The brief is informational; it does not execute trades or replace user judgement.

## 2. Scope

### In scope

- A scheduled daily and/or weekly portfolio brief.
- Content covering portfolio value, allocation, drift, notable changes, and pending recommendations.
- Publication to a user-configured Notion destination.
- Delivery status, failure handling, and retry.
- Clear attribution of data freshness and limitations.

### Out of scope

- Trade execution or approval through Notion.
- Two-way synchronisation from Notion into the product (for example, editing targets in Notion).
- Other reporting channels (email, Slack, PDF) — future.
- Client portals and white-label reporting — post-MVP.
- Tax or regulatory reports (post-MVP; see BRD-012).

## 3. Actors

| Actor | Interaction |
|-------|-------------|
| Portfolio Owner | Configures reporting destination and schedule; reads the brief |
| OpenClaw agent | Generates narrative content from portfolio data and recommendations |
| Notion | Receives and stores the published brief |
| System jobs | Trigger brief generation and delivery |
| Admin | Configures integration credentials and monitors delivery |

## 4. Brief content requirements

A brief must include, at minimum:

| Section | Content |
|---------|---------|
| Header | Portfolio name, reporting period, generation timestamp |
| Summary | Total value, change over the period (where history exists), and a one-paragraph narrative |
| Allocation | Current allocation versus target, with material drift highlighted |
| Holdings | Top holdings with value and weight; unpriced/unmapped assets called out |
| Recommendations | Pending or recent recommendations with status and key rationale |
| Data quality | Price timestamp, last sync time, and any known data gaps |
| Disclaimer | Informational-only statement; no investment advice and no execution |

## 5. Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-REP-01 | The system must generate a portfolio brief on a configured schedule. | Should |
| FR-REP-02 | The brief must use the latest portfolio valuation and allocation data. | Must |
| FR-REP-03 | The brief must include total value, allocation versus target, and material drift. | Must |
| FR-REP-04 | The brief must include pending recommendations or state that none exist. | Should |
| FR-REP-05 | The brief must include data-freshness information and any unpriced/unmapped assets. | Must |
| FR-REP-06 | The brief must be published to a Notion destination configured per user or per deployment. | Should |
| FR-REP-07 | The system must record whether delivery succeeded or failed, with a reason. | Must |
| FR-REP-08 | A failed delivery must be retryable without creating duplicate briefs. | Should |
| FR-REP-09 | The agent must not access or include exchange credentials, tokens, or other secrets. | Must |
| FR-REP-10 | The brief must include an informational-only disclaimer. | Must |
| FR-REP-11 | The system should support both daily and weekly schedules. | Should |
| FR-REP-12 | The user should be able to disable automated reporting. | Should |
| FR-REP-13 | The system should allow a manual "generate now" action for testing and recovery. | Could |
| FR-REP-14 | The system should support multiple portfolios in one brief or one brief per portfolio. | Could |
| FR-REP-15 | The system should publish to other channels (email, Slack, PDF). | Won't (MVP) |

## 6. Business rules

| ID | Rule |
|----|------|
| BR-REP-01 | The brief is informational and must never be presented as investment advice. |
| BR-REP-02 | The agent must not execute, schedule, or approve trades. |
| BR-REP-03 | The agent must not request or store exchange credentials or wallet secrets. |
| BR-REP-04 | Every brief must state the data timestamp and any known gaps; stale data must be labelled. |
| BR-REP-05 | A brief must not include another user's portfolio data. |
| BR-REP-06 | Retries must be idempotent: a repeated delivery for the same period must not create duplicate pages unless explicitly configured. |
| BR-REP-07 | Notion permissions must be limited to the configured destination; the product must not request broader workspace access than necessary. |
| BR-REP-08 | If Notion delivery fails, the portfolio data and recommendations must remain unaffected. |
| BR-REP-09 | Language in the brief must be factual and avoid prescriptive financial advice. |

## 7. Data requirements

| ID | Data element | Description | Sensitivity |
|----|--------------|-------------|-------------|
| DR-REP-01 | Report period | Daily or weekly window | Internal |
| DR-REP-02 | Portfolio valuation and allocation | Source data for the brief | Sensitive |
| DR-REP-03 | Drift and recommendations | Decision-support content | Sensitive |
| DR-REP-04 | Data freshness markers | Price and sync timestamps | Internal |
| DR-REP-05 | Notion destination ID | Target page/database | Internal |
| DR-REP-06 | Integration credential | Notion API token | Secret |
| DR-REP-07 | Delivery log | Status, timestamp, external reference, error | Internal |
| DR-REP-08 | Agent-generated narrative | Brief text | Sensitive |

## 8. Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-REP-01 | Given a scheduled brief, when it is generated, then it contains the required sections and the disclaimer. |
| AC-REP-02 | Given a portfolio with drift beyond the guardrail, when the brief is generated, then the drift is highlighted. |
| AC-REP-03 | Given a portfolio with unpriced assets, when the brief is generated, then those assets and the valuation exclusion are stated. |
| AC-REP-04 | Given successful publication, when the delivery log is inspected, then the status, time, and Notion reference are recorded. |
| AC-REP-05 | Given a Notion failure, when delivery is retried, then no duplicate brief is created for the same period. |
| AC-REP-06 | Given any generated brief, when inspected, then it contains no credentials or token values. |
| AC-REP-07 | Given a user who disabled reporting, when the schedule runs, then no brief is generated for that user. |

## 9. Assumptions, dependencies, constraints

- Portfolio valuation and recommendations exist per BRD-007 and BRD-008.
- Notion integration credentials are configured by an Admin and stored securely.
- The agent runs with least privilege and receives only the data required to compose the brief.
- The MVP supports one Notion destination per user or per deployment; multi-destination routing is future work.
- Delivery timing is bounded by the data refresh schedule (BRD-011).

## 10. Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Is Notion reporting a launch requirement or a fast-follow? | Product |
| OQ-002 | What is the default schedule (daily time, weekly day/time) and timezone? | Product |
| OQ-003 | One brief per portfolio or one consolidated brief per user? | Product |
| OQ-004 | Who owns the Notion workspace and how are destinations provisioned? | Operations |
| OQ-005 | How long are delivery logs and generated briefs retained? | Compliance |
| OQ-006 | What is the review process for agent-generated language? | Product |
