# Intent Backlog — Launch Domains Fragment (Identity, Account, Asset Registry, Market Data)

> **Fragment, not a canonical artifact.** Input to the Scope Definition synthesis; the
> canonical `intent-backlog.md` is assembled downstream from this and the sibling domain
> fragments (Portfolio/Ledger, Reporting/Billing, Job/Audit/Gateway/SPA). Proto-Units
> U-001 … U-019 only.

**Intent:** `261105-brd-001-product` · **Stage:** ideation/scope-definition · **Lead:** aidlc-product-agent
**Basis:** intent statement, confirmed feasibility answers Q6–Q8, scope answers Q1–Q3, feasibility assessment and constraint register (CON-001…CON-054), `docs/architecture/README.md`, and BRD-001/002/003/004/005/010/011/012 with `DECISIONS.md`.

## Fixed decisions applied (do not contradict)

- **Q6 Option B:** 10 launch services — Identity, Account, Asset Registry, Market Data, Portfolio, Ledger, Reporting, Billing, Job Orchestration, Audit — plus API Gateway and SPA; 5 engineers; 9-month target with a written cut list; full committed scope 10–11 months.
- **Q1:** MVP is the full Option B retail slice (aggregation, base-currency valuation, allocation/drift, ledger with cost basis and P&L, order intents, snapshots/briefs, billing).
- **Q2:** risk-first with a walking skeleton — market-data licensing, the Vault pattern, jurisdiction gating and the 99.9% topology are proven before feature breadth.
- **Q7:** EU (MiCA) + UK first; legal capped at ~USD 50k fixed fee, non-salary spend ≤ USD 150k; UK live only after counsel sign-off.
- **Q8:** Notification, Analytics and Support Console deferred to post-MVP with documented degraded paths; recorded as an explicit BRD deviation.
- **Coverage:** 5 exchanges (OKX, Binance, Coinbase, Kraken, Bybit), 10 chains (Ethereum, Base, Arbitrum, Optimism, Polygon, BNB, Avalanche, Solana, Bitcoin, Tron), WalletConnect + CSV committed; xpub/watch-only is a **Should fast-follow**.
- **Fixed constraints:** Rust/Axum/sqlx in Clean Architecture four-crate workspaces; React 18/TS SPA; Yarn ≥ 6; Supabase Postgres/Auth; one schema and one role per service with **no cross-schema grants** (the `platform` SECURITY DEFINER Vault accessor is the single recorded exception); no RLS for services; services read each other only via HTTP; no message queues; Docker Swarm; secrets in Supabase Vault.

## Unit summary

| Domain | Units | MVP effort (eng-weeks) | Deferred effort (eng-weeks) |
|---|---|---|---|
| Identity | U-001 … U-006 | 19 | 0 |
| Account | U-007 … U-012 | 34 | 2 (xpub fast-follow) |
| Asset Registry | U-013 … U-015 | 10 | 0 |
| Market Data | U-016 … U-019 | 16 | 0 |
| **Total** | **19** | **79** | **2** |

**81 engineer-weeks = 18.7 engineer-months** for these four domains (79 MVP + 2 fast-follow). This is scope for **four of the ten launch services**, leaving the sibling fragments to fit Portfolio, Ledger, Reporting, Billing, Job Orchestration, Audit, Gateway and SPA inside the remaining feasibility envelope (108–186 engineer-weeks total; 25–43 engineer-months). The gap against 5 engineers × 9 months (36 effective engineer-months) is the written cut list, not optimism.

**WSJF method:** `Cost of Delay (1–10) / Job Size (1–10) = WSJF`. Cost of delay weights user/business value, time criticality and risk reduction; job size uses the effort column and integration uncertainty. A one-line rationale accompanies each score.

---

## 1. Identity domain (U-001 … U-006)

Launch service: **Identity Service** (with the **API Gateway** for token enforcement and the SPA for provider flows).

| Unit ID | Name | Description | Launch service | BRD requirement IDs | MoSCoW | WSJF (CD / JS = score) | Dependencies | Acceptance criteria (testable) | Effort (eng-weeks) | Release |
|---|---|---|---|---|---|---|---|---|---|---|
| **U-001** | Supabase Auth integration and token trust | Browser PKCE sign-in, silent refresh and full sign-out against Supabase Auth; one shared Rust auth crate validating signature, `iss`, `aud`, `exp`/`nbf` with JWKS cache/rotation and algorithm pinning; SPA holds tokens in memory only; a distinct internal service-token audience for scheduled runs; provider-outage error surface. | Identity + API Gateway + SPA | FR-AUTH-01–04, 06–11, 21, 22; BR-AUTH-02, 03, 06, 07, 08 | Must | CD 10 / JS 4 = **2.5** — every authenticated path is blocked without token trust; well-understood platform crate, small build. | C3 Vault (internal service token); Supabase project/JWKS configuration | (1) Anonymous protected request redirects to sign-in; OAuth callback is PKCE-only and an implicit-flow callback is rejected. (2) Wrong issuer, wrong audience, expired `exp`, `alg:none` or unknown `kid` each return 401 at the gateway and at every service. (3) Expired access + valid refresh succeeds without re-sign-in; invalid refresh redirects; no access/refresh token is written to `localStorage`/`sessionStorage`. (4) A user JWT is rejected on internal run endpoints and a service token is rejected on user routes. | 4 | MVP |
| **U-002** | Internal user provisioning and profile | Lazy upsert of the internal user keyed by the Supabase `sub`; stable internal ID across claim rotation; profile (display name, email, locale, base currency, timezone) derived from the provider where possible; status lifecycle `active`/`disabled`/`pending_deletion`/`deleted`; disable without deleting history. | Identity | FR-USR-01, 02, 07, 09, 20; FR-AUTH-05; BR-USR-02, 11 | Must | CD 10 / JS 2 = **5.0** — nothing can be owned without the user record; provisioning is deliberately tiny. | U-001 | (1) First authenticated request creates exactly one internal user mapped to `sub`; a second request or claim rotation creates none. (2) Profile update of locale/base currency/timezone persists and leaves stored quantities and values unchanged. (3) A disabled user with a valid provider token is denied on the next request; only the four defined statuses are accepted. (4) Same email with a different subject does not merge identities. | 2 | MVP |
| **U-003** | Role projection, ownership scoping and administrative access | Project `app_metadata.role` into the principal; deny-by-default route policy; owner scope on every user-owned resource; non-disclosing 404/403; admin lookup, role change, disable and entitlement override with mandatory audit; cross-tenant test suite as a merge gate. | Identity + API Gateway | FR-USR-03, 04, 05, 06, 17, 21; FR-ADM-01, 14; BR-USR-01, 03, 04, 14; BR-ADM-01 | Must | CD 10 / JS 4 = **2.5** — a cross-tenant leak is the top-severity risk and admin gating is a launch gate; scoping every repository is real work. | U-001, U-002; Audit Service | (1) Non-admin calling any admin endpoint is denied; Support is denied every owner endpoint. (2) Every cross-user read/write returns 404 with no existence hint; an automated cross-tenant suite covers 100% of endpoints as a merge gate. (3) A role/entitlement change writes actor, target, before/after, reason and timestamp to audit and takes effect on the next request. (4) The repository type system rejects a user-owned query that has no owner scope (compile-time or startup test). | 4 | MVP |
| **U-004** | MFA, device/session control and account recovery | MFA enrolment and enforcement before any exchange credential; verified-email prerequisite; device/session list with last-seen and individual/all revocation; revocation after password/MFA/passkey change; new-device alert; recovery paths through the provider without secret disclosure. | Identity + SPA | FR-AUTH-12, 15, 16, 17, 18, 22; FR-USR-13; BR-AUTH-09, 10, 11 | Must | CD 8 / JS 3 = **2.7** — MFA gates exchange credentials (a Must) but the provider does most of the work. | U-001, U-002; U-008 (credential-gate integration); Reporting degraded alert path | (1) A user without MFA attempting to add an exchange credential is blocked and prompted; after enrolment the save succeeds. (2) An unverified email blocks credential add. (3) Device/session list shows last-seen; revoking one device ends its session on the next request and revoking all ends all. (4) Password/MFA/passkey change revokes other sessions; a new-device sign-in emits an alert containing no token or full fingerprint. (5) Recovery completes without revealing whether an address exists. | 3 | MVP |
| **U-005** | Support access and break-glass controls | Support role with read-only, time-bound, reason-recorded access to account metadata only — never credentials or portfolio contents; automatic expiry; every grant, read and expiry audited; a break-glass read-only admin view substitutes for the deferred Support Console (Q8). | Identity (+ Audit) | FR-USR-11, 19; FR-ADM-15; BR-USR-09, 10; BR-ADM-10 | Must | CD 5 / JS 2 = **2.5** — needed for the Q8 support degraded path and audit, not for core valuation. | U-003; Audit Service | (1) Support opens a record only with a reason and TTL; the grant expires automatically and grant/expiry both appear in audit. (2) Support responses and exports contain no credential field and no portfolio value (contract test enumerates fields). (3) Permanent support grants cannot be created; an expired grant is denied. (4) Every support session is queryable by user, actor and time for compliance review. | 2 | MVP |
| **U-006** | Consent, jurisdiction/age gating and privacy lifecycle | Versioned terms/risk/age acceptance before the first source; fail-closed jurisdiction allowlist enforced at the edge **and** in a Supabase before-user-created hook before any personal data; user preferences; self-service export; deletion cascade/anonymisation with legal hold and credential crypto-shredding; retention job hooks. | Identity + edge/WAF + SPA | FR-USR-08, 12, 14, 15, 16, 18; FR-AUTH-19, 20; FR-JOB-23; BR-USR-12, 13; BR-AUTH-12 | Must | CD 10 / JS 4 = **2.5** — collecting data without the jurisdiction/age gate is launch-blocking; gate, consent and erasure are bounded work. | U-001, U-002; edge/WAF; C8 DPIA and transfer pack | (1) Blocked-country and under-age sign-up are refused before user creation; the hook test shows no auth user, email or IP persisted beyond the gate's minimal log. (2) Terms/risk/age acceptance is versioned and timestamped before the first source; a version bump re-prompts. (3) Export returns a complete machine-readable archive within the stated SLA. (4) Deletion follows the per-entity cascade, preserves legal-hold records and renders credentials unrecoverable; a dry-run report lists survivors and their legal basis. (5) Changing locale/base currency never rewrites historical values. | 4 | MVP |

---

## 2. Account domain (U-007 … U-012)

Launch service: **Account Service** (adapters behind `ExchangeReader`, `AccountSource`, `ChainRpcProvider` ports; credentials via the Vault platform-schema exception).

| Unit ID | Name | Description | Launch service | BRD requirement IDs | MoSCoW | WSJF (CD / JS = score) | Dependencies | Acceptance criteria (testable) | Effort (eng-weeks) | Release |
|---|---|---|---|---|---|---|---|---|---|---|
| **U-007** | Account lifecycle, regional coverage and quotas | Create/edit/activate/deactivate/delete across the five account types with type-specific validation; region-aware venue availability with an explanation; server-side plan quotas; sub-account parent roll-up (Should); account summary (holdings count, last sync, last outcome). | Account | FR-ACC-01, 03, 06, 08, 09, 17, 28, 29, 30; FR-JOB-21; FR-ACC-27 (Should) | Must | CD 8 / JS 4 = **2.0** — accounts are the entry point for all holdings; CRUD, coverage and quotas are routine. | U-003; U-014 (enabled chains); Billing entitlements | (1) Exchange create requires name + complete credentials; on-chain create requires an address and ≥1 active chain; validation errors name the field. (2) An inactive account is excluded from sync and aggregation; delete removes portfolio links only and leaves portfolios intact. (3) A venue unavailable in the user's region is absent from the catalogue response with a reason code, not a generic error. (4) A Free-plan user exceeding the account quota is blocked server-side before persistence. (5) A sub-account (Should) rolls up to exactly one parent with no double counting. | 4 | MVP |
| **U-008** | Exchange onboarding and read-only key verification | Five venue adapters (OKX, Binance, Coinbase, Kraken, Bybit) behind an `ExchangeReader` port; credentials encrypted via Vault and stored only as references; permission probe at save and on a schedule charged to the same budget; reject trading/withdrawal keys; detect revoked or permission-changed keys and deactivate with a mandatory alert; per-venue circuit breaker and shared per-venue/per-IP token bucket. | Account | FR-ACC-01, 04, 05, 07, 18, 19, 29; FR-JOB-22; BR-ACC-09, 11, 12; NFR-SEC-05/06 | Must | CD 9 / JS 10 = **0.9** — highest user value and a compliance gate, but five venue auth and permission paths are the least predictable build. | U-007; U-004 (MFA/verified email); C3 Vault pattern; C14 pen test before any production key | (1) Saving a valid read-only key stores only a Vault reference; no response, log, trace or error contains the secret. (2) A key with trade or withdraw permission is rejected before persistence with actionable guidance. (3) The scheduled probe detects a revoked or permission-changed key, marks the account inactive and raises a mandatory alert. (4) Credential edit re-encrypts, records the change and never echoes the secret. (5) Adapter contract tests pass for all five venues against recorded fixtures; 429/418 triggers backoff and no retry storm. | 10 | MVP |
| **U-009** | Holdings sync orchestration and normalisation | Single and bulk sync with per-account results; idempotent holdings replacement; decimal normalisation and zero filtering; spam/dust hiding and exclusion from valuation; last-sync/last-outcome surfacing; labelled partial-sync state; plan cadence; one cluster-wide per-venue/per-IP budget shared by all callers. | Account | FR-ACC-10, 11, 12, 13, 14, 15, 16, 25, 30; FR-JOB-24 (holdings side); BR-ACC-03, 05, 08, 13 | Must | CD 9 / JS 5 = **1.8** — makes every account useful; one sync pipeline serves every source type. | U-008, U-010, U-011, U-012, U-014 | (1) A successful sync replaces holdings with normalised decimals, filters zero balances and updates last-synced only on success. (2) One account failure is recorded with a reason and does not stop the bulk run; per-account results are returned. (3) Re-running the same sync at the same point creates no duplicate holdings. (4) Spam/dust tokens are hidden and excluded from valuation but retained in the source of truth. (5) A throttled venue produces a labelled partial-sync gap, never a silently wrong total. | 5 | MVP |
| **U-010** | On-chain address accounts across ten chains | Address validation and balance readers for seven EVM chains, Solana/SPL, Bitcoin native and Tron/TRC-20 including USDT; decimals applied from the registry; RPC primary/fallback with source recorded; DeFi/staking/LP position labelling (Should). | Account | FR-ACC-02, 23, 24, 26 (Should); BR-ACC-04, 10, 14 | Must | CD 8 / JS 10 = **0.8** — ten chains are required coverage and the largest single engineering block; only bounded chain families. | U-007; U-014; RPC/indexer providers and CON-053 terms | (1) Malformed addresses for each chain family are rejected before any network call. (2) Sync returns native + token balances for all ten chains; USDT on Tron is read as TRC-20. (3) The same asset on two chains appears once with the combined quantity; unknown decimals is a sync error, never a guess. (4) A primary RPC failure falls back and records the source used. (5) Unsupported or unpriced DeFi positions are labelled, never valued as liquid tokens (Should). | 10 | MVP |
| **U-011** | WalletConnect read-only wallet connection | Browser WalletConnect session that adds a read-only account (public address/session) with no key material; session scope restricted to read methods; disconnect or expiry marks the account inactive; re-connect restores. | Account + SPA | FR-ACC-20; CON-005 browser-to-external exception | Must | CD 6 / JS 3 = **2.0** — a committed Must path, but only a subset of users; bounded SDK integration. | U-007 | (1) A WalletConnect session adds an account with its address and no stored secret. (2) The session requests no signing or transaction method; a signing request is refused by construction (scope test). (3) Disconnect or expiry marks the account inactive and stops sync; re-connect restores without duplication. | 3 | MVP |
| **U-012** | CSV import and watch-only fast-follow | CSV holdings/transaction import with a template, column-mapping preview and idempotent commit; ambiguous rows quarantined with a reason; reconciliation against synced holdings (BRD-013). **xpub/watch-only (Should) is the fast-follow** and adds native Bitcoin watch-only accounts. | Account | FR-ACC-22, 21 (Should); BR-ACC-18 | Must (CSV) / Should (xpub) | CD 6 / JS 4 = **1.5** — CSV rescues users on unsupported venues; xpub is a cheap fast-follow but not committed. | U-007, U-013, U-014; Ledger (transaction reconciliation) | (1) Preview shows detected columns, sample rows and target fields before commit. (2) Re-importing the same file creates no duplicates; each batch links to its source-file hash. (3) Unmapped or ambiguous assets are quarantined with a reason, never silently dropped. (4) xpub (deferred): a valid xpub adds a watch-only account syncing native Bitcoin with no private key. | 4 (2 CSV + 2 xpub) | MVP (CSV); xpub **deferred** fast-follow |

---

## 3. Asset Registry domain (U-013 … U-015)

Launch service: **Asset Registry Service** (`registry` schema; admin mutations via the gateway and Identity role checks).

| Unit ID | Name | Description | Launch service | BRD requirement IDs | MoSCoW | WSJF (CD / JS = score) | Dependencies | Acceptance criteria (testable) | Effort (eng-weeks) | Release |
|---|---|---|---|---|---|---|---|---|---|---|
| **U-013** | Canonical asset catalogue | Root asset identity — unique symbol, name, type, rank, aliases — shared read contract over HTTP; admin-only mutation; deactivation retains history; consumers store IDs plus cached display fields and never join registry tables. | Asset Registry | FR-AST-01, 02; BR-AST-01, 08 | Must | CD 9 / JS 3 = **3.0** — every valuation, price and holding depends on canonical identity; the model itself is small. | U-003 (admin authorisation) | (1) A new asset requires a unique symbol; a duplicate symbol or alias collision is rejected. (2) An alias resolves to exactly one canonical asset in registry and market lookups. (3) Consumers resolve identity through a batch HTTP endpoint; a cross-schema read of `registry` tables fails the boundary test. | 3 | MVP |
| **U-014** | Chain and token contract registry with decimals | One representation per asset per chain — contract/mint/address or native symbol, fixed decimals, native/spam flags; duplicate rejection; resolution of synced tokens; quarantine for unmapped tokens; decimals immutable once used in a valuation. | Asset Registry | FR-AST-03, 04, 05, 06; FR-ADM-04, 05, 06; BR-AST-02, 03, 04, 06 | Must | CD 9 / JS 4 = **2.3** — wrong contracts or decimals corrupt every balance; registry shapes are well bounded. | U-013 | (1) Registering a token requires chain, identifier, decimals and flags; a duplicate contract on the same chain is rejected with a clear error. (2) A registered token resolves to its asset and decimals and normalises a raw balance exactly. (3) An unregistered token is quarantined, visible, excluded from valuation and never guessed. (4) Solana mint, Bitcoin address format and Tron TRC-20 identifiers are each unique per chain. (5) Changing decimals after a valuation requires explicit review and triggers a re-normalisation run. | 4 | MVP |
| **U-015** | Registry administration and catalogue sync | Admin CRUD for chains and tokens with review-before-activation; chain-ID uniqueness; deactivation warning for active accounts; changes effective on the next sync without deploy; canonical token-list import staged and explicitly selected; daily catalogue refresh with run records; audited admin refresh trigger. | Asset Registry | FR-ADM-02, 03, 04, 05, 06, 07, 10, 11, 12; FR-AST-11 (Should); FR-JOB-01, 08 (trigger) | Must | CD 7 / JS 3 = **2.3** — operators need it before coverage can be corrected; thin CRUD over U-013/U-014. | U-013, U-014, U-003 | (1) A non-admin registry mutation is denied; every create/update/deactivate writes actor, timestamp and before/after. (2) A duplicate chain ID is rejected; deactivating a chain with active accounts warns and requires confirmation. (3) A registry change applies on the next on-chain sync with no deployment; historical snapshots are unchanged. (4) Import from a canonical list stages results and activates only on explicit selection; no bulk activation. (5) The daily catalogue refresh leaves a run record with updated/skipped counts. | 3 | MVP |

---

## 4. Market Data domain (U-016 … U-019)

Launch service: **Market Data Service** (`market` schema; providers behind `MarketDataProvider` and `FxProvider` ports).

| Unit ID | Name | Description | Launch service | BRD requirement IDs | MoSCoW | WSJF (CD / JS = score) | Dependencies | Acceptance criteria (testable) | Effort (eng-weeks) | Release |
|---|---|---|---|---|---|---|---|---|---|---|
| **U-016** | Multi-source price ingestion and freshness tiers | Provider port with deterministic per-asset and per-region priority/fallback; 1-minute top-asset and 15-minute tail collection; last-stored-price fallback labelled with age; persisted observation history; per-asset failure isolation; regional pair availability. | Market Data | FR-AST-07, 08, 12, 13, 15, 18, 20 (Should), 23 (Should); FR-JOB-02, 10; BR-AST-10, 11 | Must | CD 10 / JS 7 = **1.4** — freshness is a headline success metric and licensing proof; multi-source ingestion is substantial. | U-013; U-019; C1 written market-data rights | (1) Collection meets NFR-JOB-01/02 budgets; the run record counts succeeded/failed per asset. (2) Primary failure falls back to a secondary and records the source; both fail → last stored price labelled with age; no source → unpriced. (3) One asset's failure does not stop the run. (4) Observations store timestamp, source and confidence and remain queryable as history. (5) A region without the pair returns an explicit unavailable state, never a fabricated price. | 7 | MVP |
| **U-017** | FX rate collection and historical conversion | Hourly and daily-close FX for every supported base currency; source and timestamp on each observation; historical lookup by transaction/snapshot date; admin-configurable currencies and source priority; explicit degradation on source failure. | Market Data | FR-AST-16, 17; FR-JOB-17; BR-AST-12; FR-ADM-17 | Must | CD 8 / JS 3 = **2.7** — base-currency valuation is a Must for global retail; a single FX series is compact. | U-019 (source configuration) | (1) Every supported currency stores hourly and daily-close rates with source and timestamp. (2) Historical lookup returns the rate applicable to the transaction/snapshot date, or an explicit missing-rate state. (3) Source failure uses the last-known rate with its age; the system never substitutes 1.0 silently. (4) Admin currency/source changes apply on the next run without deploy. | 3 | MVP |
| **U-018** | Market data quality, provenance and unpriced handling | Provenance contract exposed to Portfolio/Reporting (source, observation time, confidence, staleness tier); stale and unpriced labels; cross-source divergence alert; stablecoin de-peg alert (Should); user price-alert thresholds and watchlist (Should). | Market Data | FR-AST-09, 10, 14, 19 (Should), 24 (Should), 26 (Should); BR-AST-05, 06, 07, 13, 15 | Must | CD 9 / JS 3 = **3.0** — trust depends on provenance and staleness labels; contracts over observations already built. | U-016, U-017 | (1) Every valuation/P&L response carries price source, observation timestamp, confidence and staleness state. (2) A price older than its tier threshold is labelled stale and never presented as current. (3) An unpriced asset is excluded from the priced total with a reason. (4) Cross-source divergence beyond threshold raises an alert preserving both observations. (5) A de-peg alert fires beyond threshold as a factual notice (Should). (6) A watchlist returns price/change without affecting portfolio valuation (Should). | 3 | MVP |
| **U-019** | Source licensing, attribution and market-data source administration | Licence register per source covering display, storage, history, derivation, redistribution and region; fail-closed activation gate; attribution payload rendered wherever licensed data appears; source priority, supported assets/regions, health and last-success; audited manual override (Could) excluded from benchmarks and history. | Market Data | FR-AST-22, 21 (Could), 25 (Should); FR-ADM-16, 17; BR-AST-14; BR-ADM-13 | Must | CD 10 / JS 3 = **3.3** — no licensed source means no launch, and attribution is cheap once the register exists. | U-003, U-013; C1 written rights; counsel licence review | (1) A source without a licence reference cannot be activated; activation fails closed. (2) Attribution for each displayed source appears wherever its data is shown; a contract test asserts the attribution payload. (3) Source priority and supported assets/regions are configurable and effective on the next run without deploy. (4) Source health exposes last success and outage state to admins and alerts when unhealthy. (5) A manual override is audited, visually distinct and excluded from benchmark/history series (Could). | 3 | MVP |

---

## 5. Must-requirement traceability

Every **Must** functional requirement in BRD-002, 003, 004, 005, 010 and 011 appears below exactly once, either mapped to a unit in this fragment, mapped to a sibling domain, or explicitly deferred with a reason. No Must requirement is orphaned.

### 5.1 BRD-002 — Users, Roles & Permissions (20 Must FRs)

| Must requirement IDs | Unit | Note |
|---|---|---|
| FR-USR-01, FR-USR-02, FR-USR-07, FR-USR-09, FR-USR-20 | U-002 | Provisioning, profile, status lifecycle |
| FR-USR-03, FR-USR-04, FR-USR-05, FR-USR-06, FR-USR-17, FR-USR-21 | U-003 | Ownership scoping, admin gating, audit |
| FR-USR-08, FR-USR-12, FR-USR-14, FR-USR-15, FR-USR-16, FR-USR-18 | U-006 | Deletion cascade, preferences, terms/age, jurisdiction, export |
| FR-USR-11, FR-USR-19 | U-005 | Time-bound support access; credential invisibility |
| FR-USR-13 | U-004 | Provider-led account recovery |
| FR-USR-10 | — | **Won't (MVP)** in the BRD; post-MVP shared portfolios; no unit |

### 5.2 BRD-003 — Authentication & Session Management (19 Must FRs)

| Must requirement IDs | Unit | Note |
|---|---|---|
| FR-AUTH-01, FR-AUTH-02, FR-AUTH-03, FR-AUTH-04, FR-AUTH-06, FR-AUTH-07, FR-AUTH-08, FR-AUTH-09, FR-AUTH-10, FR-AUTH-11, FR-AUTH-21, FR-AUTH-22 | U-001 | Sign-in/refresh/sign-out, JWT validation, in-memory tokens, revocation |
| FR-AUTH-05 | U-002 | Subject → internal user |
| FR-AUTH-12, FR-AUTH-15, FR-AUTH-16, FR-AUTH-17, FR-AUTH-18 | U-004 | MFA, devices, email verification, recovery, alerts |
| FR-AUTH-19, FR-AUTH-20 | U-006 | Terms/risk/age and jurisdiction gates |
| FR-AUTH-13, FR-AUTH-14 (Should) | U-001 / U-004 | Google/Apple OAuth, passkeys — Should, inside Must units |

### 5.3 BRD-004 — Account Management (27 Must FRs)

| Must requirement IDs | Unit | Note |
|---|---|---|
| FR-ACC-01 | U-007 / U-008 | Account shell in U-007; exchange credential path in U-008 |
| FR-ACC-02, FR-ACC-23, FR-ACC-24 | U-010 | On-chain EVM/Solana/Bitcoin/Tron |
| FR-ACC-03, FR-ACC-06, FR-ACC-08, FR-ACC-09, FR-ACC-17, FR-ACC-28 | U-007 | Validation, edit, deactivate, delete, regional coverage, quotas |
| FR-ACC-04, FR-ACC-05, FR-ACC-07, FR-ACC-18, FR-ACC-19 | U-008 | Vault encryption, secret-free responses, re-encryption, permission probes, revocation |
| FR-ACC-10, FR-ACC-11, FR-ACC-12, FR-ACC-13, FR-ACC-14, FR-ACC-15, FR-ACC-16, FR-ACC-25, FR-ACC-30 | U-009 | Sync, normalisation, spam/dust, outcomes, cadence |
| FR-ACC-20 | U-011 | WalletConnect |
| FR-ACC-22 | U-012 | CSV import |
| FR-ACC-29 | U-007 / U-008 / U-019 | Regional availability and exchange/data licence enforcement |
| FR-ACC-21 (Should) | U-012 (fast-follow) | xpub/watch-only; **deferred** by Q1/Q6 coverage decision, not a Must |
| FR-ACC-26 (Should) | U-010 | DeFi/staking/LP labelling |
| FR-ACC-27 (Should) | U-007 | Sub-account roll-up |

### 5.4 BRD-005 — Asset Catalog & Market Data (18 Must FRs)

| Must requirement IDs | Unit | Note |
|---|---|---|
| FR-AST-01, FR-AST-02 | U-013 | Canonical asset and unique symbol |
| FR-AST-03, FR-AST-04, FR-AST-05, FR-AST-06 | U-014 | Contracts, decimals, resolution, unmapped |
| FR-AST-07, FR-AST-08, FR-AST-12, FR-AST-15, FR-AST-18 | U-016 | Multi-source, fallback, history, deterministic priority, regional pairs |
| FR-AST-09, FR-AST-10, FR-AST-14 | U-018 | Provenance, display of source/time, unpriced exclusion |
| FR-AST-13 | U-019 / U-016 | Source configuration lives in U-019, execution in U-016 |
| FR-AST-16, FR-AST-17 | U-017 | FX current and historical |
| FR-AST-22 | U-019 | Attribution and licensing |
| FR-AST-11 (Should), FR-AST-19 (Should), FR-AST-20 (Should), FR-AST-23 (Should), FR-AST-24 (Should), FR-AST-25 (Should), FR-AST-26 (Should) | U-015, U-018, U-016, U-019 | Catalogue refresh, de-peg, long-tail, tiers, alert thresholds, source health, watchlist |
| FR-AST-21 (Could) | U-019 | Audited manual override; never a market price |

### 5.5 BRD-010 — Admin Configuration (16 Must FRs)

| Must requirement IDs | Unit | Note |
|---|---|---|
| FR-ADM-01 | U-003 | Admin-only access |
| FR-ADM-02, FR-ADM-03, FR-ADM-12 | U-015 | Chain CRUD, validation, deactivation warning |
| FR-ADM-04, FR-ADM-05, FR-ADM-06 | U-014 / U-015 | Token CRUD and duplicate rejection across EVM, Solana, Bitcoin, Tron |
| FR-ADM-07, FR-ADM-10, FR-ADM-11 | U-015 | Review-before-activation, no-deploy effect, audit |
| FR-ADM-14 | U-003 | User lookup, role/disable/entitlement override with audit |
| FR-ADM-15 | U-005 | Support read-only time-bound access |
| FR-ADM-16 | U-019 | Market-data source, priority and licence configuration |
| FR-ADM-17 | U-017 / U-019 | FX source and currency support |
| FR-ADM-18 | **DEFERRED** | Feature flags belong to the Analytics Service, deferred by **Q8**. Launch substitutes a deploy-time flag file plus an admin-editable gateway allowlist. Recorded as an explicit BRD deviation: post-MVP Analytics restores managed flags with owner and expiry. |
| FR-ADM-19 | Sibling — Billing | Plan quotas and entitlement defaults are Billing-domain state (BRD-016); the gateway enforcement hook is referenced from U-007. |
| FR-ADM-08 (Should), FR-ADM-09 (Should), FR-ADM-20 (Should), FR-ADM-21 (Should), FR-ADM-22 (Should) | U-015 / U-019 | Token-list import, mapping sync, health dashboards, spam lists, audit export |
| FR-ADM-13 (Could) | U-015 | Documented admin API |

### 5.6 BRD-011 — Background Jobs (22 Must FRs)

| Must requirement IDs | Unit / domain | Note |
|---|---|---|
| FR-JOB-01 | U-015 | Daily catalogue refresh |
| FR-JOB-02 | U-016 | Price collection interval |
| FR-JOB-03, FR-JOB-11, FR-JOB-20 | Sibling — Portfolio | EOD snapshots, per-portfolio isolation, per-timezone local midnight |
| FR-JOB-04, FR-JOB-05, FR-JOB-09, FR-JOB-12, FR-JOB-13 | Sibling — Job Orchestration | Run records, retry idempotency, single-flight, alerts, status |
| FR-JOB-06 | U-001 / Job | Distinct internal service-token audience for non-interactive runs |
| FR-JOB-08 | U-015 / U-019 / Job | Admin-authorised, audited manual trigger endpoints; scheduling auth in Job |
| FR-JOB-10 | U-016 | Per-asset price failure isolation |
| FR-JOB-16 | Sibling — Ledger | Transaction sync |
| FR-JOB-17 | U-017 | FX collection and storage |
| FR-JOB-18 | Sibling — Reporting (degraded) | Notification Service deferred by **Q8**; Reporting owns mandatory email + in-app alerts with retry, dedupe and delivery log |
| FR-JOB-19 | Sibling — Billing | Renewal, dunning, entitlement reconciliation |
| FR-JOB-21 | U-007 / U-009 + Billing | Plan cadence and quotas enforced on sync |
| FR-JOB-22 | U-008 | Stale/revoked key detection during sync |
| FR-JOB-23 | U-006 (+ Audit) | Retention and deletion of identity data and credentials |
| FR-JOB-24 | U-009 (holdings side) + Sibling — Ledger/Portfolio | Reconciliation of holdings against ledger and valuation |
| FR-JOB-25 | **DEFERRED** | Consent-aware analytics aggregation belongs to the Analytics Service, deferred by **Q8**. Launch uses a PII-free, consent-checked event path in the web tier with no aggregation service; recorded as the approved Q8 BRD deviation. |
| FR-JOB-07 (Should), FR-JOB-14 (Should), FR-JOB-15 (Should), FR-JOB-26 (Should) | Sibling — Job + U-015/U-019 | Manual triggers surfaced in admin UI, configurable schedules, bounded backfill, source-health alerts |

### 5.7 Cross-domain Must requirements (owned by sibling fragments)

These are Must requirements from the six BRDs read for this fragment that are **not** owned by Identity, Account, Asset Registry or Market Data. They are listed so synthesis can detect a missing sibling unit rather than silently orphaning them.

| Source | Must requirement | Owning domain |
|---|---|---|
| BRD-010 | FR-ADM-19 — plan quotas and entitlement defaults | Billing |
| BRD-011 | FR-JOB-03, 11, 20 — EOD snapshots, per-portfolio isolation, local-midnight scheduling | Portfolio |
| BRD-011 | FR-JOB-04, 05, 09, 12, 13 — run records, idempotent retry, single-flight, alerts, status | Job Orchestration |
| BRD-011 | FR-JOB-16 — transaction sync | Ledger |
| BRD-011 | FR-JOB-18 — notification dispatch (degraded path) | Reporting (Notification deferred, Q8) |
| BRD-011 | FR-JOB-19 — billing renewal, dunning, entitlement reconciliation | Billing |

### 5.8 Explicit deferrals and deviations

| Item | Reason | Degraded path at launch | Recorded where |
|---|---|---|---|
| FR-ACC-21 xpub/watch-only | Should in BRD-004; Q1/Q6 coverage keeps it as a fast-follow (feasibility: 2–4 eng-weeks sensitivity, not in committed total) | Public-address watch-only is not offered; WalletConnect and CSV cover the committed import paths | This fragment, U-012; must be stated in `scope-document.md` |
| FR-ADM-18 feature flags | Analytics Service deferred by Q8 | Deploy-time flag file plus gateway allowlist; no self-serve rollout percentages | This fragment; explicit BRD deviation |
| FR-JOB-25 analytics aggregation | Analytics Service deferred by Q8 | Consent-checked, PII-free web-tier event path; no aggregation service | This fragment; explicit BRD deviation |
| Notification Service (BRD-011 FR-JOB-18, BRD-015) | Q8 | Reporting owns mandatory email + in-app alerts with retry/dedupe/log | Cross-domain note above |
| Support Console (Q8) | Q8 | U-005 break-glass, time-bound, reason-recorded read-only admin view behind the gateway | U-005 |

---

## 6. Risk-first sequencing (Q2)

The order below front-loads the four items Q2 names — market-data licensing, the Vault pattern, jurisdiction gating and the 99.9% topology — before feature breadth. It is a dependency-safe sequence, not a date plan.

| Wave | Units | Why this order |
|---|---|---|
| **W0 — Walking skeleton, synthetic only** | U-001, U-002, U-003, U-013, U-014; C3 Vault spike inside U-008 bootstrap; U-019 licence register scaffold; U-006 gate design | Identity and canonical identity are the two roots every other service calls. No real user, key or licensed ingestion until C3/C8/C1 evidence exists (feasibility M0). |
| **W1 — Launch gates and registry** | U-004, U-005, U-006, U-015, U-019 | Jurisdiction/age gate must precede the first real user (C8); licence activation gate must precede production ingestion (C1). |
| **W2 — Sources and prices** | U-007, U-008, U-009, U-016, U-017, U-018 | Five read-only venue adapters plus multi-source prices are the critical path. Production exchange keys stay blocked until C14 passes. |
| **W3 — Chain breadth and imports** | U-010, U-011, U-012 | Ten-chain coverage, WalletConnect and CSV round out the committed source matrix. xpub is the explicit fast-follow. |

**Cut-list candidates if capacity slips** (in order): xpub/watch-only (U-012 tail, 2 wks) → FR-AST-26 watchlist / FR-AST-24 thresholds (inside U-018) → sub-accounts (U-007 Should) → DeFi position labelling (U-010 Should) → long-tail DEX pricing (U-016 Should). The Must floor — U-001…U-009, U-013…U-017, U-019 at Must scope — is not cuttable without a BRD deviation.

---

## 7. Verification notes

- **Counts:** 19 units (U-001…U-019, no gaps); 18 fully MVP, 1 MVP-with-deferred-tail (U-012); 0 units outside the assigned range.
- **Effort:** 81 engineer-weeks total = 79 MVP + 2 deferred. This is 18.7 engineer-months, roughly 43% of the 43 engineer-month upper envelope and about 52% of the 36 effective engineer-months available at 5 × 9 months; the remaining services must fit the residual, which is why the cut list above exists.
- **Coverage check against the assigned capability list:** Supabase Auth + JWT (U-001) · roles/permissions + support access (U-003, U-005) · exchange onboarding + read-only key verification (U-008) · on-chain address accounts (U-010) · WalletConnect (U-011) · CSV import + xpub fast-follow (U-012) · canonical asset registry (U-013) · chain/token contracts + decimals (U-014) · multi-source price ingestion + freshness tiers (U-016) · FX (U-017) · licensing/attribution (U-019) · admin registry (U-015, U-019, U-003).
- **Consistency with the constraint register:** U-006 implements CON-021/CON-043; U-008 implements CON-012/CON-026/CON-033/CON-034; U-009 implements CON-007/CON-019; U-013/U-014 implement CON-006; U-016/U-019 implement CON-025/CON-053; U-017 implements CON-040; U-003/U-005 implement CON-018/CON-054. No unit assumes a cross-schema read, an RLS policy or a message queue.
