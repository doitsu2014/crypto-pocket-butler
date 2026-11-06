# Requirements — Crypto Pocket Butler

**Stage:** inception/requirements-analysis · **Source anchor:** `docs/brds/BRD-001-product-vision-scope-and-success.md` (v1.2), with `docs/brds/BRD-012-security-compliance-and-non-functional-requirements.md` and the ideation artifacts under `ideation/`.

## Intent Analysis

Crypto Pocket Butler gives retail investors one trustworthy view of a crypto portfolio held across exchanges, wallets, and chains: what they own, what it is worth, and whether they are up or down. It is a read-only aggregation and decision-support layer with user-initiated order intents. It never custodies assets or keys, never places trades, and never initiates transfers.

The goals below are carried from BRD-001 §Goals:

| ID | Goal |
|----|------|
| G-01 | Consolidate holdings across venues into one portfolio view |
| G-02 | Trustworthy valuation that reconciles with manual spot-checks |
| G-03 | Make drift from target allocation visible |
| G-04 | Answer "am I up or down?" with net invested and realized/unrealized P&L |
| G-05 | Turn the view into action through order intents and recorded trades |
| G-06 | Remove manual reporting through a scheduled brief |
| G-07 | Protect credentials and privacy |
| G-08 | Preserve history through immutable snapshots and transactions |
| G-09 | Work globally: base currency, locale, timezone, jurisdiction |
| G-10 | Retain users through notifications and activation |
| G-11 | Be trustworthy and lawful at launch |
| G-12 | Sustain the business — **deferred**: the product is free and open source in this release (see Scope Decisions) |

## Scope Decisions From This Interview

These decisions refine the BRD and ideation baseline. Where they narrow a documented Must, the narrowing is the decision of record for this release.

| Area | Decision | Effect on the BRDs |
|------|----------|--------------------|
| Launch markets and languages | Vietnam plus the EU/UK markets; UI in English and Vietnamese | Narrows BRD-001 OQ-001 and the scope document's EU/UK-only assumption |
| Launch coverage | Two exchanges — Binance and OKX — and three chains: EVM, BEP20 (BNB Smart Chain), Solana | Narrows BRD-001's "≥5 exchanges, ≥10 chains" target; the coverage target is re-baselined to 2 exchanges and 3 chains for this release |
| Packaging and billing | The product is free and open source. Billing, plans, entitlements, quotas, checkout, dunning, and the merchant of record are removed from this release | Removes BRD-016 and the commercial parts of BRD-001 packaging |
| Native mobile | PWA only in the first year; native is decided after the beta | Matches BRD-001 OQ-005 as post-MVP |
| Retention | Two years for financial records, six months for audit logs | Supersedes the scope document's 5-year / 12-month proposal |
| Availability | 99.9% monthly, published as a target with no service credits, for all users | Replaces "for paid plans", since there are no paid plans |
| Must boundary | The committed Must set is the baseline; the Should fast-follows stay out | Confirms the scope document's cut |

Because coverage and billing are reduced, the scope document's 186.0 engineer-week baseline no longer describes this release. Effort must be re-estimated at Units Generation; this document does not carry the old figure forward.

## Functional Requirements

Each requirement states its testable acceptance criterion. IDs are permanent traceability keys.

### FR1 — Identity, sessions, and access

- **FR1.1** The system shall let a user sign up and sign in through Supabase Auth (email/password and supported social providers).
  - *Acceptance:* Given a valid sign-up, when the user completes verification, then a session is created and the user reaches onboarding.
- **FR1.2** The system shall offer MFA to every user and require it for any user who has stored exchange credentials.
  - *Acceptance:* Given a user with an exchange account and no MFA, when they sign in, then they are prompted to enrol before portfolio data is shown.
- **FR1.3** The system shall keep browser session tokens in memory only and never in `localStorage`.
  - *Acceptance:* Given a signed-in browser session, when storage is inspected, then no session token is present in persistent storage.
- **FR1.4** The system shall list a user's devices and sessions and let them revoke any of them.
  - *Acceptance:* Given two active sessions, when the user revokes one, then that session is signed out at its next request and the other continues.
- **FR1.5** The system shall authorise admin and support capabilities from the role claim in Supabase `app_metadata.role`.
  - *Acceptance:* Given a user without the admin role, when they call an admin endpoint, then the request is denied and the denial is audited.
- **FR1.6** The system shall enforce an ownership check on every read and write of a user-owned resource, including lists, searches, and exports.
  - *Acceptance:* Given user A's portfolio, when user B requests it by id, then the response is not-found or forbidden and no data is returned.

### FR2 — Accounts and imports

- **FR2.1** The system shall let a user add read-only exchange accounts for Binance and OKX using API keys, and verify the key's permissions before use and periodically.
  - *Acceptance:* Given a key with withdrawal or trading permission, when it is added, then it is rejected with a reason and no account is stored.
- **FR2.2** The system shall let a user add public wallet addresses for EVM chains, BNB Smart Chain (BEP20 tokens), and Solana, and WalletConnect for read-only sessions.
  - *Acceptance:* Given a supported address, when it is added, then its balances appear in the portfolio and its last-synced time is shown.
- **FR2.3** The system shall support CSV import as the third import method.
  - *Acceptance:* Given a CSV with valid rows, when it is imported, then the rows are stored, duplicates are skipped, and a summary of imported and skipped rows is shown.
- **FR2.4** The system shall combine multiple accounts into portfolios and support more than one portfolio per user.
  - *Acceptance:* Given two accounts and two portfolios, when the user assigns accounts, then each portfolio values only its own accounts.
- **FR2.5** The system shall show each account's sync status and last-synced time, and offer a manual re-sync.
  - *Acceptance:* Given a stale account, when the user triggers a re-sync, then the status shows in-progress and then a new last-synced time or a visible failure.
- **FR2.6** The system shall never request, store, or accept wallet private keys or seed phrases, and shall never enable withdrawals or trading permissions on an integration.
  - *Acceptance:* Given any onboarding flow, when it is completed, then no private key or seed phrase is collected and every stored credential is verified read-only.

### FR3 — Asset registry and market data

- **FR3.1** The system shall maintain a canonical registry of assets, contract addresses, and decimal precision, administered through audited admin changes.
  - *Acceptance:* Given a token on two chains, when prices are combined, then the two contract entries resolve to one canonical asset without double counting.
- **FR3.2** The system shall price assets from multiple sources with a defined priority and fallback.
  - *Acceptance:* Given the primary source failing, when a price is requested, then the fallback source is used and its provenance is recorded.
- **FR3.3** The system shall convert values into the user's base currency using FX rates.
  - *Acceptance:* Given a EUR-denominated asset and a USD base currency, when the portfolio is valued, then the FX rate and its timestamp are shown.
- **FR3.4** The system shall show price freshness and provenance for every valuation.
  - *Acceptance:* Given a price older than the freshness bound, when the portfolio is viewed, then the asset is marked stale with its timestamp.
- **FR3.5** The system shall mark an asset without a sourceable price as unpriced and never guess or treat it as free.
  - *Acceptance:* Given an asset with no price source, when the portfolio is valued, then it is excluded from the priced total and labelled unpriced.

### FR4 — Portfolios, targets, valuation, and snapshots

- **FR4.1** The system shall show one consolidated portfolio view in the user's base currency.
  - *Acceptance:* Given a portfolio with multiple accounts, when it is opened, then a single total value and per-asset breakdown are shown.
- **FR4.2** The system shall let a user set a target allocation per asset and optional guardrails.
  - *Acceptance:* Given a target of 50% BTC, when the actual is 60%, then drift is shown as +10 percentage points.
- **FR4.3** The system shall show drift per asset for every portfolio with a target.
  - *Acceptance:* Given a portfolio with targets, when it is viewed, then every asset shows actual, target, and drift.
- **FR4.4** The system shall record immutable snapshots and show their history.
  - *Acceptance:* Given a snapshot, when later data changes, then the snapshot's stored values are unchanged and reproducible from stored inputs.
- **FR4.5** The system shall support a user-selected base currency, locale, and timezone.
  - *Acceptance:* Given a user changing base currency, when the portfolio reloads, then all values are shown in the new currency with the new FX rates.

### FR5 — Transactions, cost basis, and P&L

- **FR5.1** The system shall record transactions in an immutable ledger with deduplication across import sources.
  - *Acceptance:* Given the same trade imported from an exchange and a CSV, when both are processed, then one ledger entry exists and the duplicate is reported.
- **FR5.2** The system shall compute cost basis, net invested, and realized and unrealized P&L per asset and per portfolio.
  - *Acceptance:* Given a buy and a partial sell, when P&L is shown, then realized and unrealized amounts reconcile to the ledger.
- **FR5.3** The system shall label P&L coverage and never fabricate cost basis for missing history.
  - *Acceptance:* Given a portfolio with no acquisition history, when P&L is requested, then it is shown as unavailable with a coverage label rather than estimated.
- **FR5.4** The system shall let a user correct or adjust a transaction, recording the adjustment in the audit trail.
  - *Acceptance:* Given a wrong quantity, when the user corrects it, then the ledger shows the correction and the original remains recoverable.

### FR6 — Order intents and recorded trades

- **FR6.1** The system shall let a user create buy and sell order intents from the portfolio view.
  - *Acceptance:* Given a portfolio, when the user creates a buy intent, then the intent is recorded with asset, quantity, and target portfolio.
- **FR6.2** The system shall show a projected allocation preview before an intent is recorded.
  - *Acceptance:* Given a buy intent, when the preview is shown, then post-trade allocation and drift are projected.
- **FR6.3** The system shall let a user cancel a pending intent and shall never place an order itself.
  - *Acceptance:* Given a pending intent, when the user cancels it, then it is marked cancelled and no external order call is made.
- **FR6.4** The system shall let a user record an executed trade, and a recorded trade shall affect allocation immediately.
  - *Acceptance:* Given a recorded trade, when the portfolio reloads, then allocation and drift reflect it without waiting for the next sync.

### FR7 — Briefs and notifications

- **FR7.1** The system shall generate a scheduled portfolio brief readable in-app and by email.
  - *Acceptance:* Given a scheduled brief time, when it fires, then the brief is generated and delivered, or explicitly marked failed with a reason.
- **FR7.2** The system shall deliver in-app and email notifications with per-type preferences.
  - *Acceptance:* Given a disabled type, when the triggering event occurs, then no notification of that type is delivered.
- **FR7.3** The system shall send mandatory email for security, credential, and billing-type events, with one-click unsubscribe where applicable.
  - *Acceptance:* Given a credential revocation event, when it occurs, then an email is sent even if general notifications are off.
- **FR7.4** The system shall notify when drift exceeds a configured guardrail.
  - *Acceptance:* Given a guardrail of 5 points and a drift of 6, when the next evaluation runs, then a notification is produced once, not repeatedly.

### FR8 — Privacy, export, and deletion

- **FR8.1** The system shall let a user export their data in a machine-readable format.
  - *Acceptance:* Given an export request, when it completes, then a file containing accounts, portfolios, transactions, and snapshots is produced.
- **FR8.2** The system shall let a user delete their account in-app, subject to legal retention.
  - *Acceptance:* Given a deletion request, when confirmed, then personal data is deleted or anonymised and retained financial records are labelled.
- **FR8.3** The system shall collect consent for marketing and non-essential analytics, with no pre-ticked boxes.
  - *Acceptance:* Given a new user, when analytics is non-essential, then no analytics event is sent before consent.
- **FR8.4** The system shall gate age and jurisdiction before collecting personal data.
  - *Acceptance:* Given a user in a blocked jurisdiction, when they reach sign-up, then access is denied before any personal data is collected.

### FR9 — Localization and accessibility

- **FR9.1** The system shall provide the UI in English and Vietnamese at launch.
  - *Acceptance:* Given a Vietnamese locale, when the UI loads, then all core flows are shown in Vietnamese.
- **FR9.2** The system shall meet WCAG 2.2 AA on core flows.
  - *Acceptance:* Given a core flow, when audited, then keyboard, screen-reader, contrast, and zoom checks pass with zero blockers.
- **FR9.3** The system shall never convey status by colour alone and shall show a missing financial value as an em dash, never `0` or `$0.00`.
  - *Acceptance:* Given a missing value, when it is rendered, then an em dash and an accessible label are shown.

### FR10 — Jobs, refresh, and reliability

- **FR10.1** The system shall run scheduled jobs for price refresh, transaction sync, FX, and notification delivery.
  - *Acceptance:* Given a schedule, when the interval elapses, then the job runs and records its outcome.
- **FR10.2** The system shall detect a missed or silently failed job and alert within five minutes.
  - *Acceptance:* Given a stopped scheduler, when the heartbeat is missed, then an alert fires within five minutes.
- **FR10.3** The system shall retry provider calls with bounded retries and idempotency keys, never repeating a side effect.
  - *Acceptance:* Given a transient provider failure, when the call is retried, then the side effect is applied once.

### FR11 — Support, status, and administration

- **FR11.1** The system shall provide a help centre, a support channel, and a public status page.
  - *Acceptance:* Given a service incident, when it is declared, then the status page shows it and SEV-1 communication goes out within 15 minutes.
- **FR11.2** The system shall provide support access that is read-only, time-bound, reason-recorded, and audited.
  - *Acceptance:* Given a support session, when it expires, then access ends and the session is recorded in the audit trail.
- **FR11.3** The system shall audit administrative changes to the asset registry and configuration.
  - *Acceptance:* Given an admin registry change, when it is saved, then an audit record names actor, target, time, and outcome.

### FR12 — Onboarding and analytics

- **FR12.1** The system shall guide a new user to a first valuation, by address in under two minutes and by exchange key in under ten.
  - *Acceptance:* Given a new user adding a supported address, when the flow completes, then a valuation is shown and the elapsed time is recorded.
- **FR12.2** The system shall measure activation, engagement, retention, conversion, and quality with consent-aware, PII-free analytics.
  - *Acceptance:* Given analytics events, when inspected, then no personal data or credential is present and only consented users are tracked.
- **FR12.3** The system shall provide feature flags with an owner, an expiry, and a rollout percentage.
  - *Acceptance:* Given a flag without an owner or expiry, when it is created, then creation is refused.

### FR13 — API surface

- **FR13.1** The system shall expose a documented API to the web client through a single API gateway; the only browser-external calls are Supabase Auth and WalletConnect.
  - *Acceptance:* Given the SPA, when it calls a service, then the call goes through the gateway and the service validates the JWT against Supabase JWKS.
- **FR13.2** The system shall apply deny-by-default routing, per-user/IP/region rate limits, and correlation IDs at the gateway.
  - *Acceptance:* Given an unlisted route, when it is called, then the gateway denies it and records the correlation id.

## Non-Functional Requirements

| ID | Requirement | Target | Source |
|----|-------------|--------|--------|
| NFR1 | Monthly availability for all users | ≥99.9%, published as a target with no service credits | BRD-012 NFR-AVL-01; interview Q6 |
| NFR2 | Dashboard response time | p95 <2 s | BRD-012 NFR-PERF-02 |
| NFR3 | Snapshot history response | p95 <2 s for one year of history | BRD-012 NFR-PERF-05 |
| NFR4 | Single-account sync feedback | <30 s typical | BRD-012 NFR-PERF-03 |
| NFR5 | Price freshness | ≤5 min top-100 assets; ≤15 min tail; timestamp always shown | BRD-012 NFR-PERF-01 |
| NFR6 | Time to first value | <2 min by address; <10 min by exchange key | BRD-012 NFR-PERF-04 |
| NFR7 | Valuation accuracy | >99% versus manual spot-check; decimal-normalisation errors zero and surfaced | BRD-012 NFR-ACC-01/02 |
| NFR8 | Snapshot and P&L integrity | Snapshots reproducible from stored inputs 100%; P&L reconciles to the ledger 100% or is labelled | BRD-012 NFR-ACC-03/04 |
| NFR9 | Transport and secret protection | TLS on all production traffic; exchange credentials encrypted in Supabase Vault; never in responses, logs, errors, analytics, or support screens | BRD-012 NFR-SEC-01/02/03/15 |
| NFR10 | Token and session security | JWTs validated against Supabase JWKS; browser tokens in memory only; MFA required with exchange credentials | BRD-012 NFR-SEC-09/10/11 |
| NFR11 | Abuse protection | Rate limits per user/IP/region at the gateway; stricter for auth and manual triggers | BRD-012 NFR-SEC-12 |
| NFR12 | Supply chain | Dependency and container-image vulnerability scanning before release | BRD-012 NFR-SEC-14 |
| NFR13 | Privacy | Data minimisation, consent, export, deletion, and processor disclosure | BRD-012 NFR-PRV-01–15 |
| NFR14 | Retention | Two years for financial records, six months for audit logs, automated | Interview Q5 |
| NFR15 | Accessibility | WCAG 2.2 AA on core flows; keyboard, screen reader, contrast, motion | BRD-012 NFR-A11Y-01–05 |
| NFR16 | Localization | English and Vietnamese; user-selected base currency, locale, timezone | Interview Q1/Q8; BRD-018 |
| NFR17 | Observability | Structured logs with correlation IDs, health and readiness endpoints, latency/error/job metrics, actionable alerts | BRD-012 NFR-OBS-01–04 |
| NFR18 | Recovery | RPO ≤24 h; RTO ≤4 h | BRD-012 NFR-AVL-04/05 |
| NFR19 | Audit | Append-only records with actor, action, target, timestamp, context, outcome; insert-only role; no credentials, tokens, or full addresses | BRD-012 audit logging |
| NFR20 | Scalability | Horizontal scaling; launch capacity defined and reviewed against Supabase limits | BRD-012 NFR-SCL-01 |

## Constraints

- **Stack (fixed):** Rust with Axum, tower, sqlx, rust_decimal, tokio for backend services and the gateway; React 18 + TypeScript 5 with Vite, TanStack Query, Tailwind CSS v4 on Parchment Ledger tokens; Yarn ≥ 6 only.
- **Clean Architecture:** each Rust service is a Cargo workspace of `domain`, `application`, `infrastructure`, `http`, with dependencies pointing inward only.
- **Database:** Supabase Postgres; one schema and one Postgres role per service; no cross-schema grants; no Row Level Security in services; cross-service reads only through the owning service's HTTP API.
- **Communication:** synchronous internal HTTP with bounded retries and idempotency keys; no message queues.
- **Deployment:** Docker Swarm, production only, rolling updates with health checks, versioned up/down sqlx migrations; secrets from Supabase Vault.
- **Testing:** TDD for all production code; 80% line coverage per Rust crate; requirement-driven tests with a happy-path floor.
- **Product boundary:** read-only; no custody or private keys; no automatic trade execution; no product-initiated transfers; no fiat on/off-ramp; no tax filing or regulated advice; no futures or margin.

## Assumptions

- Binance and OKX API terms permit read-only commercial use in Vietnam, the EU, and the UK; this is checked before launch.
- "EVM" means Ethereum-compatible chains; "BEP20" means BNB Smart Chain tokens; "SOL" means Solana. The exact EVM chain list is settled with the registry work.
- USD is the default base currency and the accounting fallback; the user may change it.
- An asset without a sourceable price is unpriced, never guessed.
- Public addresses only; no private keys.
- Web push is deferred: in-app and email notifications ship at launch, matching the scope document's partial on FR-GEN-13.
- Because there are no paid plans, the 99.9% availability target applies to every user rather than to paid plans only.
- Native mobile is decided after the beta; the PWA is the mobile path.
- A merchant of record is not needed because there is no billing.

## Out of Scope

- **Removed by interview:** billing, plans, entitlements, quotas, checkout, dunning, merchant of record, and all pricing.
- **Should fast-follow (not this release):** xpub/watch-only accounts; PDF and PDF/UA briefs; benchmark attribution; German, French, and Spanish locales; PWA installability; archive, exports beyond the required export, local-currency display and trial, venue links, optional alerts, sub-account roll-up, DeFi labelling, watchlist, DEX pricing, non-core screens, usage UI.
- **Post-MVP:** direct trade execution and on-chain signing; DeFi/staking/LP accounting; NFT valuation; all-asset aggregation; native iOS/Android; multi-user and advisor portals; tax filing; KYC/AML; white-label; AI assistant; SOC 2 / ISO 27001.
- **Never:** custody or private keys; automatic trade execution; product-initiated transfers; fiat on/off-ramp; tax filing or regulated advice; futures or margin.

## Open Questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-1 | Which specific EVM chains are in launch coverage beyond Ethereum mainnet? | Product/Data |
| OQ-2 | What is Vietnam's regulatory posture for this product, and does it require a licence or gating? | Compliance |
| OQ-3 | Are Binance and OKX API terms acceptable for commercial use in each launch market? | Legal |
| OQ-4 | Which market-data and benchmark licences are approved, and what attribution do they require? | Legal/Data |
| OQ-5 | Is the two-year / six-month retention confirmed by counsel for every launch jurisdiction? | Compliance |
| OQ-6 | Is web push expected within this release after all? | Product |

## Sources

- `ideation/intent-capture/intent-statement.md` — problem, customers, success metrics, product boundary.
- `ideation/scope-definition/scope-document.md` — MVP boundary, Must/Should split, launch gates.
- `inception/practices-discovery/team-practices.md` — TDD, coverage, deployment, and code-style practices that shape how these requirements are built and tested.
- `docs/brds/BRD-001-product-vision-scope-and-success.md` — goals, users, scope, product requirements, metrics, risks.
- `docs/brds/BRD-012-security-compliance-and-non-functional-requirements.md` — security, privacy, legal, audit, and NFR targets.
