# BRD-009 — Reporting (In-App Briefs)

Draft v1.1 · Product/Operations · 2026-11-02. A scheduled daily/weekly portfolio brief, composed deterministically in-app and read in the SPA. Informational only — no trades, no advice.

> v1.1: external publication (OpenClaw agent, Notion) was removed; briefs are composed by the Reporting Service. Retired IDs are tracked in §Retired.

**In scope:** scheduled daily/weekly briefs · value, allocation, drift, notable changes, recent orders · in-app reading with history · generation status, retry, freshness attribution.
**Out of scope:** external channels (email/Slack/PDF/Notion) · trade approval via briefs · client portals/white-label · tax/regulatory reports.

## Brief content

| Section | Content |
|---------|---------|
| Header | Portfolio name, period, generation timestamp |
| Summary | Total value, period change (where history exists), one deterministic narrative paragraph |
| Allocation | Current vs target, material drift highlighted |
| Holdings | Top holdings with value and weight; unpriced/unmapped called out |
| Orders & trades | Recent orders with status (pending/filled/cancelled) and their trades |
| Data quality | Price timestamp, last sync time, known gaps |
| Disclaimer | Informational only; no advice, no execution |

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-REP-01 | Generate a brief on a configured schedule. | Should |
| FR-REP-02 | Use the latest valuation and allocation data. | Must |
| FR-REP-03 | Include total value, allocation vs target, material drift. | Must |
| FR-REP-04 | Include recent orders or state none exist. | Should |
| FR-REP-05 | Include data freshness and unpriced/unmapped assets. | Must |
| FR-REP-06 | *(retired — see §Retired)* | — |
| FR-REP-07 | Record generation success/failure with reason. | Must |
| FR-REP-08 | Retry failures without creating duplicate briefs. | Should |
| FR-REP-09 | Composition never touches credentials, tokens, or secrets. | Must |
| FR-REP-10 | Include the informational disclaimer. | Must |
| FR-REP-11 | Support daily and weekly schedules. | Should |
| FR-REP-12 | Users can disable automated reporting. | Should |
| FR-REP-13 | Manual "generate now" for testing/recovery. | Could |
| FR-REP-14 | Multiple portfolios in one brief, or one brief each. | Could |
| FR-REP-15 | Publish to external channels. | Won't (MVP) |
| FR-REP-16 | Briefs readable in-app by their owner, with history. | Must |

## Business rules

| ID | Rule |
|----|------|
| BR-REP-01 | Informational, never investment advice. |
| BR-REP-02 | Briefs never execute, schedule, or approve trades. |
| BR-REP-03 | Generation never requests or stores credentials or secrets. |
| BR-REP-04 | Every brief states data timestamps and gaps; stale data is labelled. |
| BR-REP-05 | A brief never includes another user's data. |
| BR-REP-06 | Generation is idempotent per owner, portfolio, and period. |
| BR-REP-07 | Briefs follow the same owner-only access rules as portfolio data. |
| BR-REP-08 | A failed generation leaves portfolio data and orders unaffected. |
| BR-REP-09 | Language is factual, not prescriptive. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-REP-01 | A generated brief contains the required sections and disclaimer. |
| AC-REP-02 | Drift beyond a guardrail is highlighted in the brief. |
| AC-REP-03 | Unpriced assets and their valuation exclusion are stated. |
| AC-REP-04 | The generation log records status and time. |
| AC-REP-05 | A retried failure creates no duplicate brief for the period. |
| AC-REP-06 | No brief contains credentials or token values. |
| AC-REP-07 | A user who disabled reporting gets no scheduled brief. |

## Notes

- Valuation comes from BRD-007 and orders from BRD-008; timing is bounded by the refresh schedule (BRD-011).
- Narratives are deterministic templates from recorded data — no external agent or composition service.

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | In-app-only reporting sufficient for launch? | Product |
| OQ-002 | Default schedule and timezone? | Product |
| OQ-003 | One brief per portfolio or consolidated per user? | Product |
| OQ-004 | Retention for logs and briefs? | Compliance |
| OQ-005 | Review process for template language? | Product |

## Retired

| ID | Original | Retired | Reason |
|----|----------|---------|--------|
| FR-REP-06 | Publish to a per-user Notion destination. | v1.1, 2026-11-02 | External publication removed; replaced by FR-REP-16 |
| DR-REP-05 | Notion destination ID. | v1.1, 2026-11-02 | External publication removed |
| DR-REP-06 | Notion API token. | v1.1, 2026-11-02 | External publication removed |
