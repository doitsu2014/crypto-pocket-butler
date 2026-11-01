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
| NFR-SEC-02 | Exchange API keys, secrets, and passphrases must be encrypted at rest with Supabase Vault (pgsodium) keys and must never be stored in plaintext. | Must |
| NFR-SEC-03 | Credential fields must never appear in API responses, logs, error messages, or analytics. | Must |
| NFR-SEC-04 | The product must not request or store wallet private keys or seed phrases. | Must |
| NFR-SEC-05 | Exchange integrations must use read-only permissions; the product must not enable withdrawals. | Must |
| NFR-SEC-06 | All user-owned resources must enforce ownership checks on read and write. | Must |
| NFR-SEC-07 | Administrative capabilities must require the explicit admin role projected by Supabase Auth into the `app_metadata.role` claim. | Must |
| NFR-SEC-08 | Authentication tokens must be validated against Supabase Auth JWKS for signature, issuer, audience, and expiry. | Must |
| NFR-SEC-09 | Authentication sessions must be managed by Supabase Auth; browser tokens must be held in memory and never persisted in localStorage. | Must |
| NFR-SEC-10 | Public endpoints must be rate-limited per user/IP at the API Gateway using Supabase Postgres atomic counters; authentication and manual job triggers must be limited more strictly. | Must |
| NFR-SEC-11 | Security-sensitive events must be logged without recording secrets. | Must |
| NFR-SEC-12 | Dependencies and container images must be scanned for known vulnerabilities before release. | Should |
| NFR-SEC-13 | Secrets must be supplied through Supabase Vault or Supabase project secrets, never committed to source control. | Must |
| NFR-SEC-14 | Backups must be encrypted and access-controlled, using Supabase encrypted backups and point-in-time recovery (PITR) where configured. | Must |
| NFR-SEC-15 | Multi-factor authentication must be available through Supabase Auth. | Should |

### 4.1 Supabase platform security controls and shared responsibility

The product consumes Supabase-managed infrastructure. **Supabase does not host the Rust
services or the React SPA**; those run on the container host. Supabase is responsible for the
managed runtime, patching, encryption primitives and backup mechanics. The product remains
responsible for schema and role isolation, application authorisation, secret classification,
data minimisation, log redaction, retention and restore drills.

| Supabase service | Security control | Shared-responsibility boundary |
|------------------|------------------|--------------------------------|
| **Supabase Auth** | Passwords, MFA factors, sessions, JWT signing and refresh are managed by GoTrue. The Custom Access Token Hook projects `app_metadata.role = "admin"`. Services validate JWTs against the Supabase Auth JWKS and re-check the admin claim. Auth Hooks/webhooks for sign-up, sign-in, sign-out and user-update are verified with the standard-webhooks signature and idempotency. | Supabase secures the Auth engine and signing keys. The product owns role projection, admin gating, hook verification, user-lifecycle policy, disabled-user denial and session revocation through the Admin API. |
| **Supabase Postgres** | Managed Postgres with one schema and one Postgres role per service (identity, account, registry, market, portfolio, reporting, job, audit, plus the gateway cache schema) and no cross-schema grants. **Row Level Security is not used by services**, because a service connects as its own role rather than as an end user; ownership and admin rules are enforced in application use cases against the signed internal principal assertion. | Supabase secures the engine, storage, encryption and backup mechanics. The product owns schema design, migrations, grants, connection budgets, advisory-lock usage, query safety, RLS decisions and per-service data minimisation. |
| **Supabase Queues (pgmq)** | Durable Postgres-backed queues with visibility timeouts, archive and dead-letter handling and at-least-once delivery. Producers use a transactional outbox; consumers use an inbox dedup table and are idempotent. Events never carry credentials, tokens or signed assertions. | Supabase operates the queue engine. The product owns queue and subject naming, visibility-timeout tuning, archive/dead-letter monitoring, consumer idempotency and throughput validation. |
| **Supabase Storage** | Private buckets for audit export and retention archives. Access is only through signed, access-controlled URLs; no public bucket is permitted. | Supabase operates storage and URL signing. The product owns bucket privacy, signed-URL lifetime, archive retention, access review and encryption expectations. |
| **Supabase Vault** | pgsodium-backed secrets store with versioned encryption, per-credential data encryption keys and per-subject crypto-shredding keys. Secret values are resolved at runtime through the owning service's Postgres role and are never returned, logged, published in events or placed in briefs. | Supabase provides the encryption primitives and the secrets store. The product owns secret classification, least-privilege access, rotation runbooks, break-glass procedure and ensuring no plaintext leakage. |
| **Supabase Cron (pg_cron)** | Scheduled SQL enqueues `job.tick` into Supabase Queues, runs retention triggers and hosts the independent dead-man watchdog that calls the Operational Alerting Channel through pg_net. | Supabase runs the scheduler. The product owns the real job semantics in Job Orchestration (due-ness, logical run keys, single-flight, leases, fencing, retries and outcomes) and the alert thresholds. |
| **Supabase Logs & Metrics** | Platform logs, metrics, log drains and alerts with correlation and trace context. Credentials, tokens and signed assertions must never be written to logs. | Supabase collects telemetry. The product owns structured logging, correlation, health and readiness endpoints, metric selection, dashboards, alert routing and log-retention requirements. |

**Shared-responsibility summary.** Supabase secures the managed runtime and the primitives;
the product secures how it uses them. Configuration of Supabase plan limits, PITR, Auth rate
limits, log retention and network access is a product decision, not an implicit guarantee.
Restore drills and evidence for NFR-AVL-04/NFR-AVL-05 remain the product's obligation even
though Supabase operates the backup mechanism.

## 5. Privacy and data protection

| ID | Requirement | Priority |
|----|-------------|----------|
| NFR-PRV-01 | Collect only the personal data required to deliver the service (identity claims, account labels, wallet addresses). | Must |
| NFR-PRV-02 | Document the purpose and retention period for each category of personal data. | Should |
| NFR-PRV-03 | Provide a way to export a user's data on request. | Should |
| NFR-PRV-04 | Provide a way to delete or anonymise a user's data on request, subject to legal retention. | Should |
| NFR-PRV-05 | Do not sell or share personal data with third parties beyond the processors required to run the service. | Must |
| NFR-PRV-06 | Disclose which external providers receive data (Supabase Auth, Supabase Postgres/Storage/Logs, exchanges, RPC providers, price providers, Notion). | Should |
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

Sign-in, sign-in-failure, sign-out and user-update events are sourced from verified Supabase
Auth hooks; the product records the verified event, not raw Auth payloads.

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
| Credential compromise | Critical | Supabase Vault encryption, read-only permissions, least privilege, secret rotation, no logging |
| Data leakage between users | Critical | Ownership checks, row-level authorisation tests, audit |
| Supabase plan limits or project outage | High | Monitor plan quotas, Auth rate limits and Postgres/queue capacity; encrypted backups and PITR; documented degradation and restore procedure |
| Provider outage/stale prices | High | Multiple providers, last-known-price fallback, freshness labels, alerts |
| Silent job failure | High | Heartbeat monitoring, alerts, run records |
| Regulatory breach | High | Legal review before launch of tax/advice features; configurable restrictions |
| Incorrect decimals/normalisation | High | Registry validation, reconciliation checks, anomaly alerts |
| Insufficient backup/restore testing | High | Supabase PITR and encrypted backups, scheduled restore drills, documented RPO/RTO |

## 10. Controls checklist (pre-launch)

- [ ] Threat model completed and reviewed.
- [ ] Penetration test of authentication, authorisation, and credential handling.
- [ ] Supabase Vault and Supabase project secrets verified; no secrets in source control or logs.
- [ ] Supabase encrypted backups and PITR configured, and a restore drill exercised successfully.
- [ ] Schema/role isolation verified: one schema and role per service, no cross-schema grants, and RLS not relied on by services.
- [ ] Supabase plan limits (MAU, database, storage, pgmq throughput, Auth rate limits, log retention) reviewed against launch capacity.
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
| OQ-007 | Which Supabase plan and add-ons (compute, PITR, Storage, log retention) are committed for launch, and do they meet the RPO/RTO and capacity targets? | Leadership / Security |
