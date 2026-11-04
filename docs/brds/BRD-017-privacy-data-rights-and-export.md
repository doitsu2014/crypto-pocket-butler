# BRD-017 — Privacy, Data Rights & Export

Draft v1.0 · Security/Compliance · 2026-11-04. Global retail means multiple privacy regimes, not one. This BRD defines what data the product holds, why, where it lives, how long it stays, and how a user gets it back or has it erased.

**In scope:** data map and classification · lawful basis and consent · privacy notice · data processing agreements and subprocessors · data residency and cross-border transfers · retention schedule · user rights (access, rectification, erasure, portability, restriction, objection) · export · account deletion and anonymisation · marketing and analytics consent · cookies · breach notification · age and jurisdiction.
**Out of scope:** legal advice and jurisdiction-specific legal text drafting (external counsel) · employee/HR data · physical security · tax filing.

## Data map

| Category | Examples | Sensitivity | Purpose | Retention |
|----------|----------|-------------|---------|-----------|
| Identity | Provider subject, email, display name | Personal | Account, sign-in | Life of account + legal hold |
| Contact | Email, support messages | Personal | Service, support | Per support policy |
| Account labels | "Binance", "Ledger" | Personal | User experience | Life of account |
| Public addresses | EVM/Solana/Bitcoin addresses | Personal-linked | Holdings sync | Life of account |
| Exchange credentials | API key, secret, passphrase | **Secret** | Read-only sync | Until rotated/deleted; encrypted |
| Holdings and transactions | Quantities, prices, cost basis | Financial | Valuation, P&L | Per retention schedule |
| Portfolio and targets | Allocation, guardrails | Financial | Service | Life of account |
| Preferences | Locale, currency, notifications | Personal | Personalisation | Life of account |
| Billing | Plan, invoices, tax country | Financial | Payments, tax | Statutory period |
| Support | Tickets, attachments | Personal | Support | Per support policy |
| Analytics | Consent-aware events | Pseudonymous | Product improvement | Short, aggregate |
| Logs | Correlation IDs, errors | Operational | Security, debugging | Short; no secrets |

## Retention schedule (defaults; counsel confirms per jurisdiction)

| Data | Default | Trigger |
|------|---------|---------|
| Identity and profile | Account life | Deletion request or inactivity policy |
| Public addresses and holdings | Account life | Deletion request |
| Exchange credentials | Until rotated or account deleted | Deletion is immediate and crypto-shredded |
| Transactions and snapshots | 5 years | Tax/legal retention may extend |
| Audit logs | 12 months | Security/legal retention |
| Support records | 24 months | Legal hold |
| Billing records | Statutory period (typically 7–10 years) | Tax law |
| Analytics events | 14 months | Aggregate earlier |
| Logs | 30 days | Incident hold |

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-PRV-01 | Maintain a current data map, classification, and retention schedule. | Must |
| FR-PRV-02 | Publish a privacy notice in the user's locale, covering processors and transfers. | Must |
| FR-PRV-03 | Record lawful basis for each processing purpose. | Must |
| FR-PRV-04 | Obtain explicit consent for marketing and non-essential analytics before use. | Must |
| FR-PRV-05 | Provide a consent centre with withdraw and granular choices. | Must |
| FR-PRV-06 | Maintain a subprocessor list and a DPA with each processor. | Must |
| FR-PRV-07 | Document data residency and the transfer mechanism for each store. | Must |
| FR-PRV-08 | Let users download their data in a machine-readable format (JSON/CSV). | Must |
| FR-PRV-09 | Let users export holdings, transactions, snapshots, and P&L to CSV/PDF. | Must |
| FR-PRV-10 | Let users delete their account in-app; no email-only path. | Must |
| FR-PRV-11 | Define deletion cascade and anonymisation per entity; no orphaned user data. | Must |
| FR-PRV-12 | Crypto-shred exchange credentials on deletion; never retain plaintext. | Must |
| FR-PRV-13 | Honour access, rectification, erasure, restriction, objection, and portability requests within the statutory window. | Must |
| FR-PRV-14 | Provide a self-service privacy request log visible to the user. | Should |
| FR-PRV-15 | Notify affected users and authorities of a breach within the statutory window. | Must |
| FR-PRV-16 | Use consent-aware analytics; no PII in event payloads. | Must |
| FR-PRV-17 | Provide cookie/consent controls where required; no pre-ticked boxes. | Must |
| FR-PRV-18 | Apply age gating and jurisdiction restrictions before collecting data. | Must |
| FR-PRV-19 | Minimise data in logs, errors, and third-party processors. | Must |
| FR-PRV-20 | Retain data no longer than the schedule; automate deletion. | Must |
| FR-PRV-21 | Publish a security page with contact and disclosure policy. | Should |
| FR-PRV-22 | Support legal hold and regulatory retention overrides. | Should |

## Business rules

| ID | Rule |
|----|------|
| BR-PRV-01 | Collect only data needed for the stated purpose. |
| BR-PRV-02 | Secrets are encrypted at rest and never returned, logged, or exported. |
| BR-PRV-03 | Marketing and non-essential analytics require explicit opt-in. |
| BR-PRV-04 | Account deletion is available in-app and takes effect within the policy window. |
| BR-PRV-05 | Deletion removes or anonymises all user-linked data except legally retained records. |
| BR-PRV-06 | A subprocessor never receives more data than the purpose requires. |
| BR-PRV-07 | Cross-border transfers use an approved mechanism. |
| BR-PRV-08 | Retention is enforced automatically; indefinite retention is not allowed. |
| BR-PRV-09 | A privacy request is authenticated and audited. |
| BR-PRV-10 | Breach notification is not delayed by internal escalation. |
| BR-PRV-11 | Export includes the user's data, not another user's, and never includes credentials. |
| BR-PRV-12 | Legal hold overrides deletion only for the records it covers. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-PRV-01 | A user requests an export and receives their holdings, transactions, snapshots, and P&L without credentials. |
| AC-PRV-02 | A user deletes their account in-app; portfolios and credentials are removed or crypto-shredded. |
| AC-PRV-03 | An account deletion leaves no orphaned user-linked rows outside the legal-hold set. |
| AC-PRV-04 | Analytics events contain no email, address, or credential. |
| AC-PRV-05 | Marketing email is not sent without consent. |
| AC-PRV-06 | Withdrawing consent stops non-essential processing. |
| AC-PRV-07 | The privacy notice lists subprocessors and transfer mechanisms. |
| AC-PRV-08 | Retention jobs delete data beyond the schedule. |
| AC-PRV-09 | A breach triggers the documented notification path within the statutory window. |
| AC-PRV-10 | A jurisdiction-restricted user cannot create an account. |

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Which launch jurisdictions and which privacy regimes apply? | Compliance |
| OQ-002 | Data residency options and Supabase region strategy? | Engineering/Legal |
| OQ-003 | Which analytics tool is consent-compliant in all launch markets? | Legal/Product |
| OQ-004 | Exact retention periods per jurisdiction? | Compliance |
| OQ-005 | Is a DPO required in any launch market? | Legal |
| OQ-006 | What is the account-deletion grace period? | Product/Legal |
