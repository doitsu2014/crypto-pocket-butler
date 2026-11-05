# RAID Log — Feasibility & Constraints (fragment)

**Stage:** `ideation/feasibility` · **Artifact:** `raid-log` · **Status:** draft fragment for synthesis
**Basis:** intent statement, confirmed answers Q1–Q5, competitive analysis, market trends, build-vs-buy, and the fixed project constraints.
**Scales:** Likelihood High/Medium/Low · Impact Critical/High/Medium/Low · Issue severity Critical/High/Medium · Dependency criticality Launch-blocking/Critical/Planned.
**Treatment values:** mitigate · transfer · accept · avoid.

---

## Risks

### R-001 — Market-data redistribution and derived-valuation rights are not granted

- **Description:** The product's core surfaces — cross-venue valuation, local-currency P&L, charts — are derived works. CoinGecko's terms forbid redistribution, syndication and derived valuations without consent, require "Powered by CoinGecko" attribution, and cap the licence to a cache refreshed at least every 24 hours. CoinMarketCap's tiers include commercial-use rights, but its display and derivative-work terms have not been reviewed by counsel. No vendor has granted worldwide redistribution and derived-valuation rights at a retail price.
- **Likelihood / Impact:** High / Critical
- **Owner:** Architect Agent (technical), external counsel (contract)
- **Treatment:** Mitigate
- **Launch blocker:** Yes — cannot ingest or display vendor data in the EU/UK until resolved.
- **Response:** Send a written licence request now to CoinGecko and CoinMarketCap legal for worldwide redistribution plus derived-valuation rights, EU and UK first, with the exact display surfaces attached. Do not implement the Market Data adapter beyond a sandbox until a written grant exists; a sandbox is not a licence. Fallbacks in order: (1) CoinMarketCap Startup as primary if its terms grant derivatives in writing; (2) drop derived valuation and show vendor-sourced raw quotes only with attribution — which removes the differentiator and must be escalated to the approval gate; (3) enterprise vendor (Kaiko/CoinDesk Data) post-MVP. Track as a weekly decision item with a named legal owner; no ingestion of production data before signature.

### R-002 — MiCA treatment of user-initiated order intents

- **Description:** Execution and reception/transmission of orders are licensed CASP activities under MiCA Art 3(1)(16)(e) and (g). The product's "order intent" — a user records a buy/sell intent against a cross-venue view and never places the order — is designed as a documented handoff to a licensed venue, but the line between a recorded note with a deep link and "reception/transmission" is not settled. Grandfathering ended 1 July 2026, so no transitional cover exists.
- **Likelihood / Impact:** Medium / Critical
- **Owner:** Compliance Agent, with external counsel
- **Treatment:** Avoid the licensed activity by design; mitigate the residual interpretation risk
- **Launch blocker:** Yes for EU launch.
- **Response:** Implement the intent as a user-authored, non-transmitted record: no order routing, no order book, no trade-enabled exchange credentials, no execution API, and a human approval on every intent. Request only read-only scopes (Binance defaults to no TRADE, Kraken query-only, Coinbase view scope). Obtain a written counsel opinion on whether a recorded intent plus a venue deep link is reception/transmission before EU launch; if counsel says it crosses the line, remove the feature or reframe it as a saved watchlist note and fail closed at the jurisdiction gate. Keep all copy generic decision support — no personalised advice or discretionary management, which are separately licensed under (h)–(i).

### R-003 — UK regime timing and sequencing

- **Description:** UK applications opened 30 September 2026 with the regime expected in force 25 October 2027, so UK authorisation may not be available at MVP launch and the rules may still move. Treating the UK as a launch market without a confirmed FCA position risks unauthorised marketing or data collection.
- **Likelihood / Impact:** High / High
- **Owner:** Compliance Agent
- **Treatment:** Mitigate by sequencing; accept schedule risk
- **Launch blocker:** No, if the UK stays gated off; yes if the UK is marketed at launch.
- **Response:** Launch EU first. Keep the UK switched off in the jurisdiction gate as "not yet available" and collect no UK user data or marketing consent until the FCA position is confirmed. Re-evaluate at delivery planning against the expected 2027 in-force date. Contingency: EU-only launch with UK added after authorisation; do not let UK scope delay the EU build.

### R-004 — 99.9% availability on a single-region Docker Swarm

- **Description:** The NFR commits to 99.9% monthly availability for paid plans (43m 12s of allowed downtime), but the confirmed hosting is commodity cloud VPS running Docker Swarm in one region, with Supabase as a managed dependency. A single provider in a single region cannot underwrite that number, and Swarm has limited self-healing compared with a managed orchestrator.
- **Likelihood / Impact:** High / High
- **Owner:** Architect Agent, with DevSecOps
- **Treatment:** Mitigate; accept the residual single-region risk explicitly
- **Launch blocker:** Yes for the paid-plan SLA claim.
- **Response:** Run at least three Swarm manager/worker nodes across two availability zones from one provider, with health checks, restart policies, rolling updates and no single-node data path. Serve a read-only degraded mode from cached last-known prices with a staleness badge when a dependency fails. Scope the 99.9% commitment to paid plans only, as stated. If the provider's own SLA is below 99.9%, either budget a second provider/warm standby or downgrade the paid-plan SLA to best-effort with product sign-off. Add external synthetic checks per region and a monthly availability review; test the documented RPO ≤24h / RTO ≤4h restore.

### R-005 — Exchange credential compromise and cross-user data leakage

- **Description:** Users supply read-only exchange API keys and wallet addresses. A leak of keys, or a missing ownership check on a service endpoint, exposes one user's balances and trade history to another. Services use one Postgres role per service with no cross-schema grants and no RLS, so authorization is enforced entirely in application code — a single missed check is a cross-tenant breach.
- **Likelihood / Impact:** Medium / Critical
- **Owner:** Architect Agent, with DevSecOps
- **Treatment:** Mitigate
- **Launch blocker:** Yes.
- **Response:** Encrypt exchange credentials per user in Supabase Vault (pgsodium); never in env files, source, logs or error payloads. Request read-only scopes only. Derive `user_id` server-side from the validated Supabase JWT — never from a request body. Add a cross-tenant test per endpoint as a merge gate. Enforce MFA for any user with exchange credentials, append-only audit of key add/use/delete, and a tested key-rotation and incident runbook. Treat any suspected exposure as a notify-and-rotate event.

### R-006 — Support-access misuse

- **Description:** Support staff will need to diagnose "my numbers are wrong" tickets, which requires viewing user portfolios and connection state. Standing access, shared credentials, or screenshots of raw data create insider risk and a GDPR exposure, and the product's trust positioning makes misuse reputationally fatal.
- **Likelihood / Impact:** Medium / Critical
- **Owner:** DevSecOps, with Compliance
- **Treatment:** Mitigate
- **Launch blocker:** Yes before any support tooling ships.
- **Response:** No standing production data access. Break-glass access is time-boxed, approved by a second person, requires a ticket reference and reason, and is written to the append-only audit log. Support tooling shows masked values by default and never displays raw API keys or full wallet addresses. Alert on every support-access session; review sessions weekly. Document a customer-notification path.

### R-007 — Stale prices and stale FX

- **Description:** The success metric requires ≤5-minute freshness for top-100 assets and ≤15 minutes for the tail, and >99% valuation accuracy. Vendor tiers differ (CoinGecko 60s free to real-time from Analyst; FX from daily to 60s), vendors fail, and a stale quote silently presented as current directly breaks the accuracy claim and the P&L number.
- **Likelihood / Impact:** High / High
- **Owner:** Architect Agent
- **Treatment:** Mitigate
- **Launch blocker:** Yes.
- **Response:** Dual-source CoinGecko Analyst primary with CoinMarketCap cross-check and divergence alerting. Use exchangerate.host Business (60-second) rather than a daily tier, and persist the FX rate and its timestamp on every valuation. Surface a staleness badge whenever age exceeds 5 minutes (top-100) or 15 minutes (tail). Fall back to last-known price with the badge, never to a fabricated current price. Alert when the newest quote or FX rate breaches its freshness budget. Validate >99% accuracy against a manual reference set before launch.

### R-008 — Supabase limits or outage

- **Description:** Supabase-managed Postgres and Auth are single points of failure for identity and all service data. Plan limits (DB size, connections, Auth MAU, Vault operations) are easy to exceed at launch, and an outage blocks sign-in and every service at once.
- **Likelihood / Impact:** Medium / High
- **Owner:** Architect Agent
- **Treatment:** Mitigate; accept the residual single-provider risk
- **Launch blocker:** Yes for launch readiness.
- **Response:** Size the plan against launch forecasts for DB size, peak connections, Auth MAU and Vault use; use connection pooling and per-service credentials. Verify backup/PITR actually restores within RPO ≤24h / RTO ≤4h. Design a degraded read-only mode and a status-monitoring alert. Document the manual failover decision and the customer communication path. Re-check limits at each service go-live.

### R-009 — Silent job failure

- **Description:** Price refresh, FX refresh, valuation recompute and reconciliation run as scheduled jobs. With no message queues in the fixed stack, a job that stops running has no dead-letter queue to surface it; the product simply serves stale numbers while appearing healthy.
- **Likelihood / Impact:** High / High
- **Owner:** Architect Agent, with Quality
- **Treatment:** Mitigate
- **Launch blocker:** Yes.
- **Response:** The Job Orchestration service writes a heartbeat and `last_success_at` per job on every run. A dead-man's-switch alert fires when `last_success_at` exceeds 2× the schedule interval. Use idempotency keys plus bounded retries for notifications and recompute, and per-venue circuit breakers. Structured logs and metrics per job; an operator runbook for each. Test by killing a scheduled job and confirming the alert fires and the staleness badge appears.

### R-010 — Wrong decimals and precision

- **Description:** Balances arrive in atomic units with per-chain decimals; display, valuation and P&L use decimal money. Mixing atomic and whole units, or using binary floats, produces wrong valuations that are plausible-looking — the exact failure mode the >99% accuracy metric exists to prevent.
- **Likelihood / Impact:** High / Critical
- **Owner:** Architect Agent, with Quality
- **Treatment:** Mitigate
- **Launch blocker:** Yes.
- **Response:** `rust_decimal` end-to-end; no `f64` in domain or money paths. The canonical asset registry stores chain decimals, display decimals and the conversion rule. Normalise at the adapter boundary and never mix scales inward. Property-based and unit tests with known vectors per chain family and per venue, plus a golden-file reconciliation test. Impossible balances raise a reconciliation state rather than rendering. The >99% valuation-accuracy acceptance test gates launch.

### R-011 — 6–9 month timeline vs scope

- **Description:** Build-vs-buy estimates 64–114 engineer-weeks (~15–27 engineer-months). A 3–5 engineer team over 6–9 months provides 18–45 engineer-months, so the plan fits only at the low end of scope and the high end of staffing. Eight services plus the gateway, five venue adapters, four chain families and the compliance surface in one MVP is aggressive for a greenfield team.
- **Likelihood / Impact:** High / High
- **Owner:** Delivery/Product Agent, with Architect
- **Treatment:** Mitigate
- **Launch blocker:** Yes if scope is not frozen.
- **Response:** Build the thinnest end-to-end slice first (auth → one venue → asset registry → valuation → dashboard), one service at a time behind the API Gateway. Freeze launch scope at 5 venues and 4 chain families; keep DeFi/staking/LP, tax filing, mobile, advisor portals and multi-user post-MVP. Agree a written cut list and a monthly scope/burn checkpoint; re-baseline at each checkpoint rather than at month nine.

### R-012 — USD 150,000 non-salary budget

- **Description:** The envelope covers vendors, infrastructure, legal and contingency; salaries are separately funded. Vendor run-rate is ~USD 357–557/month plus ~USD 50–150/month Esplora hosting, which is small. The unpriced exposure is EU and UK external counsel, a DPIA, and any enterprise data contract or per-jurisdiction licensing advice — exactly the lines the launch depends on.
- **Likelihood / Impact:** Medium / High
- **Owner:** Delivery/Product Agent
- **Treatment:** Mitigate
- **Launch blocker:** No, unless legal or an enterprise data contract breaches the contingency.
- **Response:** Hold data vendors to list-price monthly plans (CoinGecko Analyst USD 129, CoinMarketCap Startup USD 79, FX USD 99.99, RPC cap ~USD 200 overage) with no annual enterprise commitment pre-MVP. Reserve 20–25% of the envelope for legal and contingency and track it as a separate line. Review burn monthly; any contract above the remaining contingency requires explicit approval. Do not let CoinTracker/CoinGecko enterprise pricing enter the MVP plan.

### R-013 — Exchange API churn, rate limits and bans

- **Description:** Exchange APIs change without notice (Binance has retired streams and schemas; Coinbase's Pro-to-Advanced-Trade migration invalidated keys). Binance meters per IP and escalates 429 to an HTTP 418 ban of 2 minutes to 3 days; Kraken meters per key with a decaying counter. A ban or a silent broken connection removes a venue from the user's total and corrupts the reconciliation claim.
- **Likelihood / Impact:** High / High
- **Owner:** Architect Agent
- **Treatment:** Mitigate
- **Launch blocker:** Yes for the launch coverage and accuracy claim.
- **Response:** One least-privilege adapter per venue behind the `ExchangeReader` port, with contract tests and a per-venue circuit breaker. Honour 429/418 backoff and never retry into a ban; cache reads. A failed connection becomes a visible reconciliation state, never a silent zero. Per-venue key-rotation and deprecation runbook; adapter changes ship behind the port without domain edits. Monitor per-venue error and latency.

---

## Assumptions

### A-001 — The budget excludes salaries

- **Description:** USD 150,000 covers non-salary spend (vendors, infrastructure, legal, contingency); the 3–5 engineer team is already funded. This resolves the apparent conflict between the envelope and the USD 150k–405k fully loaded build estimate.
- **Confidence / Impact if false:** Confirmed by answer Q2/Q5 / Critical — the whole delivery plan is unaffordable if salaries must fit.
- **Owner:** Delivery/Product Agent
- **Treatment:** Accept as confirmed; validate by cost plan
- **Response:** Record the salary exclusion in the cost plan and report non-salary burn separately. Re-open if finance cannot confirm the team is funded.

### A-002 — Delivery shape is 3–5 engineers, one service at a time, 6–9 months

- **Description:** Greenfield, sequential service delivery behind the API Gateway, MVP in 6–9 months.
- **Confidence / Impact if false:** Confirmed by answer Q1 / High — parallel delivery would change the dependency DAG and the budget.
- **Owner:** Delivery/Product Agent
- **Treatment:** Accept; validate at delivery planning
- **Response:** Convert to a unit DAG and a staffing plan; if staffing falls below three engineers, re-cut scope at the first checkpoint.

### A-003 — Supabase-managed Postgres and Auth meet the NFRs

- **Description:** Supabase provides availability, backup, connection and Auth capacity adequate for launch; per-service schema/role with no RLS is sufficient because services mediate all access.
- **Confidence / Impact if false:** High confidence / Critical — identity and data availability depend on it.
- **Owner:** Architect Agent
- **Treatment:** Validate
- **Response:** Load-test connection and query paths, verify plan limits and PITR restore, and review the per-service role model before the first service ships.

### A-004 — Public chain RPC and open-source indexers suffice at launch

- **Description:** Public RPC endpoints (with Alchemy/Infura failover) and self-hosted Esplora cover EVM, Solana, Bitcoin and Tron at launch volumes; paid RPC is a reliability refinement, not a launch requirement.
- **Confidence / Impact if false:** Medium / High — rate limits or outages would break on-chain balances and valuation.
- **Owner:** Architect Agent
- **Treatment:** Validate; mitigate with failover
- **Response:** Measure rate limits and latency per chain against launch forecasts; wire the failover provider from day one; keep the paid RPC line in the contingency.

### A-005 — Retail market-data tiers meet the freshness targets

- **Description:** CoinGecko Analyst (real-time), CoinMarketCap Startup and exchangerate.host Business (60s) meet ≤5 min top-100 and ≤15 min tail freshness and the FX timestamp requirement.
- **Confidence / Impact if false:** Medium / High — the freshness and accuracy metrics fail.
- **Owner:** Architect Agent
- **Treatment:** Validate
- **Response:** Sample freshness and timestamp behaviour per tier before commitment; keep the dual-source and staleness-badge design regardless.

### A-006 — Read-only exchange keys are the only viable access path

- **Description:** There is no PSD2-style access right to exchange accounts, so integration relies on user-created read-only API keys; exchange partner programmes are contractual and gated and are not available at launch scale.
- **Confidence / Impact if false:** High confidence / High — an access right would simplify onboarding but cannot be assumed.
- **Owner:** Architect Agent, with Compliance
- **Treatment:** Accept
- **Response:** Build the read-only key flow, least-privilege guidance and revocation handling; re-check the access-right position at each jurisdiction review.

### A-007 — 5 venues and 4 chain families is a credible launch claim

- **Description:** Launch coverage of five exchange venues and four chain families (EVM, Solana, Bitcoin, Tron) is sufficient to demonstrate cross-venue reconciliation, even though it is below the 200–700-integration table stakes.
- **Confidence / Impact if false:** Medium / High — breadth pressure would break the timeline.
- **Owner:** Product Agent, with Architect
- **Treatment:** Accept; validate with positioning
- **Response:** Position on reconciliation quality, not breadth; hold the roadmap line for expansion and do not add a sixth venue pre-launch without cutting something else.

### A-008 — MiCA's non-custodial software exclusion covers the descriptive product

- **Description:** Non-custodial wallet software is excluded from MiCA (Recital 83), and generic decision support with a recorded order intent stays outside CASP licensing provided no order is transmitted and no personalised advice is given.
- **Confidence / Impact if false:** Medium / Critical — a licensing requirement would stop EU launch.
- **Owner:** Compliance Agent, with external counsel
- **Treatment:** Validate; avoid the licensed activity
- **Response:** Obtain counsel's written opinion before EU launch; keep the feature boundary explicit in the design and copy; fail the jurisdiction gate if the opinion is adverse.

### A-009 — The jurisdiction gate can be enforced before data collection

- **Description:** The product can determine and record a user's jurisdiction (attestation plus IP/geo signal) and block onboarding and data collection in unsupported jurisdictions before any personal data is processed.
- **Confidence / Impact if false:** High confidence / Critical — GDPR and MiCA exposure from day one.
- **Owner:** Architect Agent, with Compliance
- **Treatment:** Validate
- **Response:** Implement the gate as the first step in onboarding and as a server-side check in the gateway; test the fail-closed path (unknown jurisdiction = blocked) before any data collection.

### A-010 — The fixed stack and Docker Swarm fit the team and the workload

- **Description:** Rust/Axum/sqlx plus React/Vite and Docker Swarm are workable for the team; no AWS dependency is needed and the aws-platform lens is applied to Swarm/Supabase/VPS rather than AWS services.
- **Confidence / Impact if false:** High confidence / Medium — stack changes would invalidate estimates.
- **Owner:** Architect Agent
- **Treatment:** Accept
- **Response:** Confirm team skills during delivery planning; keep the clean-architecture service template minimal so a new service is a day, not a week.

### A-011 — Vendor list pricing and monthly terms hold through MVP

- **Description:** Vendor costs remain at published list (CoinGecko USD 129/mo, CoinMarketCap USD 79/mo, FX USD 99.99/mo, Helius from USD 49/mo, RPC overage ~USD 200/mo) and contracts stay monthly-cancellable.
- **Confidence / Impact if false:** Medium / Medium — a price change or annual commitment eats the contingency.
- **Owner:** Delivery/Product Agent
- **Treatment:** Validate before signing
- **Response:** Confirm pricing and term in writing at contract time; no annual enterprise commit pre-MVP; budget a 10–20% vendor price buffer.

---

## Issues

### I-001 — No written redistribution/derived-valuation licence from any vendor

- **Severity:** Critical (open, launch-blocking)
- **Owner:** Architect Agent, with external counsel
- **Treatment:** Resolve
- **Response:** Vendor legal requests out now (R-001); decision required before Market Data implementation. Owner reports status weekly. No production ingestion until signed.

### I-002 — No counsel opinion on MiCA order-intent treatment

- **Severity:** Critical (open, launch-blocking)
- **Owner:** Compliance Agent
- **Treatment:** Resolve
- **Response:** Engage EU counsel, scope the opinion to recorded intent plus deep link, and set a decision date before EU launch. Until then the feature stays design-only.

### I-003 — No DPIA for wallet addresses and keyed hashes

- **Severity:** Critical (open, blocks EU data collection)
- **Owner:** Compliance Agent
- **Treatment:** Resolve
- **Response:** Complete a DPIA covering wallet addresses, keyed hashes, exchange metadata and support access before the jurisdiction gate opens; record lawful basis and retention.

### I-004 — No cost plan mapping the USD 150k to vendor, legal and hosting lines

- **Severity:** High (open)
- **Owner:** Delivery/Product Agent
- **Treatment:** Resolve
- **Response:** Produce a line-item non-salary plan with contingency and monthly burn; reconcile against the build-vs-buy run-rate (USD 357–557/mo) and legal reserve.

### I-005 — 99.9% availability is untested on the chosen topology

- **Severity:** High (open)
- **Owner:** Architect Agent
- **Treatment:** Resolve
- **Response:** Build the multi-node/two-AZ Swarm reference, run a failure drill and synthetic checks, and either meet 43m 12s or downgrade the paid-plan SLA with product sign-off (R-004).

### I-006 — Greenfield with no walking skeleton

- **Severity:** High (open)
- **Owner:** Delivery/Product Agent, with Architect
- **Treatment:** Resolve
- **Response:** Ship the thinnest end-to-end slice (auth → one venue → asset registry → valuation → dashboard) within the first 4–6 weeks; use it to validate the contract, the JWT path and the valuation pipeline.

### I-007 — Launch coverage sits below category table stakes

- **Severity:** Medium (accepted)
- **Owner:** Product Agent
- **Treatment:** Accept
- **Response:** Make reconciliation quality the launch claim and publish the expansion roadmap; revisit after launch with retention data (R-011, A-007).

---

## Dependencies

### D-001 — Canonical Asset Registry (launch-blocking)

- **Description:** Single source of canonical asset identity: symbol, chain, contract address, chain decimals, display decimals, vendor asset-ID mappings. Every balance, quote and P&L row joins on it; a wrong decimal or mapping propagates into every number (amplifies R-010).
- **Criticality / Owner:** Launch-blocking / Architect Agent
- **Treatment:** Manage on the critical path
- **Response:** Build first, version the mapping, and gate changes with a review plus a golden-vector test. Blocks D-002, D-004, D-005, D-008.

### D-002 — Market-data licence and provider adapters (launch-blocking)

- **Description:** Written redistribution and derived-valuation rights plus a `MarketDataProvider` adapter (CoinGecko primary, CoinMarketCap cross-check) behind a port.
- **Criticality / Owner:** Launch-blocking / Architect Agent, with counsel
- **Treatment:** Manage; escalate if not granted
- **Response:** Licence first (R-001, I-001); adapter second. Blocks D-004 (valuation) and any price display.

### D-003 — FX rate provider (launch-blocking for local-currency P&L)

- **Description:** `FxProvider` adapter against exchangerate.host Business with a persisted rate timestamp per valuation.
- **Criticality / Owner:** Launch-blocking / Architect Agent
- **Treatment:** Manage
- **Response:** Contract and adapter before local-currency P&L ships; daily-tier data is insufficient for the freshness target (R-007).

### D-004 — Valuation service (launch-blocking)

- **Description:** Computes portfolio value from balances, quotes and FX at a single valuation timestamp, with staleness metadata.
- **Criticality / Owner:** Launch-blocking / Architect Agent
- **Treatment:** Manage
- **Response:** Depends on D-001, D-002, D-003. Blocks D-006 and D-007.

### D-005 — Exchange read-only adapters (launch-blocking)

- **Description:** One least-privilege `ExchangeReader` adapter per launch venue (5), each mapping venue assets onto canonical IDs and surfacing connection health.
- **Criticality / Owner:** Launch-blocking / Architect Agent
- **Treatment:** Manage
- **Response:** Depends on D-001. Feeds D-006 (reconciliation). Adapter contract tests are the merge gate (R-013).

### D-006 — Transfer matching and reconciliation queue (critical)

- **Description:** Cross-venue matching, duplicate suppression, missing-cost-basis detection and a user-visible resolution queue — the differentiator and the evidence for the >99% accuracy claim.
- **Criticality / Owner:** Critical / Architect Agent
- **Treatment:** Manage
- **Response:** Depends on D-004 and D-005. Feeds D-007 (cost basis/P&L) and the SPA reconciliation screen.

### D-007 — Cost-basis lot engine and P&L (launch-blocking for the value proposition)

- **Description:** Domain-owned lot selection, fees, transfers and missing-basis detection; jurisdiction-agnostic, with display currency applied at the edge.
- **Criticality / Owner:** Launch-blocking / Architect Agent
- **Treatment:** Manage
- **Response:** Depends on D-004 and D-006. Gates the dashboard P&L and reporting surfaces.

### D-008 — Chain RPC and indexer adapters (critical)

- **Description:** `ChainRpcProvider` adapters for EVM, Solana, Bitcoin (self-hosted Esplora) and Tron, with failover, confirmation handling and token discovery.
- **Criticality / Owner:** Critical / Architect Agent
- **Treatment:** Manage
- **Response:** Depends on D-001. Feeds on-chain balances into D-004.

### D-009 — Supabase Auth and API Gateway JWT path (launch-blocking)

- **Description:** SPA sign-in via Supabase Auth; the gateway forwards the Supabase JWT unchanged and every service validates it; user identity is derived server-side.
- **Criticality / Owner:** Launch-blocking / Architect Agent, with DevSecOps
- **Treatment:** Manage
- **Response:** First cross-cutting dependency; blocks every authenticated service and the cross-tenant test suite (R-005).

### D-010 — Job Orchestration with heartbeat and idempotency (launch-blocking)

- **Description:** Scheduler for price/FX refresh, valuation recompute and reconciliation, with heartbeat, `last_success_at`, bounded retries and idempotency keys; no message queues.
- **Criticality / Owner:** Launch-blocking / Architect Agent
- **Treatment:** Manage
- **Response:** Depends on D-002/D-003/D-004. Enables the freshness SLA and the staleness badge (R-009).

### D-011 — Audit service (critical)

- **Description:** Append-only audit of sensitive actions: credential add/use/delete, support access, jurisdiction decisions, order-intent records.
- **Criticality / Owner:** Critical / DevSecOps, with Architect
- **Treatment:** Manage
- **Response:** Needed before support tooling or credential features ship (R-005, R-006).

### D-012 — Billing and entitlements (critical for the paid-plan SLA)

- **Description:** Subscription plans, quotas and entitlements; the 99.9% availability commitment applies to paid plans, so entitlements must be reliable and auditable.
- **Criticality / Owner:** Critical / Architect Agent
- **Treatment:** Manage
- **Response:** Depends on D-009. Can follow the free-tier MVP but must land before the paid-plan SLA is advertised.

### D-013 — External counsel and jurisdiction gate (launch-blocking)

- **Description:** Written MiCA/data-protection opinion, UK position, disclosures, DPIA and a server-side jurisdiction gate that fails closed before any data collection.
- **Criticality / Owner:** Launch-blocking / Compliance Agent, with counsel
- **Treatment:** Manage; escalate adverse opinions
- **Response:** Engage counsel now; gate the EU launch on the opinion and the DPIA (I-002, I-003). Blocks onboarding and any production data collection.

### D-014 — Parchment Ledger design system and SPA shell (planned, parallel)

- **Description:** Tokens, components, PWA shell and WCAG 2.2 AA patterns the SPA screens render against.
- **Criticality / Owner:** Planned / Frontend lead, with Architect
- **Treatment:** Manage in parallel
- **Response:** Build alongside D-009; no business math in the SPA — it renders gateway responses only.

---

## Traceability

- Budget semantics and delivery shape: `feasibility-questions.md` Q1, Q2, Q5; intent statement Q4.
- Hosting: `feasibility-questions.md` Q3; project rules (Supabase, Docker Swarm, no AWS).
- Jurisdictions and order intents: `feasibility-questions.md` Q4; intent statement Q4; `competitive-analysis.md` Execution and Order-Intent Landscape; `market-trends.md` Regulatory and Compliance Shifts.
- Market-data rights and vendor costs: `build-vs-buy.md` Market Data and FX Layer, Cost and Operational Risk; `market-trends.md` Regulatory and Compliance Shifts; `competitive-analysis.md` Assumptions.
- Availability, freshness, accuracy, audit, MFA, RPO/RTO: intent statement Success Metrics; project NFR anchors.
- Scope and coverage: `build-vs-buy.md` Recommended Composition, Build Cost and TCO; `market-trends.md` Implications for the MVP; intent statement Initial Scope Signal; memory M1 (testable acceptance criteria) and M2 (walking skeleton).
- Venue/chain API fragility: `build-vs-buy.md` Exchange Integration Layer.
