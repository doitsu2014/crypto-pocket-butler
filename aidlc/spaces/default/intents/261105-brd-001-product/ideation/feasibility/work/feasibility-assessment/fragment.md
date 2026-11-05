# Feasibility Assessment — Fragment

Producer: aidlc-architect-agent. This fragment covers the feasibility assessment only. Constraint register and RAID rows are produced separately and are referenced, not duplicated, here.

Scope of this assessment: a greenfield, read-only crypto portfolio aggregation and decision-support product, launched EU (MiCA) and UK first, then US, built by 3–5 engineers over 6–9 months, with a non-salary budget under USD 150,000. [Q1][Q2][Q4]

Verdict in one line: **conditionally feasible at 9 months with 4–5 engineers; not feasible at 6 months with 3 engineers; not feasible at all for the full 12-service target architecture inside this envelope.** Five conditions must hold before build commits (section 5).

---

## 1. Technical Viability

### 1.1 What is genuinely novel versus assembled

The product is not technically novel in any single component. It is an integration product: licensed market data, read-only exchange adapters, chain RPC readers, a domain-owned lot and valuation engine, and a React SPA. Every one of those has a known implementation path, and the build-vs-buy analysis already chose the correct boundary — buy data, integrate venues thinly, build the reconciliation and valuation logic that constitutes the differentiator. [build-vs-buy] That boundary is sound and reversible component by component through ports (`MarketDataProvider`, `FxProvider`, `ChainRpcProvider`, `ExchangeReader`, `CostBasisEngine`). [build-vs-buy]

The technical risk therefore sits almost entirely in the **platform constraints**, not in the product logic. Six constraints carry real uncertainty, and one of them — Supabase Vault from Rust — is the only item I would call unproven at the outset.

### 1.2 Hard-constraint assessment

| Constraint | Status | Assessment and required mitigation |
| --- | --- | --- |
| Supabase Vault access from Rust | **Unproven — spike first** | pgsodium is a Postgres extension; Rust has no native binding, so access is necessarily SQL through `sqlx` against Vault functions or the `vault.decrypted_secrets` view. This is workable but collides with the "no cross-schema grants" rule, because `vault` is a platform schema. Two viable designs: (a) a `SECURITY DEFINER` wrapper function per service, owned by a privileged role, returning only that service's named secrets; or (b) envelope encryption — the service fetches one master key from Vault at boot, holds it in memory with `zeroize`/`secrecy`, and stores exchange credentials as ciphertext in its own schema, decrypting per use. (b) is the better fit: one Vault read at boot, no per-request Vault round trip, credential blast radius limited to one service, and crypto-shredding falls out naturally from deleting a per-subject data key. Both designs must guarantee plaintext never reaches logs, error chains, traces or crash dumps. This is a week-one spike with a passing test that greps the log sink for the secret value. Treat Vault as an explicit documented exception to the no-cross-schema rule (a platform schema, not a service schema) — see the constraint register. |
| One schema and one Postgres role per service, no cross-schema grants | **Achievable, with a real cost** | The rule is clear and enforceable, but it removes joins and foreign keys across domains. Every cross-domain read becomes HTTP, so referential integrity becomes application-level and every aggregate becomes a fan-out. The valuation hot path needs holdings (Account) + asset mapping (Registry) + prices and FX (Market Data) + portfolio assembly (Portfolio): four hops. **A request-time fan-out will not reliably meet p95 <2 s.** Mitigation that must be designed in from the start, not retrofitted: Portfolio materialises a valuation snapshot on each price tick and serves the snapshot on read; request-time fan-out is reserved for an explicit user-initiated refresh. Each owning service must expose batch-by-ID read endpoints so callers avoid N+1. Cross-service writes need idempotency keys and compensating reconciliation, because distributed transactions do not exist here. |
| JWT validation in every service | **Proven pattern; discipline risk** | The gateway forwards the Supabase JWT unchanged and every service re-validates it. Correct and cheap, but it creates a per-service auth implementation that must not drift: JWKS fetch and cache with rotation and jitter, algorithm pinning (reject `alg: none` and algorithm substitution), `iss`/`aud` checks, clock-skew tolerance on `exp`/`nbf`, owner scoping from `sub`, and a distinct audience for the internal service token used by scheduled job runs so a job token can never be replayed as a user token. Extract this into one internal platform crate with one conformance test suite run against every service; do not let eight services each write their own. Also note the gateway's in-memory rate limits are per replica, so horizontal scaling silently multiplies the limit — acceptable at launch, a known defect at scale. [architecture] |
| Multi-source price reconciliation | **Achievable technically; commercially gated** | Deterministic source priority, a divergence threshold with an alert, and last-known-price fallback with a staleness badge is straightforward engineering, and it is what makes the >99% accuracy claim credible rather than asserted. [intent-statement] The blocker is licensing, not code: CoinGecko forbids redistribution and derived valuations without consent, and a cross-source median is itself a derived value. [build-vs-buy][market-trends] CoinMarketCap's tiers include commercial-use rights, which makes CMC the safer primary for valuation and CoinGecko a display/attribution source or a secondary check — the reverse of the current recommendation. Resolve this before it is baked into the schema. |
| Exchange read-only adapters | **Achievable; the largest and most underestimated line** | Five venues at 10–20 engineer-weeks is the single biggest build item and I believe it is optimistic. Each venue differs in auth (HMAC versus CDP JWT), rate-limit shape (Binance meters per IP and escalates 429 to a 418 ban of two minutes to three days; Kraken meters per key with a decaying counter), pagination, and key-permission defaults. APIs also churn: retired `listenKey` streams and the Coinbase Pro to Advanced Trade migration both invalidated existing integrations. [build-vs-buy] Partner programmes are contractually gated (Binance.US starts at USD 1M average monthly volume), so assume no partner status at launch. Mitigation: incremental sync with bounded backfill rather than full history on connect, per-venue circuit breakers, adapter contract tests against recorded fixtures, a per-venue capability matrix, and a user-visible "partial sync" state so a ban degrades to a labelled gap instead of a wrong number. Add 30–50% to the adapter estimate. |
| 99.9% availability on single-region Docker Swarm | **Design-achievable; not yet evidenced — and contractually risky** | 99.9% monthly is 43.2 minutes of downtime. On one self-managed Swarm cluster in one region that requires: at least two replicas of every service spread across at least three worker nodes, three manager nodes for quorum, health-checked rolling updates with `start-first`, an ingress and load balancer with health-based failover, and a backup/restore path that meets RPO ≤24 h / RTO ≤4 h. Even then, a botched cluster upgrade, a node drain, or an ingress fault can consume the whole monthly budget in one event, and a single-region topology keeps the ingress and the region as single points of failure. Two further facts: Supabase's own availability is inside the customer-visible number, so the SLA must either exclude upstream providers or the target must be stated as 99.9% of product-attributable time; and Docker Swarm is in maintenance mode with a slow security-update cadence, so the team owns node patching. Verdict: **the target is achievable by design but must not be sold as an SLA until a measured failover drill and a documented Supabase SLA exist.** Recommend a second cluster in the same region for blue/green and a CDN in front of the SPA. [architecture] |
| No message queues | **Acceptable at launch; define the ceiling** | Fire-and-forget HTTP with bounded retries plus scheduled reconciliation is a coherent substitute at launch scale, and it removes an entire operational component. The costs are real: no durable async, no backpressure, retry storms against a failing downstream, and best-effort audit events that can be lost (mitigated by local logging as fallback). Every write path needs an idempotency key or version check. The design breaks specifically on fan-out — a single price tick that must notify many users must become a scheduled digest, never per-event HTTP. Record a numeric threshold (for example notifications per minute or reconciliation lag) at which a queue is reconsidered, so the decision has a trigger rather than a vibe. |

### 1.3 Technical verdict

Every constraint is satisfiable with known patterns. The three that can actually sink the timeline are (1) the Vault access pattern, because it is the only unproven integration and it gates credential handling for the whole product; (2) the HTTP fan-out on the valuation path, because it directly determines whether the p95 <2 s target survives the no-cross-schema rule; and (3) the exchange adapters, because the effort estimate is the least reliable number in the plan. The two that can sink the **launch** rather than the build are market-data rights and MiCA scope (section 4). None of these is a reason to stop; all of them are reasons to sequence them first.

---

## 2. Delivery Feasibility and Timeline

### 2.1 The honest arithmetic

Three inputs: the team is 3–5 engineers for 6–9 months; the build-vs-buy bottom-up estimate is 64–114 engineer-weeks (15–27 engineer-months) for the product logic, adapters, SPA and compliance surface; and that estimate **does not include** the platform overhead of decomposing into eight or more independently deployed services. [Q1][build-vs-buy]

Platform overhead omitted from the bottom-up estimate:

| Item | Effort (engineer-weeks) |
| --- | --- |
| Eight Cargo workspaces at four crates each, schemas and roles, migrations, health and metrics endpoints | 8–12 |
| API Gateway (JWT, principal resolution, deny-by-default routing, entitlements, rate limits, SSE proxy) | 8–12 |
| Swarm deployment, ingress/TLS, secrets wiring, self-hosted observability stack | 6–10 |
| Job orchestration plus pg_cron wiring, single-flight locks, missed-run detection, heartbeat | 4–6 |
| Contract tests across service boundaries and the no-cross-schema proof | 3–5 |
| **Added** | **29–45 (≈7–10 engineer-months)** |

So the honest demand is **93–159 engineer-weeks, or 21–37 engineer-months** (roughly 17–30 engineer-months plus a 25% integration and contingency loading). Available capacity, at a realistic 80% feature utilisation after standups, reviews, on-call, legal liaison and hiring drag:

| Team and duration | Nominal engineer-months | Effective (80%) | Against 21–37 |
| --- | --- | --- | --- |
| 3 engineers × 6 months | 18 | 14.4 | **Short by 7–23. Not feasible.** |
| 4 engineers × 6 months | 24 | 19.2 | Short by 2–18 at most points. Not feasible. |
| 5 engineers × 6 months | 30 | 24.0 | Feasible only at the bottom of the range, with zero legal/ops time. Not recommended. |
| 3 engineers × 9 months | 27 | 21.6 | Feasible only at the very bottom, with no contingency. Fragile. |
| 4 engineers × 9 months | 36 | 28.8 | Feasible across the low and middle of the range. **Recommended minimum.** |
| 5 engineers × 9 months | 45 | 36.0 | Feasible including most of the high end. Recommended. |

The confirmed answer is "3–5 engineers, 6–9 months". The top-left of that box does not close, and the bottom-right does. **State the plan as 9 months with 4–5 engineers, or accept a materially thinner MVP.** A 6-month date is only reachable by removing whole services and deferring compliance work, which contradicts the launch intent.

### 2.2 The service-count problem the 8-service framing hides

"Eight services plus gateway and SPA" maps to the pre-extension baseline: Identity, Account, Asset Registry, Market Data, Portfolio, Reporting, Job Orchestration, Audit. That set cannot deliver the intent's own success metrics. [intent-statement] Two services are on the critical path and are missing from the count:

- **Ledger.** Cost basis, realised and unrealised P&L, transfer pairing and the reconciliation queue — the stated differentiator and the basis of the >99% accuracy claim — live in transaction history, not in valuation. [competitive-analysis][build-vs-buy] Folding that lot engine into Portfolio is possible but reverses the architecture's explicit decision to keep it separate, and it couples the highest-churn domain to the read path.
- **Billing.** Free-to-paid conversion ≥3%, paid plans with 99.9% availability, and entitlements and quotas enforced server-side all require plan and subscription state. [intent-statement][architecture] There is no way to charge without it.

The current target architecture in fact names **12 backend services plus gateway**, having added Ledger, Notification, Billing and Analytics, plus a Support Console. [architecture] So there are three different scope sizes in play, and the plan must pick one explicitly:

- **Option A — 8 services (thin, 6-month only):** cost-basis engine folded into Portfolio as a module; checkout handled by the merchant of record with entitlements read from a gateway cache rather than a service. Fastest, but weakens the differentiator and accumulates architectural debt that must be repaid before paid scale.
- **Option B — 10 launch services (recommended):** the eight plus Ledger and Billing; Notification, Analytics and Support Console deferred to post-MVP with a degraded path (in-app plus email via Reporting, product analytics from logs, support through a read-only admin view). This preserves the two load-bearing decisions and fits 9 months at 4–5 engineers.
- **Option C — full 12 services plus Support Console (not feasible):** the complete target architecture needs roughly 12–16 additional engineer-months over Option B and breaches the 9-month window even at 5 engineers.

**Recommendation: Option B, and record the deferral explicitly.** If the organisation insists on Option A, the timeline saving is about 2–3 engineer-months and the cost is a known architectural reversal; that is a defensible trade only if the launch claim drops P&L depth and paid plans.

### 2.3 Why "one service at a time" is right but serialises the critical path

The confirmed delivery shape — one service at a time behind the gateway — is the correct choice for this team size. It avoids the classic small-team failure of eight half-built services with no working end-to-end path, and it keeps the gateway contract stable while each service lands. The cost is that the critical path is genuinely serial, which is exactly why the engineer count matters: with 3 engineers there is one stream and no parallelism; with 4–5 there are two safe streams (backend services, and SPA plus adapters), which is what moves the plan from 9 months of thin delivery to 9 months of viable delivery.

### 2.4 Proposed thin-slice sequence

The sequence below front-loads risk. The first milestone exists to falsify the six hard constraints in six weeks, not to deliver features. Each milestone has an exit test; a failed exit stops the sequence rather than accumulating debt.

| Milestone | Weeks | Slice | Exit test |
| --- | --- | --- | --- |
| **M0 — Walking skeleton** | 0–6 | Gateway, Identity, Asset Registry (minimal), Market Data (one source, one FX source), Portfolio (single portfolio valuation), one exchange adapter, SPA first-valuation screen. All eight constraints exercised at least once: Vault read, per-service schema and role, JWT in every hop, Swarm with two replicas, no queue. | A real user signs in, connects one read-only exchange key, and sees a base-currency valuation of that venue within 60 seconds of connecting. Failover drill: kill one node, the dashboard stays up. Log sink contains no secret material. **If this is not green by week 6, re-plan before writing more code.** |
| **M1 — Coverage and trust surface** | 6–14 | Remaining four venue adapters, EVM and Solana readers, Bitcoin and Tron readers, price freshness tiers (5 min top-100, 15 min tail), staleness badges, unpriced and unmapped labelling, source divergence alerting. | All five venues and four chain families sync for a test account; a deliberately stale source produces a visible badge and never a silently wrong total; divergence between sources raises an alert. |
| **M2 — The differentiator** | 12–22 | Ledger: transaction import and dedupe, transfer pairing, cost-basis lot engine, realised and unrealised P&L, and the user-facing reconciliation queue. | >99% valuation accuracy against a manually reconciled reference account; a transfer between two venues pairs automatically; a missing cost basis is detected and surfaced, not guessed. [intent-statement] |
| **M3 — Decision support and retention** | 20–28 | Portfolio targets and drift, immutable snapshots, local-currency P&L, Reporting briefs, Audit append-only path, Job orchestration hardened (missed-run detection, heartbeat, bounded backfill). | Drift alert reaches the user; a missed scheduled run is detected within 5 minutes and replayed; audit writes survive an insert-only role test. |
| **M4 — Commercial and hardening** | 26–34 | Billing: plans, entitlements, quotas, merchant-of-record checkout and dunning. Plus localization, WCAG 2.2 AA audit, PWA shell, observability dashboards and alerts. | A test purchase grants entitlement within one minute; a downgrade removes gated access; accessibility audit passes; every service has a latency and error-rate dashboard. |
| **M5 — Launch gate** | 32–39 | Jurisdiction gate (fail-closed allowlist), DPIA, disclosures and terms per jurisdiction, privacy export and erasure by crypto-shredding, penetration test and remediation, load test at target concurrency, restore drill. | Counsel sign-off on MiCA and UK scope; a blocked jurisdiction cannot collect data; restore from backup meets RTO ≤4 h; pen-test findings closed or explicitly accepted. |

Notes on sequencing that matter more than the dates: the Vault spike and the failover drill are in M0 because they are the only two items that can invalidate the architecture. The jurisdiction gate is in M5 for build sequence but must be shipped **before any public beta**, not before GA — data collection is the trigger, per the confirmed answer. [Q4] Market-data rights (section 4) run in parallel from week one because they are a contract, not an engineering task, and have the longest lead time.

---

## 3. Budget and Funding Fit

### 3.1 Two separate numbers, kept separate

The confirmed interpretation is that the **USD 150,000 envelope covers non-salary spend only** — vendors, infrastructure, legal and contingency — because the team is already funded. [Q2][Q5] The build-vs-buy figure of **USD 150,000–405,000 is a fully loaded build effort** at USD 10,000–15,000 per engineer-month and is **not** the same money. [build-vs-buy] Conflating them was the original contradiction; the two must never be summed in reporting. Sanity check on the salary side, stated only to confirm the interpretation is coherent: 3–5 engineers × 6–9 months is 18–45 engineer-months, or roughly USD 180,000–675,000 fully loaded — the confirmed answer is the only reading under which the envelope is not already breached before a line of infrastructure is bought.

### 3.2 Non-salary envelope, 9-month plan

Planning ranges, not quotations. Vendor lines are drawn from the market research; hosting is a bottom-up estimate for a small Swarm cluster.

| Line | 9-month estimate (USD) | Basis |
| --- | --- | --- |
| Market data, FX and RPC vendors | 3,700–6,400 | USD 357–557/month vendor run-rate plus USD 50–150/month self-hosted Esplora. [build-vs-buy] |
| Hosting — Swarm VPS, ingress/LB, CDN, object storage, backups | 4,500–9,000 | 3 managers + 3 workers at roughly USD 40–80/node/month, plus edge and storage. |
| Supabase (Pro plus compute, PITR, pooler headroom) | 900–3,600 | Scales with database size and backup retention. |
| Email, web push, status page, help desk | 500–1,800 | Low tier at launch volume. |
| Observability tooling | 0–2,700 | Self-hosted Grafana stack costs compute only (already counted); managed is the higher figure. |
| **Legal — EU/UK/US scope opinion, disclosures, DPIA, terms, privacy** | **25,000–60,000** | The dominant line and the least predictable. External counsel validates disclosures per the confirmed answer. [Q4] |
| Penetration test and remediation | 8,000–20,000 | Required before handling exchange credentials. |
| Accessibility audit (WCAG 2.2 AA / EAA) | 5,000–15,000 | Legally required for EU consumer services. [market-trends] |
| Domain, certificates, miscellaneous | 500–2,000 | — |
| **Subtotal** | **48,100–121,500** | |
| Contingency (20–25%) | 9,600–30,400 | Non-negotiable given the legal and licensing uncertainty. |
| **Total** | **57,700–151,900** | |

### 3.3 Verdict on the budget

**Fits, but only just, and only in the low-to-middle case.** A lean plan — one-market legal scope opinion rather than three, modest VPS sizing, self-hosted observability — lands near USD 60,000–90,000, leaving 40–60% headroom. The high case, with full three-jurisdiction legal scope, a pen test, an accessibility audit and 9 months of run-rate, consumes the entire USD 150,000. Three consequences:

1. **Ring-fence legal at about 40% of the envelope.** It is the line most likely to overrun and the one that cannot be descoped at the last minute.
2. **Hold at least 20% contingency and treat it as untouchable** until launch. A single failed pen-test cycle or a second counsel opinion erases a 10% buffer.
3. **The envelope has no line for a commercial market-data licence.** If CoinGecko or CoinMarketCap requires a paid licence for derived valuations and cross-source medians, that is an unbudgeted five-figure annual cost and a direct hit on the contingency. It must be priced before the envelope is committed — this is the single largest financial unknown in the plan.

Also outside the envelope and therefore outside this assessment: user acquisition, customer support headcount, payment-processing fees (a percentage of revenue, absorbed by the merchant of record), and entity formation and insurance. The plan should state that explicitly rather than discover it at launch.

---

## 4. Regulatory and Jurisdictional Feasibility

### 4.1 The product boundary is defensible but narrow

The intent deliberately places the product outside custody and execution: read-only connections, no keys, no transfers, no automatic trades, and a human-approved order intent that is never transmitted as an order. [intent-statement] Under MiCA, non-custodial wallet software is excluded from scope (Recital 83), which is the basis on which this product can launch in the EU without CASP authorisation. Two activities remain live hazards:

- **Personalised advice and discretionary portfolio management** are licensed under Art 3(1)(16)(h)–(i) and (24)–(25). Drift alerts, rebalancing proposals and "what should I buy" copy drift toward this line quickly, especially in a retail product optimised for activation. Decision support must stay generic and factual: show the position, the drift and the option, never a recommendation addressed to the user.
- **Reception and transmission of orders** is separately licensed under (e) and (g). An order intent is safe only as a **documented handoff to a licensed venue** — recorded, shown, exported, never routed or forwarded on the user's behalf. [competitive-analysis] The moment the product transmits an intent to an exchange, it is in scope.

That boundary is technically enforceable because the product never holds exchange trade permission or user keys, and adapters are read-only by construction. It is **not** enforceable by architecture alone; it needs copy review and a design principle that survives product pressure. [market-trends]

### 4.2 Jurisdiction sequencing versus the launch date

The confirmed answer is EU (MiCA) and UK first, then US, with a gate before data collection and external counsel validating disclosures. [Q4] Two timing facts complicate the "UK first" half:

- The UK's new cryptoasset regime opened applications on 30 September 2026 and is expected in force around 25 October 2027 — **after** a 9-month MVP that lands mid-2027. [market-trends] So a UK launch inside this plan happens under the current regime, and the question counsel must answer is whether a read-only tracker needs authorisation today, and whether the order-intent surface and any UK-facing marketing create a regulated activity. The new regime then becomes a hard gate at its commencement, which falls just after launch and must be budgeted in the follow-on plan.
- EU grandfathering ended 1 July 2026 and the review consultations closed 31 August 2026, so the rules may move again during the build. [market-trends] A single legal opinion dated at the start of the build is not sufficient; it needs a review point before launch.

The US is correctly sequenced last. CLARITY passed the House but failed Senate cloture in September 2026, so there is no stable federal frame; state money-transmission exposure is the risk if an order intent is ever construed as transmission. [market-trends] Read-only tracking is generally outside MSB scope, but this needs the same counsel opinion, and 1099-DA plus DAC8 and CARF create cost-basis demand that the product serves without filing. [market-trends]

### 4.3 Non-negotiable launch gates

1. **Jurisdiction gate before any data collection.** Country selection at onboarding against a counsel-supplied allowlist, fail-closed, with an audit event on every block. It must exist before public beta, and it must be owned by Identity so no other service can bypass it.
2. **DPIA.** Wallet addresses and keyed hashes can be personal data; the EDPB guidelines make a DPIA necessary, and privacy export plus erasure by crypto-shredding must be designed with the data model, not after it. [market-trends]
3. **Accessibility.** The European Accessibility Act has applied to EU consumer services since 28 June 2025, against WCAG 2.2 AA. [market-trends] This is a legal requirement, not a quality goal, and it needs an external audit before EU launch.
4. **Disclosures and terms per jurisdiction**, validated by external counsel — the confirmed condition. [Q4]
5. **Market-data rights.** Written confirmation that the chosen sources permit display, redistribution and derived valuations in the launch jurisdictions. This is a **commercial** gate that blocks the same launch as the legal ones.

### 4.4 Regulatory verdict

Feasible, with the product boundary as the load-bearing element. The plan is not blocked by licensing, but it is blocked from launch until the market-data rights question and the MiCA scope opinion are answered in writing. Both are longer-lead than any engineering task in the plan and should start in week one.

---

## 5. Go/No-Go Recommendation

**Recommendation: CONDITIONAL GO.**

Go for a thin-slice MVP at **9 months with 4–5 engineers**, on **Option B — 10 launch services plus gateway and SPA** (the eight baseline services plus Ledger and Billing; Notification, Analytics and Support Console deferred), with legal and market-data workstreams running from week 0 and the jurisdiction gate shipped before public beta.

Do **not** go on a 6-month date with 3 engineers; do **not** go for the full 12-service target architecture inside this envelope; do **not** open paid plans before condition C4 is met.

Conditions, each with an owner and a test, because an untestable condition is a hope:

| # | Condition | Test / evidence | Gate |
| --- | --- | --- | --- |
| **C1** | Market-data rights secured. Written confirmation that at least one source permits display and derived valuations (including cross-source medians) in the launch jurisdictions, and that a second source permits cross-checking. | Signed terms or written consent on file; if unavailable, CoinGecko is removed from the valuation path and CMC alone is the primary. | Before public beta |
| **C2** | MiCA and UK scope opinion. External counsel confirms the product is outside CASP authorisation, that order intents are a documented handoff and not reception/transmission, and that decision support stays generic. | Written opinion covering EU and UK; a copy-review checklist derived from it. | Before any EU/UK data collection |
| **C3** | Vault-from-Rust pattern proven. A service reads a credential through the chosen pattern, survives key rotation, and produces zero plaintext in logs, traces and error output. | Automated test that greps the log sink for the secret; rotation drill passes. | Week 2 of M0 |
| **C4** | Availability evidenced, not asserted. Two replicas per service across three or more nodes, a measured node-kill and replica-kill drill, and a documented Supabase SLA. SLA scope excludes upstream provider outages. | Drill report with measured recovery time; signed SLA reference. | Before paid plans open |
| **C5** | Scope locked at Option B (or Option A accepted with its architectural cost recorded). Deferral of Notification, Analytics and Support Console is written into the scope, not implied. | Approved scope statement naming the 10 services and the deferrals. | Before delivery planning |
| **C6** | Capacity confirmed. At least 4 engineers, at least one with production Swarm and Postgres operations experience, and at least 21 effective engineer-months available. | Staffing plan and calendar. | Before build commit |
| **C7** | Budget release plan. Legal ring-fenced at about 40% of the USD 150,000, contingency at 20% or more, and the market-data licence cost identified or explicitly zero. | Finance-approved release schedule. | Before build commit |
| **C8** | Jurisdiction gate fail-closed and audited; DPIA complete; accessibility audit passed. | Gate test for a blocked country; DPIA sign-off; audit report. | Before public beta |
| **C9** | Valuation path meets p95 <2 s with the materialised-snapshot design, under target concurrency. | Load test at launch concurrency with the snapshot path. | Before paid plans open |

**Kill criteria — stop, do not re-plan, if:** C1 cannot be obtained on commercially acceptable terms (the product cannot display a valuation it is not licensed to derive); C2 finds order intents or decision support inside MiCA scope without a licensed partner; or the M0 walking skeleton is not green by week 6 (the architecture's hard constraints would then be unproven with most of the budget unspent, which is the cheapest possible time to stop).

**Why not a clean yes:** two of the five hardest constraints (Vault access, availability evidence) are unproven rather than merely hard, and the two launch-blocking dependencies (market-data rights, MiCA scope) are commercial and legal rather than technical and cannot be resolved by engineering effort. A clean yes would be a claim the evidence does not support. **Why not a no:** every technical constraint has a known implementation path, the build-versus-buy boundary is sound, the budget fits in the lean case, and the market gap is real and specific — reconciliation quality is the category's shared failure mode, which is exactly the thing this product is built to fix. [competitive-analysis]

---

## 6. Assumptions & Open Questions

### Assumptions made in this assessment

- The MVP service count is resolved as Option B (10 launch services) before delivery planning; the 8-service framing in the delivery question understates by two services that the stated success metrics require. [Q1][intent-statement]
- The 3–5 engineer team is productive from month one; no hiring drag and no more than 20% of capacity consumed by on-call, reviews and legal liaison.
- Supabase's contractual availability and PITR meet or exceed what a 99.9% customer-visible target requires; this is assumed, not verified.
- Launch venues are Coinbase, Kraken and Binance plus two to be named, and no partner-programme access is required for read-only personal keys.
- External counsel is available, competent in MiCA and UK cryptoasset rules, and can deliver inside the budget line above.
- The merchant of record absorbs subscription tax and remittance, so no per-jurisdiction tax registration is needed at launch. [architecture]
- The USD 10,000–15,000 per engineer-month figure is a planning rate and is not re-litigated here; it belongs to the salary side and is outside the envelope. [build-vs-buy][Q5]

### Open questions that can still change the verdict

1. **Does a cross-source median count as a derived valuation** under CoinGecko's and CoinMarketCap's terms? If yes for both, the dual-source accuracy strategy needs rework and >99% may be unachievable through licensed retail data alone. Largest single risk in the plan.
2. **Does MiCA treat a documented, untransmitted order intent as reception/transmission?** Counsel's answer decides whether the order-intent surface ships at launch or moves post-MVP behind a licensed partner.
3. **Is a UK launch lawful before the FCA regime commences (expected October 2027),** and does UK-facing marketing change that? Determines whether "UK first" is a launch market or a pre-positioning market.
4. **What is Supabase's actual SLA and does PITR cover the RPO ≤24 h / RTO ≤4 h targets?** If not, the availability target and the paid-plan promise both move.
5. **How are Vault keys backed up without defeating crypto-shredding?** Restoring the database without the per-subject keys yields unreadable credentials; backing the keys up makes erasure a policy promise rather than a cryptographic one. This is a genuine architectural tension and needs a decision, not a default.
6. **Is a second region or a second cluster funded?** Single-region single-cluster keeps the ingress and the region as single points of failure; the 99.9% target is materially safer with one of them removed, and the high-case budget does not obviously contain it.
7. **Who operates the Swarm cluster on call?** There is no dedicated operations role in a 3–5 person team, and the decision to self-manage the orchestrator is explicitly accepted in the architecture. [architecture] The feasibility plan must name the person or accept the availability risk.
8. **Which indexer provides EVM token discovery at acceptable cost?** Log scanning across four chain families is the most underestimated line after the exchange adapters, and the RPC overage allowance (up to USD 200/month) may not cover it.
9. **Is the 4-week pen-test and remediation window inside or outside the 9 months?** If outside, launch slips a month; the plan should say which.
10. **Does the market-data licence fee exist, and at what annual price?** Until answered, the USD 150,000 envelope has an unpriced liability in it.
