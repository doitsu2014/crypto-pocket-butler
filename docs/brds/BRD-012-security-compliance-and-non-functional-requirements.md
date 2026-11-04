# BRD-012 — Security, Compliance & Non-Functional Requirements

Draft v1.2 · Security/Compliance · 2026-11-04. Cross-cutting obligations that constrain how every capability is delivered. v1.2 adds the global privacy and regulatory programme, support/incident obligations, accessibility, and a 99.9% availability target. Post-MVP requirements must still be reachable from MVP decisions.

**Out of scope:** physical security · HR compliance · jurisdiction-specific legal advice (external counsel validates) · smart-contract security (no contracts deployed in the MVP) · tax filing.

## Security principles

| ID | Principle |
|----|-----------|
| SP-01 | Read-only by default — no withdrawal, transfer, or trading permission anywhere in the MVP |
| SP-02 | Least privilege for users, admins, support, jobs, and integrations |
| SP-03 | No secret exposure — encrypted at rest, masked in UI, never returned or logged |
| SP-04 | Defence in depth — AuthN, AuthZ, validation, rate limiting, encryption layered |
| SP-05 | Deny by default |
| SP-06 | Traceability — sensitive actions recorded with actor, time, outcome |
| SP-07 | Privacy by design — collect and retain only what the purpose needs |
| SP-08 | Safe failure — failures never expose data or silently corrupt valuations |
| SP-09 | Verify, do not trust — exchange key permissions, addresses, and user claims are checked |
| SP-10 | Support access is exceptional, time-bound, and audited |

## Security requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| NFR-SEC-01 | TLS for all production traffic; plaintext rejected. | Must |
| NFR-SEC-02 | Exchange keys/secrets/passphrases encrypted at rest (Supabase Vault/pgsodium), never plaintext. | Must |
| NFR-SEC-03 | Credential fields never in API responses, logs, errors, analytics, or support screens. | Must |
| NFR-SEC-04 | Never request or store wallet private keys or seed phrases. | Must |
| NFR-SEC-05 | Exchange integrations read-only; withdrawals never enabled. | Must |
| NFR-SEC-06 | Verify key permissions before use and periodically; reject withdrawal/trading permissions. | Must |
| NFR-SEC-07 | Ownership checks on every read/write of user-owned resources. | Must |
| NFR-SEC-08 | Admin and Support capabilities require the correct role from Supabase `app_metadata.role`. | Must |
| NFR-SEC-09 | Tokens validated against Supabase JWKS (signature, issuer, audience, expiry). | Must |
| NFR-SEC-10 | Sessions via Supabase Auth; browser tokens in memory only, never localStorage. | Must |
| NFR-SEC-11 | MFA available to all; required for users with exchange credentials. | Must |
| NFR-SEC-12 | Rate-limit public endpoints per user/IP/region at the gateway; auth and manual triggers stricter. | Must |
| NFR-SEC-13 | Security-sensitive events logged without secrets. | Must |
| NFR-SEC-14 | Dependency and container-image vulnerability scanning before release. | Must |
| NFR-SEC-15 | Secrets via Supabase Vault or project secrets, never in source control. | Must |
| NFR-SEC-16 | Encrypted, access-controlled backups (Supabase encrypted backups + PITR where configured). | Must |
| NFR-SEC-17 | Device/session listing and revocation. | Must |
| NFR-SEC-18 | Login alerts and anomaly detection for new devices/locations. | Should |
| NFR-SEC-19 | Support access is read-only, time-bound, reason-recorded, and audited. | Must |
| NFR-SEC-20 | Penetration test of authN, authZ, credential handling, and support access before launch. | Must |
| NFR-SEC-21 | Vulnerability disclosure policy and security contact page. | Should |
| NFR-SEC-22 | Bug-bounty or equivalent programme after launch. | Could |
| NFR-SEC-23 | Administrative and billing actions are audited and reconciled. | Must |
| NFR-SEC-24 | Data at rest encrypted; per-subject crypto-shredding keys for erasure. | Must |
| NFR-SEC-25 | Third-party processors are assessed and contracted before receiving user data. | Must |

## Privacy

| ID | Requirement | Priority |
|----|-------------|----------|
| NFR-PRV-01 | Collect only required personal data (identity claims, account labels, addresses, preferences). | Must |
| NFR-PRV-02 | Document purpose, lawful basis, and retention per data category (BRD-017). | Must |
| NFR-PRV-03 | User data export on request, machine-readable. | Must |
| NFR-PRV-04 | User data deletion/anonymisation in-app, subject to legal retention. | Must |
| NFR-PRV-05 | No selling or sharing personal data beyond required processors. | Must |
| NFR-PRV-06 | Disclose providers receiving data (Supabase, exchanges, RPC, email, payments, analytics). | Must |
| NFR-PRV-07 | Data minimisation in logs, analytics, and support tools. | Must |
| NFR-PRV-08 | Consent for marketing and non-essential analytics. | Must |
| NFR-PRV-09 | Subprocessor list and DPA maintained. | Must |
| NFR-PRV-10 | Data residency and cross-border transfer mechanism documented. | Must |
| NFR-PRV-11 | Breach notification within the statutory window. | Must |
| NFR-PRV-12 | Age and jurisdiction gates before data collection. | Must |
| NFR-PRV-13 | Cookie/consent controls where required; no pre-ticked boxes. | Must |
| NFR-PRV-14 | Automated retention and deletion jobs. | Must |
| NFR-PRV-15 | Privacy requests authenticated, logged, and fulfilled on time. | Must |

## Legal and regulatory

| ID | Requirement | Priority |
|----|-------------|----------|
| NFR-LEG-01 | Publish terms, privacy notice, and risk disclosure; localize where required. | Must |
| NFR-LEG-02 | Gate access by jurisdiction; block restricted markets before data collection. | Must |
| NFR-LEG-03 | Legal review of order, allocation, and reporting language against local advice rules. | Must |
| NFR-LEG-04 | Review marketing and financial promotions against applicable regimes (for example EU MiCA, UK FCA, US rules). | Must |
| NFR-LEG-05 | Secure market-data and benchmark licences with attribution requirements. | Must |
| NFR-LEG-06 | Review exchange API commercial-use terms per region. | Must |
| NFR-LEG-07 | Sanctions/geo restrictions and address screening where required by providers or law. | Should |
| NFR-LEG-08 | Consumer subscription compliance: auto-renewal disclosure, cancellation, refunds. | Must |
| NFR-LEG-09 | Tax treatment of subscriptions handled by the merchant of record. | Must |
| NFR-LEG-10 | Legal-hold and regulatory-retention overrides for deletion. | Must |

## Audit logging

Record (append-only, protected from modification): sign-ins/out and device events (from verified Supabase Auth hooks, not raw payloads) · account/portfolio/target/guardrail changes · credential changes (never the credential) · key permission verification and revocation · sync outcomes · transaction imports and adjustments · order decisions (create/cancel) and trade recording · admin registry changes · user/role/entitlement changes · support access · manual job triggers · brief and notification generation outcomes · billing changes · privacy requests.

Each event: actor (or system), action, target, timestamp, context, outcome. Audit records contain no credentials, tokens, or full addresses.

## Compliance roadmap

Post-MVP: transaction export and tax-ready reports · capital-gains methods beyond FIFO/average cost · staking income · sanctioned-address screening · KYC/AML · geographic restrictions · SOC 2 Type II / ISO 27001. **Must define before launch:** retention and legal-hold policy · launch jurisdictions · privacy notices · consumer-subscription terms · market-data licences · incident-response plan.

## NFRs

| Area | ID | Requirement | Target |
|------|----|-------------|--------|
| Accuracy | NFR-ACC-01 | Valuation vs. manual calculation | >99% |
| | NFR-ACC-02 | Decimals normalisation errors | Zero; detected and surfaced |
| | NFR-ACC-03 | Snapshots reproducible from stored inputs | 100% |
| | NFR-ACC-04 | P&L reconciliation to transaction ledger | 100% or labelled |
| Performance | NFR-PERF-01 | Top-asset price freshness | ≤5 min (tail ≤15 min) |
| | NFR-PERF-02 | Dashboard p95 | <2s |
| | NFR-PERF-03 | Single-account sync feedback | <30s typical |
| | NFR-PERF-04 | Time to first value | <2 min address; <10 min exchange |
| | NFR-PERF-05 | Snapshot history p95 | <2s for 1 year |
| Availability | NFR-AVL-01 | Monthly availability, paid plans | ≥99.9% |
| | NFR-AVL-02 | Silent missed job runs | Alert ≤5 min |
| | NFR-AVL-03 | Provider failure | Graceful degradation, last-known data |
| | NFR-AVL-04/05 | RPO / RTO | ≤24h / ≤4h (proposed) |
| | NFR-AVL-06 | Status page and incident communication | SEV-1 within 15 min |
| | NFR-AVL-07 | Global latency | CDN + multi-AZ; multi-region post-MVP |
| Scalability | NFR-SCL-01 | Launch capacity | Defined; horizontal scaling |
| | NFR-SCL-02/03 | Price collection / EOD within interval | <1 min top / <6h local |
| | NFR-SCL-04 | Multi-tenant reachability | No blocker by design |
| Observability | NFR-OBS-01–04 | Structured logs + correlation IDs; health/readiness; latency/error/job metrics; alerts | Must |
| | NFR-OBS-05/06 | Run-the-system docs; business-rule test coverage | Must / Should |
| Usability | NFR-UX-01–03 | Mobile + desktop; WCAG 2.2 AA; understandable errors | Must |
| Accessibility | NFR-A11Y-01–05 | Keyboard, screen reader, contrast, motion, accessible charts/PDF | Must |

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Credential compromise | Critical | Vault, permission verification, MFA, rotation, no logging |
| Cross-user leakage | Critical | Ownership checks + tests + audit + support controls |
| Privacy/regulatory breach | Critical | Jurisdiction gating, legal review, retention, DPAs |
| Data-licensing breach | High | Licence register, attribution, source review |
| Supabase outage/limits | High | Quota monitoring, PITR, restore drills |
| Stale prices or FX | High | Multi-source fallback, freshness labels, alerts |
| Silent job failure | High | Heartbeats, run records, dead-man alerts |
| Wrong decimals | High | Registry validation, reconciliation |
| Billing/tax error | High | Merchant of record, reconciliation, audit |
| Support access misuse | High | Least privilege, time-bound access, audit |
| Notification storm | Medium | Dedupe, rate limits, quiet hours |

## Pre-launch checklist

- [ ] Threat model reviewed
- [ ] Pen test of authN, authZ, credential handling, and support access
- [ ] Vault verified; no secrets in source, logs, analytics, or support screens
- [ ] Backups + PITR configured; restore drill exercised
- [ ] Schema/role isolation verified (one per service, no cross-schema grants)
- [ ] Supabase plan limits reviewed against launch capacity
- [ ] Rate limiting, bot protection, and quota enforcement validated
- [ ] Audit coverage verified
- [ ] Retention/deletion policy approved and automated
- [ ] Privacy notice, terms, risk disclosure, and cookie controls reviewed
- [ ] Launch jurisdictions and geo-blocking approved
- [ ] Exchange, RPC, market-data, and benchmark terms reviewed
- [ ] Jurisdictional legal review of order, allocation, performance, and reporting outputs
- [ ] Consumer-subscription terms and cancellation flow reviewed
- [ ] Incident-response process, status page, and on-call rota defined
- [ ] Accessibility review of core flows (WCAG 2.2 AA)
- [ ] MFA/passkey/device-management flows tested
- [ ] Data export and in-app deletion tested
- [ ] Support access and audit tested
- [ ] Product analytics consent and PII checks validated

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Which launch jurisdictions and regulatory regimes apply? | Compliance |
| OQ-002 | Which legal opinions are required before launch (advice, marketing, consumer, privacy)? | Legal |
| OQ-003 | Retention for audit logs, snapshots, transactions, and personal data? | Compliance |
| OQ-004 | Commercially promised SLA targets? | Leadership |
| OQ-005 | Third-party security audit and SOC 2 timeline? | Security |
| OQ-006 | Mandatory compliance capabilities (screening, KYC) in launch markets? | Compliance |
| OQ-007 | Supabase region and data-residency strategy? | Engineering/Legal |
| OQ-008 | Which market-data and benchmark licences are approved? | Legal/Data |
