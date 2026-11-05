# Scope Definition

**Intent:** `261105-brd-001-product` · **Stage:** `ideation/scope-definition` · **Status:** proposed

**Scope decision.** MVP is the full Option B retail slice: read-only cross-venue aggregation and decision support with user-initiated order intents, EU (MiCA) and UK first, five exchanges and ten chains, WalletConnect and CSV committed, xpub/watch-only a Should fast-follow [Q1, Q6, CON-019]. Ten launch services plus gateway, SPA, platform and observability are in scope. The backlog is **58 units (U-001–U-058), 186.0 ew committed Must** — **10.7 months** at five engineers and 80% capacity, inside Q6's 10–11 month envelope, at the feasibility ceiling of **186 ew** (0.0 headroom) and **4.7 ew under** the 11-month capacity of **190.7 ew**. The former **192.0 ew** (11.1 months) was **6.0 ew over** 186 and **1.3 ew over** 190.7; merges U-001/U-005/U-006 removed the 6.0 ew. Nine-month capacity is **156.0 ew**, leaving a **30.0 ew** cut list.

## In Scope (MVP)

One schema/role per service, no cross-schema grants except the `platform` `SECURITY DEFINER` Vault accessor (CON-011). No RLS. HTTP-only service calls; no queues. Browser-external calls: Supabase Auth and WalletConnect; scheduled runs use a separate service token (CON-005).

| Surface | Launch delivery |
| --- | --- |
| **SPA + PWA shell** | Onboarding, dashboard, portfolios, valuation, drift, P&L, snapshots, order intents, briefs, billing, settings; WCAG 2.2 AA. |
| **API Gateway** | Deny-by-default routing, JWT/JWKS pinning, entitlements, shared limits, correlation IDs. |
| **Identity; Account; Registry** | Auth/MFA/sessions/consent/gate/export/erasure; five read-only venues, ten chains, WalletConnect, CSV; canonical assets, contracts, decimals. |
| **Market Data; Portfolio; Ledger** | Prices/FX with freshness/provenance; targets, valuation, drift, snapshots, orders; ledger, dedupe, pairing, reconciliation, cost basis, P&L, export. |
| **Reporting; Billing; Jobs; Audit** | Briefs, email, degraded alerts; plans, entitlements, checkout, dunning; schedules, retries, heartbeat; insert-only audit, retention. |
| **Platform; Observability** | IaC, staging, three-manager Swarm, registry, CDN/WAF, SBOM, PITR; Loki/Prometheus/Grafana, RED, status page, SEV-1. |

**Mixed units (Must/Should ew); Must in 186.0, Should in 14.5:** U-012 2/2, U-036 1.5/1, U-037 2/1, U-049 2.5/1.5, U-051 1.5/1.5, U-020 1.5/1, U-027 2.5/1, U-039 3/1, U-034 2.5/0.5, U-052 2.5/0.5, U-007 3.5/0.5, U-010 9/0.5, U-018 2.5/0.5, U-016 6/0.5, U-042 6/1, U-040 2/0.5. Should tails sum to 14.5.

**Effort reconciliation:** 81.0 + 47.5 + 74.0 + 17.0 = 219.5; duplicate merges −6.0; other overlap −13.0; cleaned 200.5; 16 Should slices −14.5; committed **186.0**. **DAG:** 58 units, **167 hard edges, 0 cycles**; hard edges invert `Depends on` exactly, all 58 placed once with dependencies earlier (U-004 included). Former cycles are one-way edges (U-021→U-034, U-023→U-034, U-053→U-046, U-052→U-046, U-049→U-052, U-027→U-046, U-043→U-025, U-042→U-050, U-055→U-008, U-006→U-049/U-053). **9-month cut list:** one additive de-duplicated set summing to 30.0 — U-034 2.5; U-035+U-036+U-052 6.5; U-038+U-039+U-040 7.0; U-027 2.5; U-011 3.0; U-012 Must 2.0; U-008 3.0; U-010+U-014 3.5.

## Out of Scope

**Should fast-follow (14.5 ew, excluded from 186.0; built immediately after launch, workaround at launch):** xpub/watch-only (U-012); PDF/PDF-UA briefs (U-036); benchmark attribution (U-037); German, French and Spanish locales (U-049); PWA installability (U-051); plus 11 slices in backlog §5.2 (archive, exports, local-currency display/trial, venue links, optional alerts, sub-account roll-up, DeFi labelling, watchlist, DEX pricing, non-core screens, usage UI). Not Post-MVP and not "not built".

**Post-MVP (32.0 ew indicative; not built):** templates/tags, rebalancing, partial fills, direct execution/signing, external briefs, on-chain decoding, tax lots, volatility/drawdown, Sharpe/Sortino, coupons/referrals, IAP billing, PPP tiers; also NFTs, all-asset aggregation, native mobile, advisor portals, white-label, AI, SOC 2/ISO 27001, US before C2c.

**Never:** custody/private keys; automatic execution; product-initiated transfers; fiat on/off-ramp; tax filing or regulated advice; futures/margin. CPB supplies no MiCA Art 3(1)(16) service and never routes an order intent [CON-022]. KYC/AML and sanctions screening are out unless a provider term or counsel makes screening a gate (CON-050, NFR-LEG-07).

## MoSCoW Summary

| Priority | Capabilities |
| --- | --- |
| **Must** | The 186.0 ew committed slice above. |
| **Should** | The 16 fast-follow slices (14.5 ew). |
| **Could** | Extra venues/chains; bug bounty; manual price override; experiment platform. |
| **Won't** | Every Post-MVP and never item; a sixth venue pre-launch without an equivalent cut. |

## Value Stream Map (capability → customer outcome)

| Capability | Customer outcome and launch measure |
| --- | --- |
| Account, Registry, Market Data | "What do I own? What is it worth?" — first value <2 min address, <10 min key; ≤5 min top 100, ≤15 min tail; ≥95% priced and fresh. |
| Portfolio, Ledger, Reporting | "Where am I off plan?" / "Am I up?" / "What next?" — >99% accuracy, ≥90% P&L; immediate allocation effect; 100% of briefs generated or explicitly failed. |
| Billing, Identity, Audit, Jobs, Gateway, Platform | Activation ≥40%, conversion ≥3%, D30 ≥25%, availability ≥99.9%. |

## Deferred BRD Musts (explicit deviation record)

Approved at Q8. Three services are deferred at service level; each Must is bound to a unit or a named launch-time degraded path.

| Deferred service | Must requirements affected | Degraded path and coverage | Units |
| --- | --- | --- | --- |
| **Notification** | BRD-015's 14 Musts (FR-NOT-01, 02, 04, 07–14, 16, 17, 19); FR-GEN-13; BRD-009 delivery; FR-JOB-18 | In-app alerts plus mandatory email for security, credential and billing events; channel per type (FR-NOT-04); one-click unsubscribe (FR-NOT-08); per-type on/off; retry, dedupe, delivery log. **13/14 met; FR-NOT-14 partial** (email English-only). **FR-GEN-13 partial** (no web push; FR-NOT-03 is Should). | U-052, U-046 |
| **Analytics** | BRD-021's 10 Musts; FR-GEN-20; FR-JOB-25; FR-ADM-18 | Consent-aware PII-free events, fixed metrics, daily aggregation, dashboards, flags with owner/expiry/**rollout percentage**, PII-free experiment exposure, retention deletion. **All 10 met**, including FR-ANL-10 and FR-ANL-13. | U-053, U-046, U-048 |
| **Support Console** | BRD-019's 14 Musts; FR-GEN-18 | Time-boxed, second-approver, reason-recorded, audited break-glass read-only view; external help desk; status page. **All 14 met in degraded form.** FR-ADM-14/15 are U-003/U-005, **not** this deferral. | U-005, U-054, U-047 |

**Partial Musts (4):** FR-GEN-13 (no web push, U-052); FR-NOT-14 and FR-L10N-15 (email English-only, U-052 with U-049 templates); FR-REP-23 (PDF half not shipped, U-050 verifies U-035/U-036).

**386/386 Must rows bound; 4 partial; 0 wholly deferred.** Launch may not claim complete BRD-001 coverage. U-053 criteria 5–6 test owner/expiry/rollout flags (hashed, no PII) and PII-free exposure, so FR-ADM-18/FR-ANL-13 are met; U-052 criteria 2–3 test FR-NOT-04/FR-NOT-08.

## Launch Gates

| Gate | Test / evidence |
| --- | --- |
| **C1–C2 Licences, legal** | Written display/storage/derivation/region rights; exchange/benchmark/RPC terms; quotes; Tron/venue matrix; EU MiCA, UK current/future and US MSB opinions; copy review. |
| **C3–C4 Vault, availability** | CON-011 ADR, `search_path`, `REVOKE ALL`, `current_user`, KEK rotation, fail-closed, no plaintext; three managers/domains, replicas, external monitor, node-kill drill, Supabase SLA. |
| **C5–C7 Scope, capacity, budget** | Option B boundary, Q8 record, **186.0 ew reconciliation**, 30.0 ew cut list; Rust/Axum/sqlx, React, Supabase, Swarm/Postgres skills; legal ≈USD 50,000, non-salary ≤USD 150,000, quotes (R-018). |
| **C8 Data protection** | Edge/WAF **and** Supabase `before user created` gate; DPIA; Article 6/30; DPAs, TIA; erasure; **breach runbook/tabletop filed against C8 for FR-PRV-15 / NFR-PRV-11 (U-054 criterion 7)**. |
| **C9–C10 Consumer, performance** | Renewal disclosure, cancellation parity, no dark patterns, trader role; p95 <2 s at 300 RPS for 60 minutes, one worker drained. |
| **C11–C12 Operations, infrastructure** | Status page, SEV-1 within 15 minutes, on-call, five runbooks, drill; IaC-only, staging, digest-pinned scanned images, CDN/WAF, rollback. |
| **C13–C14 Accessibility, pen test** | EN 301 549 / WCAG 2.2 AA audit, zero blockers, PDF/UA only if PDF, **DECISIONS D-016**; gateway, auth, cross-tenant, Vault and credentials retested. |
| **C15–C16 Restore, locales** | RPO ≤24 h / RTO ≤4 h drill, backup shredding; English (EU) and English (UK) fixed, extra locale is a scope change. |

**Kill criteria:** C1, C2a/C2b, C3, M0 week-8 and C6 failures.

## Assumptions & Open Questions

- **Plan of record is 10.7 months**; the 9-month target needs the 30.0 ew cut set, seven engineers for nine months, or a Q1 scope change.
- **Launch locales:** English (EU)/English (UK); German, French, Spanish are Should fast-follows pending C16.
- **Licence fees are unpriced** — the largest unknown [R-018]; C1/C7 require quotes.
- **Supabase region, SLA, PITR and backup-versus-shred** are unverified M0 decisions; the UK goes live only after counsel sign-off, and an adverse opinion escalates.
- **Tron** needs a named RPC provider; region-gated venues need a region-aware matrix.
- **D-016 is two records:** **RAID D-016 (platform foundation)** is launch-blocking, owned by U-055–U-058; **DECISIONS D-016 (accessibility)** is the C13 gate owned by U-050.
- **Retention:** 5-year financial and 12-month audit periods are proposed pending BRD-017 OQ-004.
