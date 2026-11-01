# BRD-012 — Security, Compliance & Non-Functional Requirements

| Field | Value |
|-------|-------|
| Document ID | BRD-012 |
| Version | 1.0 |
| Status | Draft — requires security and compliance sign-off |
| Owner | Security / Compliance |
| Last updated | 2026-11-01 |
| Related documents | BRD-002, BRD-003, BRD-004, BRD-008, BRD-009, BRD-011 |

## 1. Purpose

Define the security, privacy, compliance, and quality obligations that apply across the product. These requirements are not optional features: they constrain how every other capability is delivered. Where a requirement is post-MVP, the MVP must still avoid decisions that make it unreachable.

## 2. Scope

### In scope

- Authentication and authorisation obligations (detail in BRD-002/003).
- Protection of exchange credentials and user data.
- Read-only and least-privilege guarantees.
- Audit logging and traceability.
- Data retention, export, and deletion.
- Availability, performance, accuracy, scalability, and observability targets.
- Compliance capabilities planned for post-MVP.

### Out of scope

- Physical security of cloud or office infrastructure.
- Employment and HR compliance.
- Jurisdiction-specific legal advice; external counsel must validate obligations.
- Smart-contract security (the product does not deploy contracts in the MVP).

## 3. Security principles

| ID | Principle | Implication |
|----|-----------|-------------|
| SP-01 | Read-only by default | No withdrawal, transfer, or trading permission is required or used in the MVP |
| SP-02 | Least privilege | Users, admins, jobs, and integrations receive only the access needed |
| SP-03 | No secret exposure | Credentials are encrypted at rest, masked in UI, and never returned or logged |
| SP-04 | Defence in depth | AuthN, AuthZ, input validation, rate limiting, and encryption are layered |
| SP-05 | Deny by default | Unlisted capabilities are denied until explicitly granted |
| SP-06 | Traceability | Security-sensitive actions are recorded with actor, time, and outcome |
| SP-07 | Privacy by design | Only data needed for the stated purpose is collected and retained |
| SP-08 | Safe failure | Failures must not expose data or silently corrupt valuations |

## 4. Security requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| NFR-SEC-01 | All production traffic must use TLS; plaintext transport must be rejected. | Must |
| NFR-SEC-02 | Exchange API keys, secrets, and passphrases must be encrypted at rest with a managed key. | Must |
| NFR-SEC-03 | Credential fields must never appear in API responses, logs, error messages, or analytics. | Must |
| NFR-SEC-04 | The product must not request or store wallet private keys or seed phrases. | Must |
| NFR-SEC-05 | Exchange integrations must use read-only permissions; the product must not enable withdrawals. | Must |
| NFR-SEC-06 | All user-owned resources must enforce ownership checks on read and write. | Must |
| NFR-SEC-07 | Administrative capabilities must require an explicit admin role. | Must |
| NFR-SEC-08 | Authentication tokens must be validated for signature, issuer, audience, and expiry. | Must |
| NFR-SEC-09 | Session cookies must be HTTP-only and secure in production. | Must |
| NFR-SEC-10 | Public endpoints must be rate-limited per user/IP; authentication and manual job triggers must be limited more strictly. | Must |
| NFR-SEC-11 | Security-sensitive events must be logged without recording secrets. | Must |
| NFR-SEC-12 | Dependencies and container images must be scanned for known vulnerabilities before release. | Should |
| NFR-SEC-13 | Secrets must be supplied through environment/secret management, never committed to source control. | Must |
| NFR-SEC-14 | Backups must be encrypted and access-controlled. | Must |
| NFR-SEC-15 | Multi-factor authentication must be available through the identity provider. | Should |

## 5. Privacy and data protection

| ID | Requirement | Priority |
|----|-------------|----------|
| NFR-PRV-01 | Collect only the personal data required to deliver the service (identity claims, account labels, wallet addresses). | Must |
| NFR-PRV-02 | Document the purpose and retention period for each category of personal data. | Should |
| NFR-PRV-03 | Provide a way to export a user's data on request. | Should |
| NFR-PRV-04 | Provide a way to delete or anonymise a user's data on request, subject to legal retention. | Should |
| NFR-PRV-05 | Do not sell or share personal data with third parties beyond the processors required to run the service. | Must |
| NFR-PRV-06 | Disclose which external providers receive data (exchanges, RPC providers, price providers, Notion). | Should |
| NFR-PRV-07 | Apply data minimisation to logs and analytics. | Must |

## 6. Audit logging

The following events are security- or financially material and must be recorded:

- Successful and failed sign-ins and sign-outs.
- Creation, update, deactivation, or deletion of accounts, portfolios, targets, and guardrails.
- Credential changes (without recording the credential).
- Sync execution outcomes per account.
- Recommendation generation, acceptance, and rejection.
- Administrative chain/token changes.
- Manual job triggers.
- Reporting delivery outcomes.

Each event should record: actor (or system), action, target resource, timestamp, source context, and outcome. Audit records must be append-only and protected from modification by application users.

## 7. Compliance roadmap

| Capability | Description | MVP status |
|------------|-------------|------------|
| Transaction history export | CSV/PDF export of holdings and activity | Post-MVP |
| Capital-gains reporting | FIFO, LIFO, HIFO methods; country-specific reports | Post-MVP |
| Income and staking tracking | Classification of staking and other income | Post-MVP |
| Sanctioned-address screening | OFAC and comparable list screening for wallet addresses | Post-MVP |
| KYC/AML | Identity verification and monitoring where required | Post-MVP |
| Geographic restrictions | Jurisdiction-based feature and access restrictions | Post-MVP |
| Record retention | Defined retention and legal-hold policy | Must define before launch |

> Launch in a jurisdiction without the required compliance controls requires explicit legal sign-off. Advisory outputs must be reviewed against local investment-advice rules.

## 8. Non-functional requirements

### 8.1 Accuracy

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-ACC-01 | Portfolio valuation accuracy versus a manual calculation | > 99% |
| NFR-ACC-02 | Balance normalisation errors caused by incorrect decimals | Zero tolerance; errors must be detected and surfaced |
| NFR-ACC-03 | Snapshot values must match the calculation at the recorded time | 100% reproducible from stored inputs |

### 8.2 Freshness and performance

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-PERF-01 | Price data freshness for portfolio valuation | See open question OQ-001; documented job cadence is 15 minutes |
| NFR-PERF-02 | Portfolio dashboard response time (p95) | < 2 seconds for typical portfolios |
| NFR-PERF-03 | Account sync feedback for a single account | < 30 seconds typical, with progress/failure shown |
| NFR-PERF-04 | Time to first value after sign-in and first account setup | < 2 minutes |
| NFR-PERF-05 | Snapshot history query (p95) | < 2 seconds for 1 year of daily snapshots |

### 8.3 Availability and reliability

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-AVL-01 | Monthly availability of the web application and API | ≥ 99.5% (proposed) |
| NFR-AVL-02 | Scheduled jobs must not silently miss a run | Alert within 15 minutes |
| NFR-AVL-03 | A failed external provider must not take down the whole product | Graceful degradation with last-known data |
| NFR-AVL-04 | Recovery point objective (RPO) for portfolio data | ≤ 24 hours (proposed) |
| NFR-AVL-05 | Recovery time objective (RTO) | ≤ 4 hours (proposed) |

### 8.4 Scalability

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-SCL-01 | Support initial user base without architectural change | Define launch capacity; scale horizontally |
| NFR-SCL-02 | Price collection must handle the tracked asset universe within its interval | < 15 minutes |
| NFR-SCL-03 | EOD snapshots must scale with number of portfolios | < 1 hour at expected volume |
| NFR-SCL-04 | The data model must not prevent future multi-tenant/team use | No blocker by design |

### 8.5 Observability and maintainability

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-OBS-01 | Structured logs with correlation IDs for requests and jobs | Must |
| NFR-OBS-02 | Health/readiness endpoints for all services | Must |
| NFR-OBS-03 | Metrics for request latency, error rate, job outcomes, and provider failures | Must |
| NFR-OBS-04 | Alerts for sustained error rates, job failures, and stale prices | Must |
| NFR-OBS-05 | Documentation sufficient for a new engineer or operator to run the system | Must |
| NFR-OBS-06 | Automated tests covering business rules in this BRD set | Should |

### 8.6 Accessibility and usability

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-UX-01 | Core flows usable on mobile and desktop browsers | Must |
| NFR-UX-02 | Colour contrast and keyboard navigation meet WCAG 2.1 AA where feasible | Should |
| NFR-UX-03 | Errors and data-quality warnings are understandable to non-technical users | Must |

## 9. Risks and mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Credential compromise | Critical | Encryption, read-only permissions, least privilege, secret rotation, no logging |
| Data leakage between users | Critical | Ownership checks, row-level authorisation tests, audit |
| Provider outage/stale prices | High | Multiple providers, last-known-price fallback, freshness labels, alerts |
| Silent job failure | High | Heartbeat monitoring, alerts, run records |
| Regulatory breach | High | Legal review before launch of tax/advice features; configurable restrictions |
| Incorrect decimals/normalisation | High | Registry validation, reconciliation checks, anomaly alerts |
| Insufficient backup/restore testing | High | Scheduled restore drills, documented RPO/RTO |

## 10. Controls checklist (pre-launch)

- [ ] Threat model completed and reviewed.
- [ ] Penetration test of authentication, authorisation, and credential handling.
- [ ] Secrets management verified; no secrets in source control or logs.
- [ ] Backup and restore exercised successfully.
- [ ] Rate limiting and abuse controls validated.
- [ ] Audit log coverage verified for all sensitive events.
- [ ] Data-retention and deletion policy approved.
- [ ] Privacy notice and terms reviewed.
- [ ] Provider terms of service reviewed (price, exchange, RPC, Notion).
- [ ] Jurisdictional legal review completed for advisory/recommendation outputs.
- [ ] Incident-response and breach-notification process defined.
- [ ] Accessibility review completed for core flows.

## 11. Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | BRD-001 targets <30-second price freshness, while BRD-011 specifies 15-minute collection. Which is the committed requirement? | Product |
| OQ-002 | Which jurisdictions are in scope at launch? | Compliance |
| OQ-003 | What retention periods apply to audit logs, snapshots, and personal data? | Compliance |
| OQ-004 | What availability and recovery targets are commercially promised (SLA)? | Leadership |
| OQ-005 | Is a third-party security audit required before launch? | Security |
| OQ-006 | Which compliance capabilities (screening, KYC) are mandatory in the launch market? | Compliance |
