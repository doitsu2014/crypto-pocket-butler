# BRD-009 — Reporting (In-App, Email & PDF Briefs)

Draft v1.2 · Product/Operations · 2026-11-04. A scheduled daily/weekly portfolio brief, composed deterministically and readable in-app, by email, or as PDF. Informational only — no trades, no advice. v1.2 adds P&L and performance, delivery channels, localization, timezone, and retention.

**In scope:** scheduled daily/weekly briefs · value, allocation, drift, P&L, performance, notable changes, recent orders · in-app reading with history · email delivery · PDF export · localized content · user-timezone scheduling · generation status, retry, freshness attribution · retention.
**Out of scope:** trade approval via briefs · client portals/white-label · tax/regulatory reports · AI-composed narratives · Slack/Notion publication at launch (post-MVP).

## Brief content

| Section | Content |
|---------|---------|
| Header | Portfolio name, period, local dates, generation timestamp |
| Summary | Total value, period change, one deterministic narrative paragraph |
| P&L | Net invested, realized/unrealized P&L, fees, income, data coverage |
| Performance | Period return, benchmark comparison where configured, methodology link |
| Allocation | Current vs target, material drift highlighted |
| Holdings | Top holdings with value and weight; unpriced/unmapped called out |
| Orders & trades | Recent orders with status (pending/filled/cancelled) and their trades |
| Data quality | Price timestamp, FX timestamp, last sync time, known gaps |
| Disclaimer | Informational only; no advice, no execution |

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-REP-01 | Generate a brief on a configured schedule. | Should |
| FR-REP-02 | Use the latest valuation, allocation, P&L, and performance data. | Must |
| FR-REP-03 | Include total value, allocation vs target, material drift. | Must |
| FR-REP-04 | Include recent orders or state none exist. | Should |
| FR-REP-05 | Include data freshness and unpriced/unmapped assets. | Must |
| FR-REP-06 | *(retired — see §Retired)* | — |
| FR-REP-07 | Record generation success/failure with reason. | Must |
| FR-REP-08 | Retry failures without creating duplicate briefs. | Must |
| FR-REP-09 | Composition never touches credentials, tokens, or secrets. | Must |
| FR-REP-10 | Include the informational disclaimer. | Must |
| FR-REP-11 | Support daily and weekly schedules. | Should |
| FR-REP-12 | Users can disable automated reporting. | Must |
| FR-REP-13 | Manual "generate now" for testing/recovery. | Could |
| FR-REP-14 | Multiple portfolios in one brief, or one brief each. | Should |
| FR-REP-15 | Deliver briefs by email and as PDF, in addition to in-app. | Should |
| FR-REP-16 | Briefs readable in-app by their owner, with history. | Must |
| FR-REP-17 | Include P&L and period performance when data supports it. | Must |
| FR-REP-18 | Localize the brief for the user's locale and format. | Must |
| FR-REP-19 | Schedule and date-group briefs in the user's timezone. | Must |
| FR-REP-20 | Send a notification when a brief is ready (BRD-015). | Should |
| FR-REP-21 | Export any brief to PDF; export data to CSV. | Should |
| FR-REP-22 | Apply the retention schedule and let the user delete briefs. | Must |
| FR-REP-23 | Meet accessibility requirements for the in-app and PDF brief. | Must |
| FR-REP-24 | Let users choose frequency, channels, and portfolio scope. | Must |
| FR-REP-25 | Publish to Slack/Notion or other external destinations. | Could |

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
| BR-REP-10 | Email delivery requires consent and honours unsubscribe. |
| BR-REP-11 | A brief is not a tax or regulatory report. |
| BR-REP-12 | Localized briefs preserve every disclaimer and data-quality label. |

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
| AC-REP-08 | A user receives the brief in-app and, if enabled, by email and PDF. |
| AC-REP-09 | P&L is shown when history is complete and labelled unavailable otherwise. |
| AC-REP-10 | The brief is localized and uses the user's timezone. |
| AC-REP-11 | The PDF is accessible and contains the same disclaimer. |
| AC-REP-12 | Deleting a brief does not delete portfolio data. |

## Notes

- Valuation comes from BRD-007, P&L from BRD-013, performance from BRD-020, and orders from BRD-008; timing is bounded by the refresh schedule (BRD-011).
- Narratives are deterministic templates from recorded data — no external agent or composition service.
- Email and PDF composition must never include secrets or full addresses.

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Default schedule, timezone, and frequency per plan? | Product |
| OQ-002 | One brief per portfolio or consolidated per user? | Product |
| OQ-003 | Retention for logs and briefs? | Compliance |
| OQ-004 | Review process for template language? | Product |
| OQ-005 | Which email provider and from-address? | Engineering |
| OQ-006 | Which external destinations, if any, are post-MVP priorities? | Product |

## Retired

| ID | Original | Retired | Reason |
|----|----------|---------|--------|
| FR-REP-06 | Publish to a per-user Notion destination. | v1.1, 2026-11-02 | External publication removed; replaced by FR-REP-16 |
| DR-REP-05 | Notion destination ID. | v1.1, 2026-11-02 | External publication removed |
| DR-REP-06 | Notion API token. | v1.1, 2026-11-02 | External publication removed |
