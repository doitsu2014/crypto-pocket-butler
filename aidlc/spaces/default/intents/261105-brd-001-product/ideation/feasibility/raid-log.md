# RAID Log — Feasibility & Constraints

**Intent:** `261105-brd-001-product` · **Stage:** `ideation/feasibility` · **Status:** canonical (synthesised)
**Basis:** intent statement, confirmed answers Q1–Q8, competitive analysis, market trends, build-vs-buy, `docs/architecture/README.md`, the BRD set (especially BRD-012/016/017/019/020) and `DECISIONS.md`.
**Scales:** Likelihood High/Medium/Low · Impact Critical/High/Medium/Low · Issue severity Critical/High/Medium · Dependency criticality Launch-blocking/Critical/Planned.
**Treatment values:** mitigate · transfer · accept · avoid (a row may combine values where it names which part each applies to).
**Row fields:** every entry carries an accountable owner role, a due date or review cadence, and a per-row source/evidence reference. Dates are planning gates anchored to the confirmed 9-month target (EU launch 2027-08-06) and are re-baselined at each monthly checkpoint.

---

## Risks

### R-001 — Market-data, exchange and benchmark rights are not granted

- **Due / cadence:** 2026-11-28 (licence requests out); weekly until signed · **Source:** BRD-012 NFR-LEG-05/06; BRD-020 OQ-002; confirmed Q4; `build-vs-buy.md` Market Data and FX Layer

- **Description:** The core surfaces — cross-venue valuation, local-currency P&L, stored price history and charts — are derived works that also require **storage and retention**. CoinGecko's terms forbid redistribution, syndication, storage and derived valuations without consent, require "Powered by CoinGecko" attribution, and say caching/storage is not encouraged (refresh at least every 24 hours). CoinMarketCap's tiers include commercial-use rights, but its display, storage and derivative-work terms have not been reviewed by counsel. **Separately**, exchange API commercial-use and retention terms for storing full trade history were explicitly not researched, and benchmark index terms are open (BRD-020 OQ-002). No vendor has granted worldwide redistribution, storage and derived-valuation rights at a retail price, and no exchange term has been cleared.
- **Likelihood / Impact:** High / Critical
- **Owner:** Principal Architect (accountable); External Counsel (delivery)
- **Treatment:** mitigate
- **Launch blocker:** Yes — cannot ingest, store or display vendor data in the EU/UK until resolved; cannot ship the cost-basis/P&L core if exchange retention is forbidden.
- **Response:** Send written licence requests now to CoinGecko, CoinMarketCap, each launch venue and the benchmark provider for worldwide **display, storage, history, redistribution and derived-valuation** rights, EU and UK first, with the exact display surfaces attached. Keep a licence register with evidence, attribution and review cadence; sources without a licence reference cannot be activated (D-002, I-001). Do not implement the Market Data adapter beyond a sandbox until a written grant exists; a sandbox is not a licence. Fallbacks in order: (1) CoinMarketCap (or another licensed source) as primary for valuation and storage if its terms grant derivatives in writing; (2) drop derived valuation and show vendor-sourced raw quotes only with attribution — which removes the differentiator and must be escalated to the approval gate; (3) enterprise vendor (Kaiko/CoinDesk Data) post-MVP. Track as a weekly decision item with a named legal owner; no production ingestion or retention before signature.

### R-002 — MiCA treatment of user-initiated order intents

- **Due / cadence:** 2027-01-15 (EU counsel opinion before M1); quarterly regulatory review · **Source:** BRD-002 AC-USR-11; MiCA Art 3(1)(16); confirmed Q4; `market-trends.md` Regulatory and Compliance Shifts

- **Description:** Execution and reception/transmission of orders are licensed CASP activities under MiCA Art 3(1)(16)(e) and (g). The product's "order intent" — a user records a buy/sell intent against a cross-venue view and never places the order — is designed as a documented handoff to a licensed venue, but the line between a recorded note with a deep link and "reception/transmission" is not settled. Grandfathering ended 1 July 2026, so no transitional cover exists. **The legal basis must not rely on the Recital 83 non-custodial wallet-software exclusion**: CPB is a hosted, operator-run aggregator, not wallet software. The defensible basis is that the product supplies none of the Art 3(1)(16) services, which counsel must confirm service by service.
- **Likelihood / Impact:** Medium / Critical
- **Owner:** Head of Compliance (accountable), with External Counsel
- **Treatment:** avoid (licensed activity by design); mitigate (residual interpretation risk)
- **Launch blocker:** Yes for EU launch.
- **Response:** Implement the intent as a user-authored, non-transmitted record: no order routing, no order book, no trade-enabled exchange credentials, no execution API, and human approval on every intent. Request only read-only scopes. Obtain a written EU counsel opinion analysing each Art 3(1)(16) service by name before EU launch; if counsel says a recorded intent plus a venue deep link crosses the line, remove the feature or reframe it as a saved watchlist note and fail closed at the jurisdiction gate. Keep all copy generic decision support — no personalised advice or discretionary management, which are separately licensed under (h)–(i) — and run the MiCA Art 66 marketing review.

### R-003 — UK regime timing, current perimeter and sequencing

- **Due / cadence:** 2027-06-30 (dated UK go/no-go before EU launch); re-run 2027-10-25 at the new regime's in-force date · **Source:** confirmed Q4 and Q7; BRD-002 AC-USR-11; FSMA 2000 s.21 and RAO 2001 Art 25/53; `market-trends.md`

- **Description:** The UK has **two** exposures, and the earlier draft addressed only one. (a) **Current regime:** s.21 FSMA 2000 and the FSMA (Financial Promotion) Order 2005 restrict cryptoasset financial promotions to UK consumers (FCA regime since October 2023), and RAO 2001 Art 25 (arranging) / Art 53 (advice) may catch order intents, venue deep links, affiliate/referral revenue and marketing copy. (b) **Future regime:** applications opened 30 September 2026, expected in force around 25 October 2027, so a 9–11 month MVP from November 2026 lands between August and October 2027 — just before or at commencement — and the new regime becomes a narrow post-launch gate. Treating the UK as a launch market without a confirmed current-regime position risks unauthorised marketing or data collection.
- **Likelihood / Impact:** High / High
- **Owner:** Head of Compliance (accountable)
- **Treatment:** mitigate (sequencing and dated go/no-go); accept (schedule risk)
- **Launch blocker:** Yes for UK marketing/onboarding; the UK remains a gated launch market (Q4), not a market removed from scope.
- **Response:** **Do not silently override confirmed answer Q4.** Keep the UK as a gated launch market with a dated go/no-go (**2027-06-30**) and the Q7 launch condition: **the UK goes live only when counsel signs off the current-regime opinion and the marketing/onboarding plan**. If the opinion is adverse or the capped legal budget cannot support it, gate UK marketing and onboarding while the EU proceeds, and escalate any change to **pre-positioning market** at the approval gate as a recorded decision; never switch the UK off or downgrade it as a risk-owner action. In the meantime: obtain a written UK opinion covering the current promotion/arranging position **and** the future authorisation regime; gate UK marketing (SPA copy, SEO, app-store listings, email, affiliate/referral) as well as UK onboarding; collect no UK user data or marketing consent until the position is confirmed. At the expected 2027 in-force date, re-run the gate. Do not let UK scope delay the EU build.

### R-004 — 99.9% availability on a single-region Docker Swarm

- **Due / cadence:** 2027-06-15 (topology and drills before paid-plan launch); monthly availability review · **Source:** BRD-012 NFR-AVL-01–06; CON-014; confirmed Q3; architecture §6–§8

- **Description:** The NFR commits to 99.9% monthly availability for paid plans (43.2 minutes in a 30-day month; 44.6 in a 31-day month), but the confirmed hosting is commodity cloud VPS running Docker Swarm in one region, with Supabase as a managed dependency. A single provider in a single region cannot underwrite that number, and Swarm has limited self-healing compared with a managed orchestrator. **A two-manager cluster or three managers split 2+1 across two availability zones loses Raft quorum when the zone holding the majority fails, taking down every service; two nodes cannot provide both quorum and service redundancy.** The ingress and the region are single points of failure, and Supabase Auth is on every authenticated request with no fallback.
- **Likelihood / Impact:** High / High
- **Owner:** Principal Architect (accountable), with DevSecOps Lead
- **Treatment:** mitigate (product-attributable availability); transfer (upstream-provider outages under the contractual SLA scope); accept (residual single-region risk)
- **Launch blocker:** Yes for the paid-plan SLA claim.
- **Response:** Run **three manager nodes, each in a distinct failure domain**, plus **at least three workers**, with two-plus replicas of every service spread across workers, health checks, `start-first` rolling updates, redundant ingress/LB and no single-node data path. Add an **external synthetic monitor independent of the VPS provider and Supabase**, checking the public health endpoint, sign-in and one authenticated read, with an out-of-band alert channel. Serve a read-only degraded mode from cached last-known prices with a staleness badge when a dependency fails. **Scope the 99.9% commitment to product-attributable time with upstream-provider outages explicitly excluded**, evidenced by a measured node-kill/replica-kill drill, a documented Supabase SLA and a written SLA-scope statement. Do **not** restate the Must NFR as "best-effort with product sign-off"; the valid resolutions are redundancy (a second cluster/warm standby, currently unfunded) or the explicit upstream exclusion. Add a monthly availability review and the RPO ≤24h / RTO ≤4h restore drill.

### R-005 — Exchange credential compromise, Vault access and cross-user data leakage

- **Due / cadence:** 2026-11-21 (week-two Vault spike); cross-tenant tests at every merge; quarterly key-access review · **Source:** BRD-012 NFR-SEC-01/02/03/05/06/07/11/12/14; CON-034; confirmed Q3; architecture §7

- **Description:** Users supply read-only exchange API keys and wallet addresses. A leak of keys, or a missing ownership check on a service endpoint, exposes one user's balances and trade history to another. Services use one Postgres role per service with no cross-schema grants and no RLS, so authorization is enforced entirely in application code — a single missed check is a cross-tenant breach. **The Vault path is not settled:** pgsodium has no Rust binding, a service role cannot read `vault.decrypted_secrets` under the no-cross-schema rule, and a naive read exposes every secret the service role can see. A compromised host or provider account reaches node memory and disk where Vault-derived plaintext lives.
- **Likelihood / Impact:** Medium / Critical
- **Owner:** Principal Architect (accountable), with DevSecOps Lead
- **Treatment:** mitigate
- **Launch blocker:** Yes.
- **Response:** Resolve the Vault access pattern in the week-two spike (C3): a `SECURITY DEFINER` platform accessor granting only `EXECUTE` to each service role; a versioned KEK with per-subject wrapped data keys; rotation without restart; a stated unreachable-Vault behaviour; a decision on key backup versus crypto-shredding. Encrypt exchange credentials per user; never in env files, source, logs or error payloads. **Per-user isolation must be explicit**: a service-level read that returns all secrets is prohibited. Request read-only scopes only. Derive `user_id` server-side from the validated Supabase JWT — never from a request body. Add a cross-tenant test per endpoint as a merge gate. Enforce MFA for any user with exchange credentials, append-only audit of key add/use/delete, and a tested key-rotation and incident runbook. Apply host least privilege (CON-034) so a stolen SSH key or provider account cannot reach the decryption path. Treat any suspected exposure as a notify-and-rotate event.

### R-006 — Support-access misuse

- **Due / cadence:** 2027-06-30 (before any support tooling or public beta); weekly access-session review · **Source:** BRD-012 NFR-SEC; BRD-017 FR-PRV-03/06/07/13/15; D-011

- **Description:** Support staff will need to diagnose "my numbers are wrong" tickets, which requires viewing user portfolios and connection state. Standing access, shared credentials, or screenshots of raw data create insider risk and a GDPR exposure, and the product's trust positioning makes misuse reputationally fatal.
- **Likelihood / Impact:** Medium / Critical
- **Owner:** DevSecOps Lead (accountable), with Head of Compliance
- **Treatment:** mitigate
- **Launch blocker:** Yes before any support tooling ships.
- **Response:** No standing production data access. Break-glass access is time-boxed, approved by a second person, requires a ticket reference and reason, and is written to the append-only audit log. Support tooling shows masked values by default and never displays raw API keys or full wallet addresses. Alert on every support-access session; review sessions weekly. Document a customer-notification path. If the Support Console is deferred, the read-only admin view must carry the same break-glass, time-bound, reason-recorded controls — deferral is not a compliance exemption.

### R-007 — Stale prices, stale FX and historical FX

- **Due / cadence:** 2027-03-31 (before Market Data go-live); daily freshness-alert review · **Source:** BRD-012 NFR-PERF-01/02; intent statement Success Metrics; BRD-012 NFR-LEG-05

- **Description:** The success metric requires ≤5-minute freshness for top-100 assets and ≤15 minutes for the tail, with top-asset collection at 1-minute cadence, and >99% valuation accuracy. Vendor tiers differ, vendors fail, and a stale quote silently presented as current directly breaks the accuracy claim and the P&L number. **Historical FX is a separate gap:** exchangerate.host Business historical data is end-of-day only, so local-currency cost basis and P&L at transaction timestamps have no stated intraday source.
- **Likelihood / Impact:** High / High
- **Owner:** Principal Architect (accountable)
- **Treatment:** mitigate
- **Launch blocker:** Yes.
- **Response:** **Use CoinMarketCap (or another licensed source) as the primary for valuation** and CoinGecko as display/attribution or secondary cross-check — the reverse of the earlier draft — because CoinGecko forbids derived valuations and storage while CMC's tiers include commercial-use rights pending counsel review. Dual-source divergence alerting; use a 60-second FX tier (Business or equivalent) and persist the FX rate and its timestamp on every valuation. Surface a staleness badge whenever age exceeds 5 minutes (top-100) or 15 minutes (tail). Fall back to last-known price with the badge, never to a fabricated current price. Alert when the newest quote or FX rate breaches its freshness budget. Validate >99% accuracy against a manual reference set before launch. Add a historical FX source or state the EOD-only limitation and its effect on P&L accuracy.

### R-008 — Supabase limits or outage

- **Due / cadence:** 2026-12-19 (plan sizing at M0 exit); quarterly limit re-check · **Source:** CON-014; CON-035; CON-037; confirmed Q3; architecture §9

- **Description:** Supabase-managed Postgres and Auth are single points of failure for identity and all service data. Plan limits are easy to exceed at launch, and an outage blocks sign-in and every service at once. The specific couplings that matter are Supavisor pool size versus the per-service connection budget, `pg_net` request timeout and queue behaviour (the whole scheduler ticks through it every minute), pg_cron concurrency and its inability to detect its own database being down, Vault operation limits under per-request decryption, Storage egress for exports and PDFs, and the Auth MAU pricing cliff. Data residency is a Supabase region decision that cannot be changed in place.
- **Likelihood / Impact:** Medium / High
- **Owner:** Principal Architect (accountable)
- **Treatment:** mitigate (quota and dependency controls); transfer (Supabase-SLA-covered portion); accept (residual single-provider risk)
- **Launch blocker:** Yes for launch readiness.
- **Response:** Size the plan against the CON-035 capacity baseline for DB size, peak connections, Auth MAU and Vault use; use connection pooling and per-service credentials. Monitor each quota with an alert threshold before it is a limit. Verify backup/PITR actually restores within RPO ≤24h / RTO ≤4h. Design a degraded read-only mode and a status-monitoring alert. Record that a serial Supabase Auth dependency caps product availability at or below Supabase's SLA (CON-014). Document the manual failover decision and the customer communication path. Re-check limits at each service go-live.

### R-009 — Silent job failure

- **Due / cadence:** 2027-01-15 (before the first scheduled job); monthly job-health review · **Source:** BRD-012 NFR-AVL-02, NFR-JOB-04; AC-JOB-13; CON-008; CON-030

- **Description:** Price refresh, FX refresh, valuation recompute and reconciliation run as scheduled jobs. With no message queues in the fixed stack, a job that stops running has no dead-letter queue to surface it; the product simply serves stale numbers while appearing healthy. **The earlier "alert at 2× the schedule interval" rule violates the NFRs for any interval above 2.5 minutes:** the 15-minute tail price job would alert at 30 minutes, hourly FX/reconciliation at 2 hours and daily retention at 48 hours, against NFR-AVL-02 (alert ≤5 min), NFR-JOB-04 (<5 min) and AC-JOB-13 (≤15 min).
- **Likelihood / Impact:** High / High
- **Owner:** Principal Architect (accountable), with QA Lead
- **Treatment:** mitigate
- **Launch blocker:** Yes.
- **Response:** The Job Orchestration service writes a heartbeat and `last_success_at` per job on every run. **A dead-man evaluator outside the database and the cluster** (CON-030) polls every minute and alerts within **5 minutes** of a missed run, independent of the Supabase/pg_cron path it is monitoring; pg_cron runs an internal check every minute as a second line. Write a Postgres dead-letter row for failed runs (CON-008), with idempotency keys plus bounded retries for notifications and recompute, and per-venue circuit breakers. Structured logs and metrics per job; an operator runbook for each. Test by killing a scheduled job and confirming the alert fires within 5 minutes and the staleness badge appears.

### R-010 — Wrong decimals and precision

- **Due / cadence:** 2027-02-15 (before Valuation service); property tests at every merge · **Source:** BRD-012 accuracy NFRs; CON-006; architecture §4

- **Description:** Balances arrive in atomic units with per-chain decimals; display, valuation and P&L use decimal money. Mixing atomic and whole units, or using binary floats, produces wrong valuations that are plausible-looking — the exact failure mode the >99% accuracy metric exists to prevent. **`rust_decimal`'s 96-bit mantissa caps at about 7.9×10^28**, so a raw uint256 atomic balance above roughly 79 billion 18-decimal tokens overflows before normalisation.
- **Likelihood / Impact:** High / Critical
- **Owner:** Principal Architect (accountable), with QA Lead
- **Treatment:** mitigate
- **Launch blocker:** Yes.
- **Response:** `rust_decimal` end-to-end; no `f64` in domain or money paths. **Parse atomic units as `u128`/bigint at the adapter boundary and convert to `rust_decimal` only after scaling**; guard overflow. The canonical asset registry stores chain decimals, display decimals and the conversion rule. Normalise at the adapter boundary and never mix scales inward. Property-based and unit tests with known vectors per chain family and per venue, plus a golden-file reconciliation test. Impossible balances raise a reconciliation state rather than rendering. The >99% valuation-accuracy acceptance test gates launch.

### R-011 — 9-month timeline versus committed scope

- **Due / cadence:** 2026-11-20 (scope freeze and cut list); monthly scope/burn checkpoint · **Source:** confirmed Q1 and Q6; `build-vs-buy.md` Build Cost and TCO; BRD-001 success metrics

- **Description:** Build-vs-buy estimates 64–114 engineer-weeks (14.8–26.3 engineer-months). The plan adds **36–56 engineer-weeks of platform overhead** (ten workspaces, gateway, IaC/staging, Swarm/observability, job orchestration, contract tests, image supply chain, external monitoring) and **8–16 engineer-weeks of committed Must scope** (WalletConnect, CSV import, ten-chain coverage; xpub/watch-only is a Should fast-follow) that the base does not visibly price. **Honest demand is 108–186 engineer-weeks, or 25–43 engineer-months** — a direct sum, not a further 25% loading. At 80% effective capacity, 3 × 9 = 21.6 and 4 × 9 = 28.8 do not cover the range; 5 × 9 = 36 covers the low-to-upper-middle; the full range needs 5 × 11 months; 6 × 9 reaches 43.2 effective, just below the 44 upper bound. The earlier "fits only at the low end of scope and the high end of staffing" conclusion was computed on the smaller, under-scoped base.
- **Likelihood / Impact:** High / High
- **Owner:** Delivery Lead (accountable), with Principal Architect
- **Treatment:** mitigate
- **Launch blocker:** Yes if scope is not frozen.
- **Response:** Lock scope at Option B plus the committed Must account-source/chain coverage, with a **written cut list against the top ~8 engineer-months**. Plan **9 months with 5 engineers and 36 effective engineer-months**, with **10–11 months as the realistic date for the full committed scope**; a 4-engineer plan is Option A/thin scope only and must be recorded as a scope decision. Build the thinnest end-to-end slice first (synthetic M0: auth → one venue → asset registry → valuation → dashboard), one service at a time behind the gateway. Freeze launch scope at five named venues, ten chains and two committed import methods plus xpub as a Should fast-follow; keep DeFi/staking/LP, tax filing, mobile, advisor portals and multi-user post-MVP. Monthly scope/burn checkpoint and re-baseline at each checkpoint rather than at month nine.

### R-012 — USD 150,000 non-salary budget

- **Due / cadence:** 2026-11-30 (line-item cost plan); monthly burn and contingency review · **Source:** confirmed Q2, Q5 and Q7; `build-vs-buy.md` Cost and Operational Risk

- **Description:** The envelope covers vendors, infrastructure, legal and contingency; salaries are separately funded. The recurring vendor line is ~USD 357–557/month plus ~USD 50–150/month Esplora, which is small. **The budget does not close at the high end when tested:** decomposed 9-month non-salary spend is **USD 71,100–167,500 before contingency and USD 85,320–209,375 with 20–25% contingency**, against a USD 150,000 ceiling. A lower-cost EU-only comparison case (~USD 85k–110k) would fit but is **not** the confirmed plan (Q4 keeps EU+UK first); the recommended EU+UK case (~USD 130k–165k) is at or above the ceiling; the high case (~USD 200k–210k) is ~1.3–1.4× the envelope. The earlier 20–25% reserve (USD 30,000–37,500) was below the low-end legal line plus contingency and omitted the pen test and accessibility audit entirely. **Q7 resolves the gap by capping legal at ~USD 50,000 fixed fee so total non-salary spend stays ≤ USD 150,000**, with the UK still gated on counsel sign-off; the plan must be re-cut to that cap rather than the earlier USD 40,000–90,000 range. The market-data/benchmark/exchange licence fee is unpriced.
- **Likelihood / Impact:** Medium / **Critical** (launch-affecting, not "No")
- **Owner:** Delivery Lead (accountable)
- **Treatment:** mitigate
- **Launch blocker:** Yes if the chosen scope cannot be funded; the assessment's conditional GO depends on C7.
- **Response:** Produce a line-item non-salary plan with **legal at a ~USD 50,000 fixed fee under the Q7 cap**, pen test USD 12,000–30,000, accessibility audit USD 8,000–20,000, hosting/staging USD 5,500–11,000 and vendor lines at list price, so the total stays ≤ USD 150,000. Obtain fixed-fee counsel quotes. Hold ≥20% contingency as untouchable. Price the market-data/benchmark/exchange licence before commit, or record it as an explicit unbudgeted liability requiring approval. If the capped EU+UK case still cannot close, **do not switch the UK off or silently phase it**: escalate a scope or envelope decision at the approval gate, and keep the UK as a gated launch market (R-003, A-031) until that decision is recorded. Track burn monthly; any contract above the remaining contingency requires explicit approval. Keep merchant-of-record fees, user acquisition, entity formation, insurance and the US phase outside the envelope and state that explicitly.

### R-013 — Exchange API churn, rate limits and bans

- **Due / cadence:** 2027-01-31 (cluster-wide limiter before first venue adapter); per-venue monthly review · **Source:** BRD-012 NFR-LEG-05; `build-vs-buy.md` Exchange Integration Layer; architecture §4

- **Description:** Exchange APIs change without notice (Binance has retired streams and schemas; Coinbase's Pro-to-Advanced-Trade migration invalidated keys). Binance meters **per IP** and escalates 429 to an HTTP 418 ban of 2 minutes to 3 days; Kraken meters per key with a decaying counter. Account Service and Market Data Service call exchange APIs from the same Swarm egress IP, so per-service limiters can each stay within budget while the shared IP budget is exhausted — a cluster-wide ban. A ban or a silent broken connection removes a venue from the user's total and corrupts the reconciliation claim.
- **Likelihood / Impact:** High / High
- **Owner:** Principal Architect (accountable)
- **Treatment:** mitigate
- **Launch blocker:** Yes for the launch coverage and accuracy claim.
- **Response:** One least-privilege adapter per venue behind the `ExchangeReader` port, with contract tests and a per-venue circuit breaker. **One cluster-wide per-venue and per-IP token bucket** shared by every caller (dedicated egress proxy or one owning service), with permission probes charged to the same budget. Honour 429/418 backoff and never retry into a ban; cache reads. A failed connection becomes a visible reconciliation state, never a silent zero. Per-venue key-rotation and deprecation runbook; adapter changes ship behind the port without domain edits. Monitor per-venue error and latency.

### R-014 — Consumer-subscription law is not covered

- **Due / cadence:** 2027-06-30 (before paid plans, gate C9); annual terms review · **Source:** BRD-012 NFR-LEG-08/09; BRD-016 FR-BIL-09/12 and BR-BIL-03/10; D-018

- **Description:** The only revenue path is a paid EU/UK retail subscription. No risk, issue, dependency or budget line covered the EU Consumer Rights Directive 2011/83/EU (pre-contract information; 14-day withdrawal and the digital-content consent/waiver mechanics), the UK Consumer Contracts Regulations 2013 regs 13–18, the DMCCA 2024 subscription/auto-renewal duties, UCPD price transparency, VAT/OSS, unfair-terms review (93/13/EEC; CRA 2015 Part 2), ADR/complaint information, or member-state rules such as Germany §312k BGB and France L215-1. The merchant of record handles tax remittance, not necessarily the trader's disclosure, cancellation and withdrawal duties.
- **Likelihood / Impact:** High / Critical
- **Owner:** Head of Compliance (accountable), with Billing Lead
- **Treatment:** mitigate
- **Launch blocker:** Yes before paid plans open.
- **Response:** Add CON-038 and C9: counsel-reviewed auto-renewal disclosure and receipt terms; cancellation as easy as sign-up; "order with obligation to pay" button; withdrawal/refund path with the digital-content exception documented; no pre-checked consent or dark patterns; ADR details; translated terms where required; a written allocation of the trader role in the merchant-of-record contract. Add the workstream to the legal budget line and to M4; do not rely on A-014's "auto-renewal compliance" assumption to transfer the duty.

### R-015 — Data-protection transfers and DPIA sequencing

- **Due / cadence:** 2027-01-15 (M1 gate before first real user); annual DPIA and transfer review · **Source:** BRD-012 NFR-PRV-09–14; BRD-017 FR-PRV-03/06/07/13/15/18/20; CON-037; GDPR Art 35

- **Description:** The earlier DPIA covered only wallet addresses and keyed hashes and was sequenced after the first real-user path. GDPR Art 35 and EDPB Guidelines 02/2025 require it before processing, and it must also cover exchange credentials, support access, analytics/consent, audit logs, age gating, retention and cross-border transfers. Article 27 representatives, Article 30 records, processor DPAs and the transfer mechanism (SCCs + TIA, IDTA/Addendum or DPF) are Must requirements; the Supabase region cannot be changed in place. The M0 exit test as originally written had a real user connecting an exchange key at week 6, weeks before the DPIA and jurisdiction gate.
- **Likelihood / Impact:** High / Critical
- **Owner:** Head of Compliance (accountable), with Principal Architect
- **Treatment:** mitigate
- **Launch blocker:** Yes before the first real user.
- **Response:** Make M0 **synthetic-only**; move the DPIA, jurisdiction/age gate, DPAs, representatives and transfer mechanism to an M1 gate before the first real user (C8). Fix the Supabase/VPS/backup/log/email/analytics region pair at M0 (CON-037). Publish a data map with processor location and transfer mechanism. Link erasure/crypto-shredding and retention into the DPIA; reconcile the 5-year financial and 12-month audit retention with Article 17(3). Add the breach-notification path and decision-maker.

### R-016 — European Accessibility Act obligation and audit

- **Due / cadence:** 2027-04-30 (external audit before EU launch); annual re-audit · **Source:** BRD-012 NFR-A11Y-01–05 and NFR-UX-01–03; BRD-018; Directive (EU) 2019/882

- **Description:** The EAA (Directive (EU) 2019/882) has applied to EU consumer services since 28 June 2025. The earlier RAID log had no EAA risk, issue or launch-blocking dependency, and treated "WCAG 2.2 AA patterns" as planned/parallel rather than a gate; the audit cost disappeared from the budget risk. The EAA presumption of conformity is **EN 301 549**, and the Act also requires an accessibility statement and feedback mechanism. Art 4(5) may exempt microenterprises providing services (confirm <10 staff and ≤€2m), and member-state implementation varies.
- **Likelihood / Impact:** Medium / High
- **Owner:** Frontend Lead (accountable), with Head of Compliance
- **Treatment:** mitigate
- **Launch blocker:** Yes for EU launch (or a documented exemption decision).
- **Response:** Treat accessibility as a release gate (C13): WCAG 2.2 AA on core flows, EN 301 549 mapping, accessible charts, **PDF/UA-tagged** briefs, accessibility statement and feedback channel. Budget the external audit at USD 8,000–20,000. Have counsel confirm the member-state implementation and the microenterprise exemption; if the exemption applies, keep WCAG 2.2 AA as a product commitment (D-016) but correct the "legally required" wording and gate.

### R-017 — Platform operations, environment parity and supply chain

- **Due / cadence:** 2026-12-19 (M0 exit); quarterly drill and configuration review · **Source:** CON-028–CON-034, CON-045–CON-048; architecture §5/§7/§8/§10

- **Description:** The architecture models one environment (Production) and the earlier artifacts introduced no IaC, staging, image supply chain or host least privilege. Without them: the Vault rotation, restore, node-kill and migration-rollback drills are first-run against the cluster holding users' exchange credentials; a mutable `latest` tag is an unreviewed production deploy; a tampered registry image is undetectable; and a stolen SSH key or provider account reaches node memory and disk. Docker Swarm also has no drift detection, no disruption budget and no autoscaling.
- **Likelihood / Impact:** High / High
- **Owner:** DevSecOps Lead (accountable), with Principal Architect
- **Treatment:** mitigate
- **Launch blocker:** Yes before public beta.
- **Response:** Add CON-028 to CON-034, CON-045 to CON-048: IaC of record (Terraform/OpenTofu + cloud-init/Ansible) with stack files in Git as the only deploy path; a staging environment; a reconcile job and no hand-run changes; private registry with digest pinning, CVE scan gate and SBOM; host/provider least privilege with hardware-key MFA, SSO, bastion-only key SSH and an audited DB path; rollback and expand/contract migration rules; bounded log rotation, disk alerts, resource limits and node labels; a recorded Swarm maintenance-mode decision with a revisit trigger. Make "staging deploy, rollback and scan gate" part of C12.

### R-018 — Unpriced market-data/benchmark/exchange licence fee

- **Due / cadence:** 2026-12-15 (written quotes before envelope commit); annual renewal review · **Source:** confirmed Q2 and Q7; BRD-020 OQ-002; `build-vs-buy.md` Cost and Operational Risk

- **Description:** The feasibility assessment names an unbudgeted five-figure annual licence cost for derived valuations and cross-source medians, but no RAID entry or budget line carried it. Benchmark and exchange commercial-use licences are also unpriced. The envelope has no headroom for it at the top of the range.
- **Likelihood / Impact:** Medium / High
- **Owner:** Delivery Lead (accountable), with External Counsel
- **Treatment:** mitigate
- **Launch blocker:** No, unless the fee breaches the contingency (then yes).
- **Response:** Add the fee as an explicit line in the cost plan (C7) and to the licence register. Obtain written quotes before committing the envelope. If the fee is material, re-cut scope or re-approve the envelope; do not absorb it silently into contingency. Track renewal as a recurring line.

### R-019 — Launch capacity is undefined

- **Due / cadence:** 2027-01-31 (baseline before load test); quarterly re-baseline · **Source:** BRD-012 NFR-SCL-01; CON-035

- **Description:** NFR-SCL-01 requires launch capacity to be defined, but no artifact gave concurrent users, requests per second, per-service peak connections or database size. C10 ("load test at launch concurrency") therefore has no number, and the Supabase pooler/plan sizing cannot be verified.
- **Likelihood / Impact:** Medium / High
- **Owner:** Principal Architect (accountable)
- **Treatment:** mitigate
- **Launch blocker:** Yes before paid plans.
- **Response:** Publish CON-035 with a derived launch-capacity model; feed it into the load test (C10), Supabase plan and Supavisor sizing, and the node placement/headroom model (CON-048). Re-baseline quarterly.

### R-020 — US state money transmission at the US phase

- **Due / cadence:** 2028-01-31 (before any US marketing or onboarding); annual review · **Source:** confirmed Q4; BRD-002 AC-USR-11; `market-trends.md`

- **Description:** Read-only tracking is generally outside MSB scope, but an order intent, deep link or future routing partner could be construed as money transmission. CLARITY has no stable federal frame after failing Senate cloture in September 2026, so state law governs.
- **Likelihood / Impact:** Medium / High
- **Owner:** Head of Compliance (accountable)
- **Treatment:** avoid (US at launch); mitigate (before US phase)
- **Launch blocker:** No for EU/UK; yes before US.
- **Response:** Keep the US out of the launch gate. Obtain a state money-transmission/MSB opinion before any US marketing or onboarding; keep order intents non-transmitted; record the trigger that would change the analysis (a routing partner or execution feature).

### R-081 — Data residency and the transfer mechanism are unresolved across processors

- **Description:** The launch accepts one Supabase region plus commodity VPS instances, but the full processor set — VPS compute, backups, logs/metrics, transactional email, analytics, support tooling and any CDN — has not been mapped to a country, and no transfer mechanism has been selected per processor. A Supabase region cannot be changed in place, so a late discovery forces a migration. UK transfers need a separate mechanism from EU transfers after Brexit (IDTA/Addendum or the UK extension to the DPF), and an SCC-only answer without a transfer impact assessment is not defensible. The EU/UK-first decision (Q4) makes this a launch gate, not a US-phase concern.
- **Likelihood / Impact:** High / Critical
- **Owner:** Head of Compliance (accountable), with Principal Architect
- **Treatment:** mitigate
- **Due / cadence:** 2027-01-15 (M1 gate); annual transfer review
- **Source:** confirmed Q3 and Q4; BRD-012 NFR-PRV-09–14; BRD-017 FR-PRV-03/06/07/13/15/18/20; CON-037; GDPR Ch V; A-013
- **Launch blocker:** Yes — before any real user or EU/UK data collection.
- **Response:** Publish a data map naming each processor, its storage location and its transfer mechanism before M0 exit. Fix the Supabase/VPS/backup/log/email/analytics region pair at M0 and record it as a decision. Execute DPAs with SCCs plus a written TIA, or the IDTA/Addendum where the UK is the exporter, or rely on DPF certification where valid; do not rely on consent or a browser-side check. Confirm each mechanism with counsel as part of D-013/D-017. If a processor cannot be moved or covered, replace it before launch rather than accept an unlawful transfer. Re-open the mapping for the US phase.

### R-082 — Localisation scope is undefined and can inflate cost or breach consumer-information duties

- **Description:** "Localisation" is used without a scope decision: interface languages, legal terms and pre-contract information, display currency, number/date formats, local payment methods, VAT/OSS treatment, customer support language and accessibility statements. The consumer-law workstream requires key pre-contract information and cancellation terms in a clear and intelligible language, and the EAA requires an accessible statement; machine-translated legal copy does not meet either. An unbounded localisation claim also consumes effort that the Option B estimate does not carry.
- **Likelihood / Impact:** Medium / High
- **Owner:** Product Lead (accountable), with Head of Compliance
- **Treatment:** mitigate
- **Due / cadence:** 2026-12-05 (scope decision); reviewed at each market gate
- **Source:** BRD-001 scope; confirmed Q4 and Q7; BRD-016 FR-BIL-09/12; BRD-012 NFR-LEG-08/09 and NFR-A11Y-01–05; R-014; R-016
- **Launch blocker:** Yes if a launch market is served in a non-English language or requires translated terms.
- **Response:** Record the minimum launch localisation in the scope decision: English for the UK; the named EU launch languages; display currency per user; localised number/date formats; translated pre-contract information and cancellation terms reviewed by counsel; local payment method only if the merchant of record supports it; accessibility statement per language. Do not machine-translate legal or cancellation copy. Put the effort in the C9/D-018 estimate and defer additional languages post-MVP with a documented gap, rather than promising full localisation.

### R-083 — Market-data classes beyond display rights are uncleared

- **Description:** R-001 covers vendor display, storage, history and derived-valuation rights, but at least three further classes carry different terms and can each block a launch surface: (a) exchange API terms for retaining full trade history and deriving cost basis; (b) benchmark index terms for any index, median or composite that could be treated as a benchmark; (c) RPC/indexer provider terms for commercial use, caching and retention (Alchemy, Infura, Helius), with self-hosted Esplora only covering Bitcoin. A grant for one class does not imply the others, and an adverse term in any class removes a launch claim.
- **Likelihood / Impact:** Medium / Critical
- **Owner:** Principal Architect (accountable), with External Counsel
- **Treatment:** mitigate
- **Due / cadence:** 2026-12-15 (written terms review); before any adapter activation
- **Source:** BRD-012 NFR-LEG-05/06; BRD-020 OQ-002; CON-039/040; R-001; R-013; A-004
- **Launch blocker:** Yes for the cost-basis/P&L and on-chain surfaces respectively.
- **Response:** Add a class column to the licence register — display, storage, history, derived valuation, retention period, commercial use — and record the written evidence per source. Obtain exchange written confirmation for trade-history retention, benchmark-provider confirmation that the composite is licensed or not a benchmark, and RPC commercial-use/caching confirmation. Keep self-hosted Esplora for Bitcoin and document cache TTLs. Sources without a reference remain disabled; escalate an adverse class to the approval gate rather than silently narrowing the product.

### R-084 — CDN/WAF, bot control and rate limiting are absent

- **Description:** D-016 names edge/CDN/WAF, but no risk or control owns it. Without a CDN/WAF and application rate limits, the SPA, the gateway and Supabase Auth are directly exposed to credential stuffing, scraping, enumeration and volumetric attack; the single Swarm ingress is a denial-of-service single point; and abusive clients can exhaust the shared exchange/RPC quotas that R-013 caps. Per-IP limits alone are insufficient when users share NAT and the cluster shares one egress IP.
- **Likelihood / Impact:** High / High
- **Owner:** DevSecOps Lead (accountable), with Principal Architect
- **Treatment:** mitigate; transfer (volumetric DDoS absorption under the CDN/WAF provider SLA)
- **Due / cadence:** 2026-12-19 (M0); quarterly rule and threshold review
- **Source:** BRD-012 NFR-SEC and NFR-AVL-01–06; CON-028–CON-034; D-016; R-013; architecture §8
- **Launch blocker:** Yes before public beta.
- **Response:** Put the SPA and gateway behind a CDN/WAF with TLS, managed OWASP rules, bot management and provider DDoS protection. Add edge limits plus application limits per IP, per authenticated user, per API key and per endpoint; throttle sign-in, sign-up and exchange-credential endpoints separately; return 429 with `Retry-After`; alert on 4xx/5xx and WAF-block spikes. Load-test the limiter at the CON-035 baseline. Record the provider SLA and the residual exposure in the availability statement.

### R-085 — The Vault access exception has no security specification

- **Description:** R-005 covers credential compromise generally, but the proposed `SECURITY DEFINER` accessor is an explicit exception to the no-cross-schema and least-privilege rules and needs its own controls. Risks specific to the exception: an over-broad function lets any service role read every secret the owner can see; a mutable `search_path` enables object hijacking; a missing subject predicate turns a per-user read into a bulk read; decrypt calls are not audited; plaintext survives in memory, logs, traces or error payloads; key backup versus crypto-shredding is undecided; and behaviour is undefined when Vault is unreachable. Without these, the exception becomes the easiest path to a cross-tenant breach.
- **Likelihood / Impact:** Medium / Critical
- **Owner:** Principal Architect (accountable), with DevSecOps Lead
- **Treatment:** mitigate
- **Due / cadence:** 2026-11-21 (week-two spike); quarterly access and audit review
- **Source:** CON-034; BRD-012 NFR-SEC-01/02/03/05; R-005; I-015; D-011; architecture §7
- **Launch blocker:** Yes before any exchange-credential feature.
- **Response:** Specify the function with a pinned `search_path`, an owner that is not a service role, `EXECUTE` granted only to named service roles, a mandatory server-derived subject argument and no bulk/`ALL` variant; enforce a statement timeout and a per-call audit row. Deny decrypt when the subject does not match the JWT-derived user. Fail closed on an unreachable Vault; never fall back to a cached or env secret. Define key backup versus crypto-shredding with counsel and privacy. Add a security review of the SQL plus a cross-user denial test to the merge gate, and log the exception and its compensating controls in the architecture record.

### R-086 — Penetration testing is unbudgeted in time and unscheduled

- **Description:** The budget risk names a USD 12,000–30,000 penetration test, but no dated test, scope or retest window exists. Testing after launch leaves the JWT/gateway path, cross-tenant authorization, Vault access and exchange-credential handling unproven in exactly the areas where a defect is a breach; a late critical finding has no schedule room before public beta, and a retest cannot be evidenced at the launch gate.
- **Likelihood / Impact:** Medium / High
- **Owner:** DevSecOps Lead (accountable), with QA Lead
- **Treatment:** mitigate; transfer (residual financial exposure via cyber-liability cover)
- **Due / cadence:** book 2027-04-15; test 2027-05-15 to 2027-05-31; retest before public beta 2027-06-30; annual thereafter
- **Source:** BRD-012 NFR-SEC; C11; R-012 budget line; R-005; R-084
- **Launch blocker:** Yes before public beta.
- **Response:** Book an external test with a fixed scope: authentication and JWT validation, gateway authorization, cross-tenant access on every endpoint, Vault accessor and key handling, exchange-credential storage, rate limits/WAF, and dependency/image scanning. Require critical and high findings fixed and retested before beta; track medium findings with owners and dates. Publish the retest letter as launch evidence. Budget the test in R-012/I-004 and keep a cyber-liability policy in place for the residual financial exposure, while stating that insurance does not mitigate the vulnerability itself.

---

## Assumptions

### A-001 — The budget excludes salaries, but its adequacy is not established

- **Due / cadence:** 2026-11-30 (cost plan); monthly burn review · **Source:** confirmed Q2 and Q5; `build-vs-buy.md` Build Cost and TCO

- **Description:** USD 150,000 covers non-salary spend (vendors, infrastructure, legal, contingency); the 3–5 engineer team is already funded. This resolves the apparent conflict between the envelope and the USD 150k–405k fully loaded build estimate. **It does not establish that USD 150,000 is adequate for the non-salary scope**: the pre-cap decomposed plan is USD 85k–209k, and the recommended EU+UK case is at or above the ceiling before the Q7 fixed-fee legal cap of ~USD 50,000 is applied.
- **Confidence / Impact if false:** Confirmed by Q2/Q5 / Critical — the whole delivery plan is unaffordable if salaries must fit, and the scope is unaffordable if the envelope is treated as sufficient without a cost plan.
- **Owner:** Delivery Lead (accountable)
- **Treatment:** accept (salary exclusion, confirmed); mitigate (cost-plan adequacy)
- **Response:** Record the salary exclusion in the cost plan and report non-salary burn separately. Do not report "fits" until C7 is evidenced; re-open if finance cannot confirm the team is funded or the licence fee is material.

### A-002 — Delivery shape is 5 engineers × 9 months on Option B

- **Due / cadence:** 2026-11-20 (staffing and unit DAG); monthly checkpoint · **Source:** confirmed Q1 and Q6

- **Description:** Greenfield, sequential service delivery behind the API Gateway. **5 engineers × 9 months = 36 effective engineer-months**, covering the low-to-upper-middle of the 25–43 engineer-month demand (25–44 if xpub is confirmed); the full committed scope needs 5 × 11 months; 6 × 9 reaches 43.2 effective, just below the 44 upper bound. A 4-engineer plan is Option A/thin scope only. 3 engineers × 9 months is not feasible.
- **Confidence / Impact if false:** Confirmed team range Q1, and the specific shape is the user-confirmed decision at Q6 (not an assessment recommendation) / Critical — parallel delivery or lower staffing changes the dependency DAG, the scope and the budget.
- **Owner:** Delivery Lead (accountable)
- **Treatment:** accept (planning basis); mitigate (delivery planning)
- **Response:** Convert to a unit DAG and a staffing plan; if staffing is 4, record the Option A scope decision and its architectural cost; if staffing falls below 4, re-cut scope at the first checkpoint. Do not treat 3 × 6 or 3 × 9 as viable.

### A-003 — Supabase-managed Postgres and Auth meet the NFRs

- **Due / cadence:** 2026-12-19 (SLA and restore evidence at M0); quarterly review · **Source:** BRD-012 NFR-AVL-01–06; CON-014; CON-035; confirmed Q3

- **Description:** Supabase provides availability, backup, connection and Auth capacity adequate for launch; per-service schema/role with no RLS is sufficient because services mediate all access. **The SLA and the Auth dependency are unverified.**
- **Confidence / Impact if false:** Medium confidence / Critical — identity and data availability depend on it, and a serial Auth dependency caps product availability at or below Supabase's SLA.
- **Owner:** Principal Architect (accountable)
- **Treatment:** mitigate (validate)
- **Response:** Load-test connection and query paths against CON-035, verify plan limits and PITR restore, obtain the SLA in writing, and review the per-service role model before the first service ships. If the SLA is below the target, apply the CON-014 upstream-exclusion scope or fund redundancy.

### A-004 — Named RPC and indexer providers cover the launch chains

- **Due / cadence:** 2026-12-15 (name every provider); monthly rate-limit review · **Source:** BRD-012 NFR-SCL-01; architecture §4; `build-vs-buy.md` Exchange Integration Layer

- **Description:** **EVM:** public/paid JSON-RPC with Alchemy or Infura failover. **Solana:** a named provider (Helius is budgeted; it must be named). **Bitcoin:** self-hosted Esplora. **Tron:** a named Tron provider — **neither Alchemy nor Infura serves Tron**, so the earlier claim that Alchemy/Infura failover covers all four families was false. Paid RPC is a reliability refinement, not a launch requirement, except where a chain has no public path.
- **Confidence / Impact if false:** Medium / High — rate limits or outages would break on-chain balances and valuation; a missing Tron provider blocks TRC-20/USDT coverage.
- **Owner:** Principal Architect (accountable)
- **Treatment:** mitigate (validate providers; failover)
- **Response:** Name the provider per chain before the adapter estimates are treated as launch coverage; measure rate limits and latency against launch forecasts; wire failover from day one; keep the paid RPC line in the contingency.

### A-005 — Retail market-data tiers meet the collection and freshness targets

- **Due / cadence:** 2026-12-15 (tier sampling before commitment); quarterly · **Source:** BRD-012 NFR-PERF-01/02 and NFR-LEG-05; BRD-020 OQ-002

- **Description:** The chosen tiers must support **1-minute top-asset collection** and **5-minute publication** for the top-100, 15-minute for the tail, plus a 60-second FX timestamp and licensed storage/history. CoinGecko Analyst, CoinMarketCap Startup and exchangerate.host Business are candidates, not confirmed.
- **Confidence / Impact if false:** Medium / High — the freshness and accuracy metrics fail, and a tier sized only to 5 minutes under-provisions collection.
- **Owner:** Principal Architect (accountable)
- **Treatment:** mitigate (validate)
- **Response:** Sample freshness, rate limits, storage rights and timestamp behaviour per tier before commitment; keep the dual-source and staleness-badge design regardless; re-size the plan to the 1-minute cadence.

### A-006 — Read-only exchange keys are the only viable access path

- **Due / cadence:** 2027-01-15 (onboarding design); annual legal review · **Source:** confirmed Q4; BRD-012 NFR-LEG-05/06

- **Description:** There is no PSD2-style access right to exchange accounts, so integration relies on user-created read-only API keys; exchange partner programmes are contractual and gated and are not available at launch scale.
- **Confidence / Impact if false:** High confidence / High — an access right would simplify onboarding but cannot be assumed.
- **Owner:** Principal Architect (accountable), with Head of Compliance
- **Treatment:** accept
- **Response:** Build the read-only key flow, least-privilege guidance and revocation handling; re-check the access-right position at each jurisdiction review.

### A-007 — Five venues, ten chains and two committed import methods (plus xpub fast-follow) are a credible launch claim

- **Due / cadence:** 2026-11-20 (scope freeze); quarterly roadmap review · **Source:** BRD-001 success metrics; BRD-004 FR-ACC-20/22; BRD-014 FR-ONB-07; D-005; D-015

- **Description:** Launch coverage is **OKX, Binance, Coinbase, Kraken and Bybit**, **ten chains** (Ethereum, Base, Arbitrum, Optimism, Polygon, BNB, Avalanche, Solana, Bitcoin, Tron) and **two committed import methods** (WalletConnect, CSV) plus xpub/watch-only as a Should fast-follow pending a product decision. This is below the 200–700-integration table stakes but matches D-005, D-015 and BRD-001's success metric.
- **Confidence / Impact if false:** Medium / High — breadth pressure would break the timeline; missing WalletConnect or CSV would breach committed Must requirements; xpub is a Should fast-follow.
- **Owner:** Product Lead (accountable), with Principal Architect
- **Treatment:** accept (scope and positioning)
- **Response:** Position on reconciliation quality, not breadth; hold the roadmap line for expansion and do not add a sixth venue pre-launch without cutting something else; include the two committed import paths (plus xpub if the product decision approves it) in the effort estimate and M1.

### A-008 — The product supplies none of the MiCA Art 3(1)(16) services

- **Due / cadence:** 2027-01-15 (written EU opinion); annual review · **Source:** BRD-002 AC-USR-11; MiCA Art 3(1)(16); confirmed Q4

- **Description:** CPB is a hosted, read-only aggregator, not non-custodial wallet software. The defensible basis is that it supplies none of the Art 3(1)(16) services (custody, exchange, execution, reception/transmission, advice, portfolio management, transfer), and generic decision support with a recorded order intent stays outside CASP authorisation provided no order is transmitted and no personalised advice is given. **The Recital 83 carve-out is not the basis.**
- **Confidence / Impact if false:** Medium / Critical — a licensing requirement would stop EU launch.
- **Owner:** Head of Compliance (accountable), with External Counsel
- **Treatment:** avoid (licensed activity); mitigate (validate counsel opinion)
- **Response:** Obtain counsel's written opinion analysing each service by name before EU launch; keep the feature boundary explicit in the design and copy; fail the jurisdiction gate if the opinion is adverse.

### A-009 — The jurisdiction and age gates can be enforced before any data collection

- **Due / cadence:** 2027-01-15 (M1 gate); quarterly bypass test · **Source:** BRD-002 AC-USR-11; BRD-017 AC-PRV-10; confirmed Q4; CON-037

- **Description:** The product can determine and record a user's jurisdiction and age and block onboarding and data collection in unsupported jurisdictions **before any personal data is stored**. **This requires a server-side enforcement point**: an edge/DNS-WAF check plus a Supabase "before user created" auth hook or signup proxy, because the SPA calls Supabase Auth directly and Supabase stores the email/IP before the gateway or Identity sees a request. A browser-side check plus self-declaration is bypassable.
- **Confidence / Impact if false:** Medium confidence / Critical — GDPR and MiCA exposure from day one; AC-USR-11 and AC-PRV-10 fail.
- **Owner:** Principal Architect (accountable), with Head of Compliance
- **Treatment:** mitigate (validate)
- **Response:** Implement the gate at the edge and in the Supabase pre-signup hook, fail-closed on unknown country, before request logging and non-essential cookies; test the blocked-country and under-age paths before any real user; audit every block; minimise and state the retention of the gate's own IP processing.

### A-010 — The fixed stack and Docker Swarm fit the team and the workload

- **Due / cadence:** 2026-11-20 (skills check at delivery planning); quarterly · **Source:** confirmed Q3; project rules (Rust/Axum/sqlx, React/Vite, Docker Swarm)

- **Description:** Rust/Axum/sqlx plus React/Vite and Docker Swarm are workable for the team; no AWS dependency is needed and the AWS platform lens is applied to Swarm/Supabase/VPS rather than AWS services. **The team's actual skill profile was never asked in the question set and is assumed.**
- **Confidence / Impact if false:** Medium confidence / Medium — stack changes would invalidate estimates; a skills gap would delay M0.
- **Owner:** Principal Architect (accountable)
- **Treatment:** accept (planning basis); mitigate (delivery planning)
- **Response:** Confirm team skills during delivery planning or budget training/hiring; keep the clean-architecture service template minimal so a new service is a day, not a week; add the skill assumption to the approval gate.

### A-011 — Vendor list pricing and monthly terms hold through MVP

- **Due / cadence:** 2026-12-15 (contract terms in writing); quarterly price review · **Source:** confirmed Q2; `build-vs-buy.md` Cost and Operational Risk

- **Description:** Vendor costs remain at published list (CoinGecko USD 129/mo, CoinMarketCap USD 79/mo, FX USD 99.99/mo, Helius from USD 49/mo, RPC overage ~USD 200/mo) and contracts stay monthly-cancellable.
- **Confidence / Impact if false:** Medium / Medium — a price change or annual commitment eats the contingency.
- **Owner:** Delivery Lead (accountable)
- **Treatment:** mitigate (validate before signing)
- **Response:** Confirm pricing and term in writing at contract time; no annual enterprise commit pre-MVP; budget a 10–20% vendor price buffer.

### A-012 — The controller is established in the EU or UK, or representatives are appointed

- **Due / cadence:** 2026-12-05 (entity and representative decision); annual review · **Source:** confirmed Q4; BRD-012 NFR-PRV-09–14; GDPR Art 27

- **Description:** The question set never asked where the controller is established, which entity contracts with users and the merchant of record, or whether EU/UK representatives are appointed. The plan assumes a representative path is available and budgeted.
- **Confidence / Impact if false:** Low confidence / Critical — without establishment or representation, EU/UK launch is unlawful regardless of product design.
- **Owner:** Head of Compliance (accountable)
- **Treatment:** mitigate (approval-gate validation)
- **Response:** Decide the contracting entity and establishment; appoint Article 27 EU and UK representatives if needed; record Article 30 records; add the cost and lead time to the legal line.

### A-013 — A Supabase region pair is acceptable and can be fixed at M0

- **Due / cadence:** 2026-12-19 (region pair fixed at M0); annual transfer review · **Source:** confirmed Q3 and Q4; CON-037; GDPR Ch V

- **Description:** The launch accepts a Supabase region pair (primary plus backup region), with the VPS/backups/logs/analytics/email processors mapped, and the transfer mechanism is lawful for every processor.
- **Confidence / Impact if false:** Medium / Critical — the region cannot be changed in place; a late change is a migration.
- **Owner:** Principal Architect (accountable), with Head of Compliance
- **Treatment:** mitigate (validate before M0 exit)
- **Response:** Fix the region pair at M0 (CON-037); document the transfer mechanism per processor; revisit for the US phase, which may change residency and topology.

### A-014 — The merchant of record handles tax, not the trader role

- **Due / cadence:** 2026-12-15 (MoR contract allocation); annual review · **Source:** BRD-016 OQ-002 and FR-BIL-09/12; BRD-012 NFR-LEG-08/09

- **Description:** The MoR absorbs subscription tax and remittance; it does **not** automatically absorb the trader's consumer-law disclosure, cancellation and withdrawal duties. The MoR choice is still open (BRD-016 OQ-002).
- **Confidence / Impact if false:** Medium / Critical — if the MoR is not the trader, the product owes the duties; if it is, the contract must say so.
- **Owner:** Billing Lead (accountable), with Head of Compliance
- **Treatment:** mitigate (validate before paid plans)
- **Response:** Allocate the trader role explicitly in the MoR contract; confirm the consumer-law position in each launch market; add the flow and copy to C9.

### A-015 — Launch capacity can be defined and met

- **Due / cadence:** 2027-01-31 (CON-035 published); quarterly re-baseline · **Source:** BRD-012 NFR-SCL-01; CON-035

- **Description:** A launch forecast exists or can be derived; the load test can be run at that concurrency before paid plans.
- **Confidence / Impact if false:** Medium / High — C10 is untestable without a number, and the p95/availability claims stay directional.
- **Owner:** Principal Architect (accountable)
- **Treatment:** mitigate (validate)
- **Response:** Publish CON-035; derive the number from onboarding and marketing forecasts; re-baseline after launch.

### A-016 — The market-data/benchmark/exchange licence fee is zero or absorbed

- **Due / cadence:** 2026-12-15 (written quotes); annual renewal review · **Source:** confirmed Q2 and Q7; BRD-020 OQ-002

- **Description:** Either no paid licence is required for display, storage, history and derived valuations, or the fee fits inside the contingency. This is **not established**; the assessment calls it the largest financial unknown.
- **Confidence / Impact if false:** Low confidence / High — a five-figure annual fee breaches the high-end envelope.
- **Owner:** Delivery Lead (accountable), with External Counsel
- **Treatment:** mitigate (validate before envelope commit)
- **Response:** Obtain written quotes; add the fee to C7; re-cut or re-approve if material.

### A-017 — The EAA applies, or a microenterprise exemption is documented

- **Due / cadence:** 2026-12-05 (entity-size determination); annual review · **Source:** Directive (EU) 2019/882; BRD-012 NFR-A11Y-01–05; BRD-018

- **Description:** The EAA's service obligations may exempt microenterprises providing services (<10 staff and ≤€2m, subject to member-state implementation). The entity-size determination is not recorded.
- **Confidence / Impact if false:** Medium / Medium — affects whether the audit is a statutory gate or a market-access commitment.
- **Owner:** Head of Compliance (accountable)
- **Treatment:** mitigate (validate)
- **Response:** Record the entity-size determination and counsel's view; keep WCAG 2.2 AA as a product commitment either way; correct the gate wording if exempt.

### A-018 — A historical FX source exists at transaction granularity

- **Due / cadence:** 2027-02-28 (before local-currency P&L); quarterly · **Source:** BRD-012 NFR-PERF-02; `build-vs-buy.md` Market Data and FX Layer

- **Description:** Local-currency cost basis and P&L at transaction timestamps need a historical FX source; exchangerate.host Business historical data is end-of-day only.
- **Confidence / Impact if false:** Medium / Medium — EOD FX introduces small P&L inaccuracies; an unavailable source blocks the local-currency claim.
- **Owner:** Principal Architect (accountable)
- **Treatment:** mitigate (validate)
- **Response:** Identify an intraday historical FX source or state the EOD-only limitation and its effect on P&L accuracy before the local-currency surface ships.

### A-031 — The UK remains a gated launch market, not a removed market

- **Description:** Confirmed answers Q4 and Q7 keep the UK in the initial EU+UK launch and cap legal spend so the envelope closes. The UK is therefore a **gated launch market**: it is not switched off, and it is not silently downgraded to pre-positioning. The condition for UK launch is a written current-regime opinion from counsel covering s.21 FSMA/financial-promotion restrictions, the financial-promotion Order and RAO 2001 arranging/advice, followed by sign-off of the UK marketing and onboarding plan. The dated go/no-go is 2027-06-30, with a re-run at the expected 2027-10-25 commencement of the future regime.
- **Confidence / Impact if false:** Confirmed by Q4/Q7; the go/no-go date is an assessment proposal / Critical — launching or marketing in the UK without the opinion is unauthorised promotion; removing the UK without approval contradicts the confirmed answer.
- **Owner:** Head of Compliance (accountable)
- **Treatment:** mitigate (dated gate and counsel sign-off); accept (schedule risk)
- **Due / cadence:** 2027-06-30 (go/no-go); re-run 2027-10-25
- **Source:** confirmed Q4 and Q7; R-003; I-017; FSMA 2000 s.21; RAO 2001; `market-trends.md`
- **Response:** Hold the UK gate as written. Any change to pre-positioning-only is an approval-gate decision with a recorded rationale, never a risk-owner action. If the opinion is adverse, gate UK marketing and onboarding while keeping the EU launch on schedule; do not collect UK user data or marketing consent before sign-off.

---

## Issues

### I-001 — No written redistribution/storage/derived-valuation licence from any vendor

- **Due / cadence:** 2026-11-28 (requests out); weekly until signed · **Source:** R-001; BRD-012 NFR-LEG-05/06; BRD-020 OQ-002

- **Severity:** Critical (open, launch-blocking)
- **Owner:** Principal Architect (accountable), with External Counsel
- **Treatment:** mitigate (resolve)
- **Response:** Vendor, exchange and benchmark legal requests out now (R-001); decision required before Market Data implementation and before any retention. Owner reports status weekly. No production ingestion or storage until signed.

### I-002 — No counsel opinion on MiCA order-intent treatment

- **Due / cadence:** 2027-01-15 (opinion before EU launch); quarterly · **Source:** R-002; BRD-002 AC-USR-11; confirmed Q4

- **Severity:** Critical (open, launch-blocking)
- **Owner:** Head of Compliance (accountable)
- **Treatment:** mitigate (resolve)
- **Response:** Engage EU counsel, scope the opinion to a recorded intent plus deep link and each Art 3(1)(16) service, and set a decision date before EU launch. Until then the feature stays design-only.

### I-003 — No DPIA covering the full processing set

- **Due / cadence:** 2027-01-15 (before first real user); annual · **Source:** R-015; GDPR Art 35; BRD-012 NFR-PRV-09–14

- **Severity:** Critical (open, blocks EU data collection)
- **Owner:** Head of Compliance (accountable)
- **Treatment:** mitigate (resolve)
- **Response:** Complete a DPIA covering wallet addresses, keyed hashes, exchange credentials, support access, analytics/consent, audit logs, age gating, retention and transfers before the first real user; record lawful basis and retention; link crypto-shredding and legal hold.

### I-004 — No cost plan mapping the USD 150k to vendor, legal and hosting lines

- **Due / cadence:** 2026-11-30 (cost plan); monthly · **Source:** R-012; confirmed Q2, Q5 and Q7

- **Severity:** High (open)
- **Owner:** Delivery Lead (accountable)
- **Treatment:** mitigate (resolve)
- **Response:** Produce a line-item non-salary plan with contingency and monthly burn; reconcile against the build-vs-buy run-rate, the widened legal line, pen test, accessibility audit, staging and the licence fee. State whether the recommended EU+UK scope closes or requires phasing.

### I-005 — 99.9% availability is untested and the earlier topology could not meet it

- **Due / cadence:** 2027-05-31 (drills and SLA evidence); monthly availability review · **Source:** R-004; BRD-012 NFR-AVL-01–06; CON-014

- **Severity:** High (open)
- **Owner:** Principal Architect (accountable)
- **Treatment:** mitigate (resolve)
- **Response:** Build the three-manager/three-failure-domain, three-plus-worker Swarm reference, run a node-kill/replica-kill drill, add the external synthetic monitor, obtain the Supabase SLA, and publish the upstream-exclusion scope. Do not restate the Must NFR as best-effort.

### I-006 — Greenfield with no walking skeleton

- **Due / cadence:** 2026-12-19 (synthetic M0 slice); weekly demo · **Source:** R-011; implementation plan M0; memory M2 (walking skeleton)

- **Severity:** High (open)
- **Owner:** Delivery Lead (accountable), with Principal Architect
- **Treatment:** mitigate (resolve)
- **Response:** Ship the synthetic thinnest end-to-end slice (auth → one venue → asset registry → valuation → dashboard) within the first 6–8 weeks; use it to validate the contract, the JWT path, the Vault path and the valuation pipeline. No real user until the M1 gates are in place.

### I-007 — Launch coverage sits below category table stakes

- **Due / cadence:** 2027-08-06 (launch positioning); post-launch review · **Source:** A-007; BRD-001 success metrics

- **Severity:** Medium (accepted)
- **Owner:** Product Lead (accountable)
- **Treatment:** accept
- **Response:** Make reconciliation quality the launch claim and publish the expansion roadmap; revisit after launch with retention data (R-011, A-007).

### I-008 — No enforceable pre-collection jurisdiction/age gate

- **Due / cadence:** 2027-01-15 (M1 gate); quarterly bypass test · **Source:** R-015; A-009; BRD-002 AC-USR-11; BRD-017 AC-PRV-10

- **Severity:** Critical (open, blocks real-user launch)
- **Owner:** Principal Architect (accountable), with Head of Compliance
- **Treatment:** mitigate (resolve)
- **Response:** Implement the edge/DNS-WAF check plus the Supabase "before user created" hook or signup proxy, fail-closed, with the age gate and audit; test a blocked country and an under-age sign-up before any real user. A client-side check or a post-auth gateway check is not acceptable.

### I-009 — No consumer-subscription compliance plan

- **Due / cadence:** 2027-06-30 (before paid plans, C9); annual · **Source:** R-014; BRD-012 NFR-LEG-08/09; BRD-016 FR-BIL-09/12

- **Severity:** Critical (open, blocks paid plans)
- **Owner:** Head of Compliance (accountable), with Billing Lead
- **Treatment:** mitigate (resolve)
- **Response:** Add the R-014 workstream to M4 and the legal budget; produce counsel-reviewed copy and flow, a cancellation-parity test, withdrawal/refund mechanics, ADR information and the MoR trader allocation. Gate on C9.

### I-010 — No IaC, staging environment or environment model

- **Due / cadence:** 2026-12-19 (M0 exit); quarterly drift review · **Source:** R-017; CON-028–CON-034

- **Severity:** High (open)
- **Owner:** DevSecOps Lead (accountable)
- **Treatment:** mitigate (resolve)
- **Response:** Build the IaC of record, staging stack and Git-only deploy path; add the reconcile job and ban hand-run changes; run the first drills against staging, not production (CON-028).

### I-011 — No launch capacity baseline

- **Due / cadence:** 2027-01-31 (CON-035); quarterly · **Source:** R-019; BRD-012 NFR-SCL-01

- **Severity:** High (open)
- **Owner:** Principal Architect (accountable)
- **Treatment:** mitigate (resolve)
- **Response:** Publish CON-035 with concurrent users, RPS, per-service connections and DB size; feed it into the load test, Supabase plan and placement model.

### I-012 — No exchange or benchmark licence review

- **Due / cadence:** 2026-12-15 (written terms review); quarterly · **Source:** R-001; R-083; CON-039/040

- **Severity:** Critical (open, launch-blocking for the P&L core)
- **Owner:** Principal Architect (accountable), with External Counsel
- **Treatment:** mitigate (resolve)
- **Response:** Open the exchange commercial-use/retention and benchmark-licence reviews now (CON-039/040); add them to the licence register; do not activate a source without a reference.

### I-013 — No incident-response, on-call or status-page readiness

- **Due / cadence:** 2027-06-30 (public beta); monthly game-day · **Source:** D-020; CON-044; C11

- **Severity:** High (open, blocks public beta)
- **Owner:** DevSecOps Lead (accountable), with Support Lead
- **Treatment:** mitigate (resolve)
- **Response:** Stand up the status page, paging path, primary/secondary rota and five runbooks; run one game day before launch (CON-044, C11).

### I-014 — No controller/representative/transfer decision

- **Due / cadence:** 2026-12-05 (entity decision); annual · **Source:** R-081; A-012; CON-037; GDPR Art 27

- **Severity:** Critical (open, blocks EU/UK launch)
- **Owner:** Head of Compliance (accountable)
- **Treatment:** mitigate (resolve)
- **Response:** Decide the contracting entity and establishment; appoint EU/UK representatives if needed; fix the region pair and transfer mechanism per processor (CON-037, A-012).

### I-015 — Vault access from Rust is unproven

- **Due / cadence:** 2026-11-21 (week-two spike); quarterly access review · **Source:** R-005; R-085; CON-034; BRD-012 NFR-SEC-01/02

- **Severity:** High (open, blocks credential features)
- **Owner:** Principal Architect (accountable)
- **Treatment:** mitigate (resolve)
- **Response:** Run the C3 spike in weeks 2–3 of M0: `SECURITY DEFINER` accessor, versioned KEK, per-subject wrapped keys, rotation without restart, unreachable-Vault behaviour, zero plaintext in logs/traces/errors. Resolve key backup versus crypto-shredding.

### I-016 — No image supply chain or host least privilege

- **Due / cadence:** 2026-12-19 (M0 exit); quarterly scan review · **Source:** R-017; CON-032/034

- **Severity:** High (open)
- **Owner:** DevSecOps Lead (accountable)
- **Treatment:** mitigate (resolve)
- **Response:** Add the private registry, digest pinning, scan gate and SBOM; apply provider hardware-key MFA, SSO, bastion-only key SSH, no network-exposed Docker socket, audited DB access and short-lived CI identity (CON-032/034).

### I-017 — No UK current-regime opinion

- **Due / cadence:** 2027-06-30 (dated UK go/no-go); re-run 2027-10-25 · **Source:** R-003; confirmed Q4 and Q7; FSMA 2000 s.21; RAO 2001

- **Severity:** Critical (open, blocks UK marketing/onboarding)
- **Owner:** Head of Compliance (accountable)
- **Treatment:** mitigate (resolve)
- **Response:** Obtain a UK opinion covering s.21 FSMA/financial promotion and RAO arranging/advice as well as the future regime; gate UK marketing and onboarding; escalate to the approval gate if the UK becomes a pre-positioning market.

### I-018 — The market-data/benchmark/exchange licence fee is unpriced

- **Due / cadence:** 2026-12-15 (quotes); annual · **Source:** R-018; confirmed Q2 and Q7; BRD-020 OQ-002

- **Severity:** High (open)
- **Owner:** Delivery Lead (accountable), with External Counsel
- **Treatment:** mitigate (resolve)
- **Response:** Obtain written quotes and add the fee to the cost plan and C7; re-cut or re-approve if material (R-018).

---

## Dependencies

### D-001 — Canonical Asset Registry (launch-blocking)

- **Due / cadence:** 2026-12-19 (before first consumer service); mapping change review at every release · **Source:** CON-006; R-010; BRD-004; architecture §4

- **Description:** Single source of canonical asset identity: symbol, chain, contract address, chain decimals, display decimals, vendor asset-ID mappings, price-source registry and quarantine. **Consumers must batch-fetch canonical IDs and display fields over the Asset Registry HTTP API; they may not join `registry` tables** (CON-006). A wrong decimal or mapping propagates into every number (amplifies R-010).
- **Criticality / Owner:** Launch-blocking / Principal Architect (accountable)
- **Treatment:** mitigate on the critical path
- **Response:** Build first, version the mapping, gate changes with a review plus a golden-vector test. Blocks D-002, D-004, D-005, D-008.

### D-002 — Market-data licence and provider adapters (launch-blocking)

- **Due / cadence:** 2026-11-28 (licence requests); weekly until granted · **Source:** R-001; I-001; BRD-012 NFR-LEG-05/06

- **Description:** Written display, **storage, history, redistribution and derived-valuation** rights plus a `MarketDataProvider` adapter (CoinMarketCap or another licensed source primary for valuation; CoinGecko display/secondary) behind a port.
- **Criticality / Owner:** Launch-blocking / Principal Architect (accountable), with External Counsel
- **Treatment:** mitigate; transfer (licensed alternative)
- **Response:** Licence first (R-001, I-001); adapter second. Blocks D-004 (valuation), any price display and any stored history.

### D-003 — FX rate provider (launch-blocking for local-currency P&L)

- **Due / cadence:** 2027-02-28 (before local-currency P&L); quarterly provider review · **Source:** R-007; A-018; BRD-012 NFR-PERF-02

- **Description:** `FxProvider` adapter against a 60-second FX tier with a persisted rate timestamp per valuation, plus a historical FX source or a stated EOD limitation.
- **Criticality / Owner:** Launch-blocking / Principal Architect (accountable)
- **Treatment:** mitigate
- **Response:** Contract and adapter before local-currency P&L ships; daily-tier data is insufficient for the freshness target (R-007, A-018).

### D-004 — Valuation service (launch-blocking)

- **Due / cadence:** 2027-03-15 (M2); daily freshness review · **Source:** BRD-012 NFR-PERF-01/02; architecture §6

- **Description:** Computes portfolio value from balances, quotes and FX at a single valuation timestamp, with staleness metadata, from a materialised snapshot recomputed by the scheduled job.
- **Criticality / Owner:** Launch-blocking / Principal Architect (accountable)
- **Treatment:** mitigate
- **Response:** Depends on D-001, D-002, D-003. Blocks D-006 and D-007.

### D-005 — Exchange read-only adapters (launch-blocking)

- **Due / cadence:** 2027-02-15 (before reconciliation); per-release contract tests · **Source:** R-013; A-006; BRD-012 NFR-LEG-05

- **Description:** One least-privilege `ExchangeReader` adapter per launch venue (**OKX, Binance, Coinbase, Kraken, Bybit**), each mapping venue assets onto canonical IDs and surfacing connection health.
- **Criticality / Owner:** Launch-blocking / Principal Architect (accountable)
- **Treatment:** mitigate
- **Response:** Depends on D-001. Feeds D-006 (reconciliation). Adapter contract tests are the merge gate (R-013).

### D-006 — Transfer matching and reconciliation queue (critical)

- **Due / cadence:** 2027-03-31 (M2); monthly accuracy review · **Source:** BRD-001 success metrics; architecture §6

- **Description:** Cross-venue matching, duplicate suppression, missing-cost-basis detection and a user-visible resolution queue — the differentiator and the evidence for the >99% accuracy claim.
- **Criticality / Owner:** Critical / Principal Architect (accountable)
- **Treatment:** mitigate
- **Response:** Depends on D-004 and D-005. Feeds D-007 (cost basis/P&L) and the SPA reconciliation screen.

### D-007 — Cost-basis lot engine and P&L (launch-blocking for the value proposition)

- **Due / cadence:** 2027-04-30 (M3); monthly accuracy review · **Source:** BRD-001 success metrics; BRD-012 accuracy NFRs

- **Description:** Domain-owned lot selection, fees, transfers and missing-basis detection; jurisdiction-agnostic, with display currency applied at the edge.
- **Criticality / Owner:** Launch-blocking / Principal Architect (accountable)
- **Treatment:** mitigate
- **Response:** Depends on D-004 and D-006. Gates the dashboard P&L and reporting surfaces.

### D-008 — Chain RPC and indexer adapters (critical)

- **Due / cadence:** 2027-02-15 (before on-chain balances); monthly provider review · **Source:** A-004; BRD-012 NFR-SCL-01; architecture §4

- **Description:** `ChainRpcProvider` adapters for EVM (with a named provider and failover), Solana (Helius or equivalent, named), Bitcoin (self-hosted Esplora) and Tron (a named Tron provider), with confirmation handling, token discovery and **ten-chain coverage**.
- **Criticality / Owner:** Critical / Principal Architect (accountable)
- **Treatment:** mitigate
- **Response:** Depends on D-001. Feeds on-chain balances into D-004. Name every provider before the estimate is treated as coverage (A-004).

### D-009 — Supabase Auth and API Gateway JWT path (launch-blocking)

- **Due / cadence:** 2026-12-19 (M0); quarterly security review · **Source:** CON-014; CON-037; BRD-012 NFR-SEC; architecture §7

- **Description:** SPA sign-in via Supabase Auth; the gateway forwards the Supabase JWT unchanged and every service validates it; user identity is derived server-side; the pre-signup hook hosts the jurisdiction/age gate.
- **Criticality / Owner:** Launch-blocking / Principal Architect (accountable), with DevSecOps Lead
- **Treatment:** mitigate
- **Response:** First cross-cutting dependency; blocks every authenticated service, the pre-collection gate (D-013) and the cross-tenant test suite (R-005).

### D-010 — Job Orchestration with heartbeat and idempotency (launch-blocking)

- **Due / cadence:** 2027-01-31 (before price/FX schedules); monthly job-health review · **Source:** R-009; CON-008; CON-030; BRD-012 NFR-JOB-04

- **Description:** Scheduler for price/FX refresh, valuation recompute and reconciliation, with heartbeat, `last_success_at`, bounded retries, idempotency keys, a Postgres dead-letter row and an external dead-man evaluator; no message queues.
- **Criticality / Owner:** Launch-blocking / Principal Architect (accountable)
- **Treatment:** mitigate
- **Response:** Depends on D-002/D-003/D-004. Enables the freshness SLA, the 5-minute missed-run alert and the staleness badge (R-009).

### D-011 — Audit service (critical)

- **Due / cadence:** 2027-01-15 (before support tooling or credentials); quarterly audit review · **Source:** R-005; R-006; BRD-012 NFR-SEC

- **Description:** Append-only audit of sensitive actions: credential add/use/delete, support access, jurisdiction decisions, order-intent records.
- **Criticality / Owner:** Critical / DevSecOps Lead (accountable), with Principal Architect
- **Treatment:** mitigate
- **Response:** Needed before support tooling or credential features ship (R-005, R-006).

### D-012 — Billing and entitlements (critical for the paid-plan SLA)

- **Due / cadence:** 2027-06-30 (paid-plan launch); monthly billing reconciliation · **Source:** BRD-016; D-018; R-014

- **Description:** Subscription plans, quotas and entitlements; the 99.9% availability commitment applies to paid plans, so entitlements must be reliable and auditable; consumer-subscription flows are part of this service's surface (D-018).
- **Criticality / Owner:** Critical / Principal Architect (accountable)
- **Treatment:** mitigate
- **Response:** Depends on D-009 and D-018. Can follow the free-tier MVP but must land before the paid-plan SLA is advertised.

### D-013 — External counsel, jurisdiction gate and data-protection pack (launch-blocking)

- **Due / cadence:** 2027-01-15 (EU opinion, DPIA and gate); quarterly regulatory review · **Source:** R-002; R-003; R-015; confirmed Q4

- **Description:** Written EU MiCA opinion, UK current-regime and future-regime opinion, US state MT opinion (before US), disclosures, DPIA, DPAs, representatives, transfer mechanism, retention and a **server-side jurisdiction and age gate that fails closed before any data collection**.
- **Criticality / Owner:** Launch-blocking / Head of Compliance (accountable), with External Counsel
- **Treatment:** mitigate
- **Response:** Engage counsel now; gate the EU (and UK, if launched) on the opinion, the DPIA, the transfer pack and the pre-signup hook (I-002, I-003, I-008, I-014, I-017). Blocks onboarding and any production data collection.

### D-014 — Parchment Ledger design system and SPA shell (planned, parallel)

- **Due / cadence:** 2027-03-31 (SPA shell); quarterly accessibility review · **Source:** BRD-012 NFR-A11Y-01–05; BRD-018; `design-system/`

- **Description:** Tokens, components, PWA shell and WCAG 2.2 AA/EN 301 549 patterns the SPA screens render against.
- **Criticality / Owner:** Planned / Frontend Lead (accountable), with Principal Architect
- **Treatment:** mitigate in parallel
- **Response:** Build alongside D-009; no business math in the SPA — it renders gateway responses only.

### D-015 — Account-source import paths (launch-blocking)

- **Due / cadence:** 2027-02-28 (M1); per-release contract tests · **Source:** BRD-004 FR-ACC-20/22; BRD-014 FR-ONB-07; A-007

- **Description:** WalletConnect read-only session and CSV import with template and mapping preview (BRD-004 FR-ACC-20/22, BRD-014 FR-ONB-07); xpub/watch-only is a Should fast-follow (FR-ACC-21).
- **Criticality / Owner:** Launch-blocking / Principal Architect (accountable)
- **Treatment:** mitigate
- **Response:** Depends on D-001 and D-009. Include in M1 and the effort estimate; a coverage claim without them breaches committed Must scope (A-007).

### D-016 — Platform foundation (launch-blocking for operations)

- **Due / cadence:** 2026-12-19 (M0); quarterly drill · **Source:** R-017; R-084; CON-028–CON-034

- **Description:** IaC of record, staging environment, private registry with digest pinning/scanning, Swarm three-domain topology, redundant ingress, edge/CDN/WAF, external synthetic monitor, observability stack and host least privilege.
- **Criticality / Owner:** Launch-blocking / DevSecOps Lead (accountable), with Principal Architect
- **Treatment:** mitigate
- **Response:** Build in M0; every drill (Vault, node-kill, restore, rollback) runs here first; C4/C12 evidence depends on it (CON-028 to CON-034, R-017).

### D-017 — Data-protection pack (launch-blocking)

- **Due / cadence:** 2027-01-15 (M1 gate); annual review · **Source:** R-015; R-081; CON-037; GDPR Ch V

- **Description:** Controller/processor map, DPAs and subprocessor list, EU/UK representatives, Article 6/30 records, transfer mechanism and TIA, retention/deletion automation and breach-notification runbook.
- **Criticality / Owner:** Launch-blocking / Head of Compliance (accountable)
- **Treatment:** mitigate
- **Response:** Depends on D-013; gates the first real user (C8, R-015).

### D-018 — Consumer-subscription compliance (launch-blocking for paid plans)

- **Due / cadence:** 2027-06-30 (C9); annual terms review · **Source:** R-014; I-009; BRD-016 FR-BIL-09/12

- **Description:** Counsel-reviewed auto-renewal disclosure, cancellation parity, refund/withdrawal mechanics, no dark patterns, ADR information and the MoR trader allocation.
- **Criticality / Owner:** Launch-blocking / Head of Compliance (accountable), with Billing Lead
- **Treatment:** mitigate
- **Response:** Depends on D-013; gates D-012's paid-plan launch (C9, R-014).

### D-019 — Capacity baseline and load test (critical)

- **Due / cadence:** 2027-01-31 (baseline); quarterly re-baseline · **Source:** R-019; A-015; CON-035

- **Description:** Defined concurrent users, RPS, per-service connections and DB size; the C10 load test at that capacity.
- **Criticality / Owner:** Critical / Principal Architect (accountable)
- **Treatment:** mitigate
- **Response:** Feeds the Supabase plan, pooler sizing, node placement and the p95 evidence; untestable without it (R-019, CON-035).

### D-020 — Operational readiness (launch-blocking for public beta)

- **Due / cadence:** 2027-06-30 (public beta); monthly game-day · **Source:** I-013; CON-044; C11

- **Description:** Status page, SEV-1 paging within 15 minutes, primary/secondary on-call rota, five runbooks and one incident drill.
- **Criticality / Owner:** Launch-blocking / DevSecOps Lead (accountable), with Support Lead
- **Treatment:** mitigate
- **Response:** Depends on D-016; gate public beta (C11, I-013). The first real outage must not be the first rehearsal.

---

## Traceability

- Budget semantics and delivery shape: `feasibility-questions.md` Q1, Q2, Q5, Q6, Q7, Q8; intent statement Q4; feasibility assessment §2–§3 (re-baselined arithmetic). Q6 fixes the Option B shape (10 launch services, 5 engineers, 9-month target, written cut list); Q7 fixes the ~USD 50k legal cap; Q8 records the Notification/Analytics/Support Console deferral as an explicit BRD deviation.
- Hosting: `feasibility-questions.md` Q3; project rules (Supabase, Docker Swarm, no AWS); architecture §9–§10.
- Jurisdictions and order intents: `feasibility-questions.md` Q4 and Q7; intent statement Q4; `competitive-analysis.md` Execution and Order-Intent Landscape; `market-trends.md` Regulatory and Compliance Shifts; BRD-002 AC-USR-11; BRD-017 AC-PRV-10. Q4 and Q7 keep the UK as a gated launch market with a dated go/no-go; no RAID response may switch it off.
- Market-data, exchange and benchmark rights: `build-vs-buy.md` Market Data and FX Layer, Exchange Integration Layer, Cost and Operational Risk; `market-trends.md`; BRD-012 NFR-LEG-05/06; D-002; R-083.
- Consumer subscription: BRD-012 NFR-LEG-08/09; BRD-016 FR-BIL-09/12, BR-BIL-03/10, AC-BIL-10; A-014; D-018.
- Privacy and transfers: BRD-012 NFR-PRV-09/10/11/12/13/14, NFR-SEC-25; BRD-017 FR-PRV-03/06/07/13/15/18/20.
- Availability, freshness, accuracy, audit, MFA, RPO/RTO: intent statement Success Metrics; BRD-012 NFR-AVL-01–06, NFR-SEC-01/02/03/05/06/07/11/12/14, NFR-PERF-01/02, NFR-SCL-01; architecture §6–§8.
- Platform operations: architecture §5/§7/§8/§10; stage platform lens (IaC, staging, observability, least privilege, image supply chain).
- Scope and coverage: `build-vs-buy.md` Recommended Composition, Build Cost and TCO; `market-trends.md` Implications for the MVP; intent statement Initial Scope Signal; BRD-001 success metrics; BRD-004 FR-ACC-20/22; BRD-014 FR-ONB-07; D-015; memory M1 (testable acceptance criteria) and M2 (walking skeleton).
- Venue/chain API fragility: `build-vs-buy.md` Exchange Integration Layer; architecture §4.
- Accessibility: BRD-001 FR-GEN-15; BRD-012 NFR-UX-01–03, NFR-A11Y-01–05; BRD-018; D-014; R-016; `market-trends.md`.

### Required coverage map at this revision

- Consumer-subscription law: R-014, I-009, D-018.
- UK current regime (s.21 FSMA / financial promotions) and UK marketing: R-003, A-031, I-017.
- Data residency and transfer mechanism: R-081, A-013, D-017.
- DPIA timing: R-015, I-003.
- Localisation scope: R-082.
- Market-data classes (exchange API retention, benchmarks, RPC): R-001, R-083, I-012.
- IaC/staging absence: R-017, I-010, D-016.
- Single control plane / failure domains: R-004.
- External monitoring: R-004, R-009.
- CDN/WAF and rate limiting: R-084.
- Image supply chain: R-017, I-016.
- Host least privilege: R-017, I-016.
- Capacity baseline: R-019, A-015, D-019.
- Vault exception security specifics: R-085, I-015.
- Accessibility audit timing: R-016, A-017.
- Penetration test timing: R-086.
