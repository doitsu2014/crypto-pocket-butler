# BRD-012 — Security, Compliance & Non-Functional Requirements

Draft v1.1 · Security/Compliance · 2026-11-02. Cross-cutting obligations that constrain how every other capability is delivered. Post-MVP requirements must still be reachable from MVP decisions.

**Out of scope:** physical security · HR compliance · jurisdiction-specific legal advice (external counsel validates) · smart-contract security (no contracts deployed in the MVP).

## Security principles

| ID | Principle |
|----|-----------|
| SP-01 | Read-only by default — no withdrawal, transfer, or trading permission anywhere in the MVP |
| SP-02 | Least privilege for users, admins, jobs, and integrations |
| SP-03 | No secret exposure — encrypted at rest, masked in UI, never returned or logged |
| SP-04 | Defence in depth — AuthN, AuthZ, validation, rate limiting, encryption layered |
| SP-05 | Deny by default |
| SP-06 | Traceability — sensitive actions recorded with actor, time, outcome |
| SP-07 | Privacy by design — collect and retain only what the purpose needs |
| SP-08 | Safe failure — failures never expose data or silently corrupt valuations |

## Security requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| NFR-SEC-01 | TLS for all production traffic; plaintext rejected. | Must |
| NFR-SEC-02 | Exchange keys/secrets/passphrases encrypted at rest (Supabase Vault/pgsodium), never plaintext. | Must |
| NFR-SEC-03 | Credential fields never in API responses, logs, errors, or analytics. | Must |
| NFR-SEC-04 | Never request or store wallet private keys or seed phrases. | Must |
| NFR-SEC-05 | Exchange integrations read-only; withdrawals never enabled. | Must |
| NFR-SEC-06 | Ownership checks on every read/write of user-owned resources. | Must |
| NFR-SEC-07 | Admin capabilities require the `admin` role from Supabase `app_metadata.role`. | Must |
| NFR-SEC-08 | Tokens validated against Supabase JWKS (signature, issuer, audience, expiry). | Must |
| NFR-SEC-09 | Sessions via Supabase Auth; browser tokens in memory only, never localStorage. | Must |
| NFR-SEC-10 | Rate-limit public endpoints per user/IP at the gateway; auth and manual triggers stricter. | Must |
| NFR-SEC-11 | Security-sensitive events logged without secrets. | Must |
| NFR-SEC-12 | Dependency and container-image vulnerability scanning before release. | Should |
| NFR-SEC-13 | Secrets via Supabase Vault or project secrets, never in source control. | Must |
| NFR-SEC-14 | Encrypted, access-controlled backups (Supabase encrypted backups + PITR where configured). | Must |
| NFR-SEC-15 | MFA available through Supabase Auth. | Should |

### Platform shared responsibility

Supabase is the **database platform** — it does not host the Rust services, the SPA, or the observability stack; those run on the product's **Docker Swarm** cluster. Supabase secures the managed runtime, patching, encryption primitives, and backup mechanics. The product owns schema/role isolation, authorisation, secret classification, data minimisation, log redaction, retention and restore drills, and the Swarm cluster itself.

| Service | Split |
|---------|-------|
| Supabase Auth | Supabase secures GoTrue, MFA, sessions, JWT signing. Product owns role projection (`app_metadata.role`), admin gating, webhook verification, user lifecycle, session revocation. |
| Supabase Postgres | One schema + one role per service, no cross-schema grants; **RLS not used by services** (they connect as their own role; ownership enforced in use cases against the forwarded JWT). Product owns schema design, migrations, grants, query safety. |
| Supabase Storage | Private buckets, signed URLs only. Product owns bucket privacy, URL lifetime, retention, access review. |
| Supabase Vault | pgsodium secrets with per-credential and per-subject crypto-shredding keys. Product owns classification, rotation runbooks, break-glass, no-leak guarantees. |
| Supabase Cron | Ticks jobs via pg_net; hosts the dead-man alert. Product owns job semantics (due-ness, single-flight, retries) and alert thresholds. |
| Swarm Observability Stack | Loki/Prometheus/Tempo/Grafana on Swarm. Product operates it all: logging, correlation, metrics, dashboards, alert routing, log retention. |

## Privacy

| ID | Requirement | Priority |
|----|-------------|----------|
| NFR-PRV-01 | Collect only required personal data (identity claims, account labels, on-chain account addresses). | Must |
| NFR-PRV-02 | Document purpose and retention per data category. | Should |
| NFR-PRV-03 | User data export on request. | Should |
| NFR-PRV-04 | User data deletion/anonymisation on request, subject to legal retention. | Should |
| NFR-PRV-05 | No selling or sharing personal data beyond required processors. | Must |
| NFR-PRV-06 | Disclose providers receiving data (Supabase, exchange, RPC providers). | Should |
| NFR-PRV-07 | Data minimisation in logs and analytics. | Must |

## Audit logging

Record (append-only, protected from modification): sign-ins/out (from verified Supabase Auth hooks, not raw payloads) · account/portfolio/target/guardrail changes · credential changes (never the credential) · sync outcomes · order decisions (create/cancel) and trade recording · admin registry changes · manual job triggers · brief generation outcomes. Each event: actor (or system), action, target, timestamp, context, outcome.

## Compliance roadmap

Post-MVP: transaction export · capital-gains (FIFO/LIFO/HIFO) · staking income · sanctioned-address screening · KYC/AML · geographic restrictions. **Must define before launch:** retention and legal-hold policy. Launching in a jurisdiction without required controls needs explicit legal sign-off; advisory outputs reviewed against local investment-advice rules.

## NFRs

| Area | ID | Requirement | Target |
|------|----|-------------|--------|
| Accuracy | NFR-ACC-01 | Valuation vs. manual calculation | >99% |
| | NFR-ACC-02 | Decimals normalisation errors | Zero; detected and surfaced |
| | NFR-ACC-03 | Snapshots reproducible from stored inputs | 100% |
| Performance | NFR-PERF-01 | Price freshness | See OQ-001 (cadence: 15 min) |
| | NFR-PERF-02 | Dashboard p95 | <2s |
| | NFR-PERF-03 | Single-account sync feedback | <30s typical |
| | NFR-PERF-04 | Time to first value | <2 min |
| | NFR-PERF-05 | Snapshot history p95 | <2s for 1 year |
| Availability | NFR-AVL-01 | Monthly availability | ≥99.5% (proposed) |
| | NFR-AVL-02 | Silent missed job runs | Alert ≤15 min |
| | NFR-AVL-03 | Provider failure | Graceful degradation, last-known data |
| | NFR-AVL-04/05 | RPO / RTO | ≤24h / ≤4h (proposed) |
| Scalability | NFR-SCL-01 | Launch capacity | Defined; horizontal scaling |
| | NFR-SCL-02/03 | Price collection / EOD within interval | <15 min / <1h |
| | NFR-SCL-04 | Multi-tenant reachability | No blocker by design |
| Observability | NFR-OBS-01–04 | Structured logs + correlation IDs; health/readiness; latency/error/job metrics; alerts | Must |
| | NFR-OBS-05/06 | Run-the-system docs; business-rule test coverage | Must / Should |
| Usability | NFR-UX-01–03 | Mobile+desktop; WCAG 2.1 AA where feasible; understandable errors | Must / Should / Must |

## Risks

Credential compromise (critical) → Vault, read-only keys, rotation, no logging · cross-user leakage (critical) → ownership checks + tests + audit · Supabase outage/limits (high) → quota monitoring, PITR, restore drills · stale prices (high) → fallback, freshness labels, alerts · silent job failure (high) → heartbeats, run records · regulatory breach (high) → legal review, configurable restrictions · wrong decimals (high) → registry validation, reconciliation.

## Pre-launch checklist

- [ ] Threat model reviewed
- [ ] Pen test of authN, authZ, credential handling
- [ ] Vault verified; no secrets in source or logs
- [ ] Backups + PITR configured; restore drill exercised
- [ ] Schema/role isolation verified (one per service, no cross-schema grants)
- [ ] Supabase plan limits reviewed against launch capacity
- [ ] Rate limiting validated
- [ ] Audit coverage verified
- [ ] Retention/deletion policy approved
- [ ] Privacy notice and terms reviewed
- [ ] Exchange and RPC provider terms reviewed
- [ ] Jurisdictional legal review of order and reporting outputs
- [ ] Incident-response process defined
- [ ] Accessibility review of core flows

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Price freshness: BRD-001 says <30s, BRD-011 says 15-min collection — which is committed? | Product |
| OQ-002 | Launch jurisdictions? | Compliance |
| OQ-003 | Retention for audit logs, snapshots, personal data? | Compliance |
| OQ-004 | Commercially promised SLA targets? | Leadership |
| OQ-005 | Third-party security audit required pre-launch? | Security |
| OQ-006 | Mandatory compliance capabilities (screening, KYC) in the launch market? | Compliance |
| OQ-007 | Supabase plan/add-ons for launch; do they meet RPO/RTO and capacity? | Leadership/Security |
