# Intent Backlog

**Intent:** `261105-brd-001-product` · **Stage:** `ideation/scope-definition` · **Status:** proposed for approval

**Basis:** intent statement, confirmed feasibility answers Q6–Q8, scope answers Q1–Q3, `../feasibility/feasibility-assessment.md`, `../feasibility/constraint-register.md` (CON-001…CON-054), `docs/architecture/README.md`, and BRD-001…BRD-021 with `docs/brds/DECISIONS.md`.

**Decisions applied (not re-opened):** Q6 Option B — ten launch services (Identity, Account, Asset Registry, Market Data, Portfolio, Ledger, Reporting, Billing, Job Orchestration, Audit) plus API Gateway and SPA, five engineers, a 9-month target with a written cut list, and 10–11 months for full committed scope; Q7 EU (MiCA) + UK first, legal ≈USD 50k fixed, non-salary ≤USD 150k, UK live after counsel sign-off; Q8 Notification, Analytics and Support Console deferred with documented degraded paths; Q1 full Option B retail slice; Q2 risk-first walking skeleton; Q3 no external hard date. Launch coverage: five exchanges, ten chains, WalletConnect + CSV committed, xpub/watch-only **Should fast-follow**. Constraints: Rust/Axum/sqlx Clean Architecture four-crate workspaces; React 18/TS 5; Yarn ≥ 6; Supabase Postgres/Auth; one schema and role per service with **no cross-schema grants** (the `platform` `SECURITY DEFINER` Vault accessor is the only recorded exception); no RLS; HTTP-only service reads; no message queues; Docker Swarm; secrets in Supabase Vault; read-only exchange credentials; no custody, automatic trading, fiat ramp, tax filing or advice.

## 1. Method, release taxonomy and effort reconciliation

**WSJF method.** `Cost of Delay (1–10) / Job Size (1–10) = score`. Cost of delay weights user/business value, time criticality and risk reduction; job size uses committed effort plus integration uncertainty. Each unit carries a one-line rationale. WSJF is informational: Q2's risk-first order overrides it where a gate or architecture risk is involved, and each override names its reason.

**Release taxonomy (these categories are not interchangeable).**

| Category | Meaning | Counted in the committed 186.0 ew? |
|---|---|---|
| **MVP** | Committed launch scope, built before launch. | Yes |
| **Should fast-follow** | Named, estimated work built immediately after launch; a workaround exists at launch. Not "not built", and not part of the committed total. | No |
| **Post-MVP** | Explicitly not built in this plan; no launch workaround promised beyond the named degraded path. | No |
| **MVP degraded path** | A Q8 deferred service's Musts delivered through a narrower launch capability. | Yes |

**Fragment merge and renumbering.** The three source fragments are merged into one register. Every unit ID is globally sequential **U-001…U-058 with no gaps**. Duplicate capability sets are merged into single owners:

| Merged unit | Absorbs | Duplicate capability removed |
|---|---|---|
| **U-001** | old U-001 + U-040 + U-043 | JWT validation/session existed in the shared crate, the gateway and the SPA shell; one unit now owns crate + gateway request contract + SPA session/shell. |
| **U-005** | old U-005 + U-058 | Break-glass support access existed as an Identity policy unit and again as the Q8 support degraded path; one unit owns policy + surface. |
| **U-006** | old U-006 + U-045 + U-051 | Jurisdiction/age gating and legal acceptance appeared in consent, onboarding and the gate unit; one unit owns the gate, consent and first-run journey. |

All later IDs are renumbered sequentially; §5, §7, §8 and §9 reference only the new IDs. Old→new mapping for the affected ranges: old U-040/U-043 → U-001; old U-041 → U-041, old U-042 → U-042, old U-044 → U-044 (unchanged, not merged); old U-045/U-051 → U-006; old U-046 → U-043; old U-047 → U-044; old U-048 → U-045; old U-049 → U-046; old U-050 → U-047; old U-052 → U-048; old U-053 → U-049; old U-054 → U-050; old U-055 → U-051; old U-056 → U-052; old U-057 → U-053; old U-058 → U-005; old U-059 → U-054; old U-060 → U-055; old U-061 → U-056; old U-062 → U-057; old U-063 → U-058. Units old U-001–U-039 keep their numbers.

### 1.1 Effort reconciliation (every line is a row sum)

| Reconciliation line | Engineer-weeks |
|---|---|
| Fragment 1 — launch domains, 19 units (includes 2.0 xpub Should) | 81.0 |
| Fragment 2 — Portfolio/Ledger/Reporting/Billing, 20 units | 47.5 |
| Fragment 3 — Gateway/Jobs/Audit/SPA/gate/privacy/degraded paths, 20 units | 74.0 |
| Platform and DevSecOps workstream added (U-055–U-058) | 17.0 |
| **Raw total** | **219.5** |
| Duplicate-unit merges: U-001 set 10.0→7.0 (−3.0); U-005 set 3.0→2.5 (−0.5); U-006 set 9.5→7.0 (−2.5) | −6.0 |
| Other single-owner overlap removal: brief/alert delivery U-036/U-052 1.5; plan catalogue U-038/U-040 2.0; gateway route table U-041/U-001 1.0; schedule-only vs execution U-045/U-046 3.0; SPA surface U-042/U-001 1.5; run-endpoint overlap U-044/U-016–U-017 1.0; redaction U-047/U-054 1.0; audit-event overlap U-003/U-047 1.0; holdings contract U-007/U-024 1.0 | −13.0 |
| **Cleaned Must-scope total** | **200.5** |
| Should fast-follow slices reclassified out (16 slices, exact list in §5.2) | −14.5 |
| **Committed MVP Must total** | **186.0** |

The previous artifact's −15.5 line was wrong: its 16 itemised slices sum to 14.5, so this table uses 14.5 consistently. The previous 192.0 total was also based on that mislabelled line; the same itemisation plus the three duplicate merges now lands on 186.0 with no hidden adjustment.

### 1.2 Capacity and the plan of record

| Figure | Derivation | Engineer-weeks |
|---|---|---|
| 9-month capacity | 5 engineers × 9 months × 80% = 36.0 engineer-months × 4.333 | **156.0** |
| 11-month capacity | 5 engineers × 11 months × 80% = 44.0 engineer-months × 4.333 | **190.7** |
| Committed MVP Must | §2 row sum | **186.0** (≈42.9 engineer-months) |
| Gap to 9 months | 186.0 − 156.0 | **30.0** |
| Headroom at 11 months | 190.7 − 186.0 | **4.7** |

**Plan of record: 10.7 months at five engineers** (186.0 ew ÷ 17.33 effective ew per month, where 17.33 = 5 engineers × 80% × 4.333), inside Q6's confirmed 10–11 month envelope. The explicit comparisons the prior artifact lacked:

- The former total was **192.0 ew**: it exceeded the feasibility upper bound of **186 ew** by 6.0 ew and exceeded the 11-month capacity of **190.7 ew** by 1.3 ew (it was 11.1 months, not 10.7).
- The three duplicate merges removed **6.0 ew of duplicated effort**, not scope, taking 192.0 → 186.0.
- 186.0 now sits **exactly at the feasibility upper bound of 186 ew (0.0 headroom)** and **4.7 ew under the 11-month capacity of 190.7 ew**. It is inside both envelopes; the 9-month target is not.
- The 108–186 ew range is the feasibility demand range with xpub as Should. 186.0 is its top edge; it is not a claim that the low end is available.

### 1.3 Cluster totals (sum of §2 rows)

| Cluster | Units | Committed MVP ew |
|---|---|---|
| Identity (incl. merged auth/session, support, onboarding/gate) | U-001–U-006 (6) | 25.5 |
| Account | U-007–U-012 (6) | 31.5 |
| Asset Registry | U-013–U-015 (3) | 10.0 |
| Market Data | U-016–U-019 (4) | 14.5 |
| Portfolio, Ledger, Reporting, Billing | U-020–U-039 (20) | 40.0 |
| Gateway, Jobs, Audit, SPA, privacy, degraded paths | U-040–U-054 (15) | 47.5 |
| Platform and DevSecOps | U-055–U-058 (4) | 17.0 |
| **Total** | **58 units** | **186.0** |

### 1.4 Mixed Must/Should units (explicit split)

Every mixed unit carries its **Must** effort in the §2 `ew` column and its **Should** tail in §5.2, never both in the same total.

| Unit | Must capability (in 186.0) | Must ew | Should fast-follow tail (in 14.5) | Should ew |
|---|---|---|---|---|
| U-012 | CSV import | 2.0 | xpub/watch-only (FR-ACC-21) | 2.0 |
| U-036 | Brief email delivery | 1.5 | PDF/PDF-UA briefs (FR-REP-15, 21) | 1.0 |
| U-037 | Performance core | 2.0 | benchmark comparison, attribution, contribution, decomposition, fee drag, export (FR-PERF-03, 04, 05, 15, 16, 17) | 1.0 |
| U-049 | English (EU) and English (UK) locales | 2.5 | German/French/Spanish locales and legal review (FR-L10N-05/06/09/19 under C16) | 1.5 |
| U-051 | PWA shell for mobile delivery | 1.5 | PWA installability and install prompt | 1.5 |
| U-020 | Portfolio lifecycle | 1.5 | archive/restore (FR-PORT-15, 25) | 1.0 |
| U-027 | Snapshot capture, history and trends | 2.5 | snapshot and performance CSV/PDF exports (FR-VAL-22, FR-PERF-17) | 1.0 |
| U-039 | Checkout and subscription lifecycle | 3.0 | local-currency display and trial (FR-BIL-04, 05) | 1.0 |
| U-034 | Order intents and trade recording | 2.5 | venue deep links (FR-ORD-14) | 0.5 |
| U-052 | Mandatory alert degraded path | 2.5 | optional alert types (FR-NOT-05/06 class) | 0.5 |
| U-007 | Account lifecycle and quotas | 3.5 | sub-account parent roll-up (FR-ACC-27) | 0.5 |
| U-010 | Ten-chain accounts | 9.0 | DeFi/staking/LP labelling (FR-ACC-26) | 0.5 |
| U-018 | Provenance and unpriced handling | 2.5 | watchlist and price thresholds (FR-AST-24, 26) | 0.5 |
| U-016 | Multi-source price ingestion | 6.0 | long-tail DEX pricing (FR-AST-20) | 0.5 |
| U-042 | SPA core screens | 6.0 | non-core screens | 1.0 |
| U-040 | Gateway entitlements and quotas | 2.0 | usage-versus-quota UI | 0.5 |
| **Total** | | | | **14.5** |

## 2. Proto-Unit register

Every unit names one primary launch service, one MoSCoW value, one WSJF score, hard build dependencies, testable acceptance criteria, committed Must effort and one release category. Requirement IDs are single-owner; a unit may consume another unit's contract but does not re-own the ID. Runtime contracts consumed through stubs are named in the description and are not build edges.

### 2.1 Identity — U-001…U-006

| Unit | Name | Description | Launch service | BRD requirement IDs | MoSCoW | WSJF (CD/JS = score — rationale) | Depends on | Acceptance criteria (testable) | ew | Release |
|---|---|---|---|---|---|---|---|---|---|---|
| **U-001** | Auth, session and gateway trust | One shared Rust auth crate validating the Supabase JWT (`iss`, `aud`, `exp`/`nbf`, algorithm pinning, JWKS cache/rotation) and the internal service token; the gateway's reviewed route table, principal resolution, correlation IDs and deny-by-default; the SPA shell with PKCE flows, in-memory tokens, silent refresh, full sign-out, one API-client module and the Parchment Ledger token layer. | API Gateway (with Identity and SPA) | FR-AUTH-01–04, 06–11, 21, 22; FR-JOB-06; NFR-SEC-08, 09, 10 | Must | 10/7 = 1.43 — every request and screen depends on it; the merged crate + gateway + shell is large, but one implementation replaces three. | U-055 | (1) `alg:none`, wrong `iss`/`aud`, expired token or unknown `kid` returns 401 at the gateway and at every service; an unknown `kid` triggers one refetch then 401. (2) An unregistered path returns 404 with no upstream call; a user token is refused on internal run endpoints and a service token is refused on user routes. (3) An implicit-flow callback is rejected; expired access + valid refresh succeeds without re-sign-in. (4) No access or refresh token appears in `localStorage`, `sessionStorage`, a readable cookie or any log across a full session lifecycle. (5) An architecture test proves every gateway call originates in the single API-client module; the shell is keyboard navigable and axe-clean at 360 px. | 7.0 | MVP |
| **U-002** | Internal user provisioning and profile | Lazy upsert keyed by the Supabase `sub`; stable internal ID across claim rotation; profile locale, base currency and timezone; `active`/`disabled`/`pending_deletion`/`deleted` lifecycle; disable without deleting history. | Identity | FR-USR-01, 02, 07, 09, 20; FR-AUTH-05 | Must | 10/2 = 5.0 — nothing can be owned without the user record; deliberately tiny. | U-001 | (1) First authenticated request creates exactly one internal user mapped to `sub`; a second request or claim rotation creates none. (2) Profile update persists and leaves stored quantities and values unchanged. (3) A disabled user with a valid provider token is denied on the next request; only the four statuses are accepted. (4) Same email with a different subject does not merge identities. | 2.0 | MVP |
| **U-003** | Roles, ownership scoping and admin access | `app_metadata.role` projection; deny-by-default route policy; owner scope on every user-owned resource; non-disclosing 404/403; admin lookup, role change, disable and entitlement override with mandatory audit; cross-tenant suite as a merge gate. Owns admin-access FR-ADM-14. | Identity (with API Gateway and Audit) | FR-USR-03, 04, 05, 06, 17, 21; FR-ADM-01, 14; NFR-SEC-07, 08 | Must | 10/4 = 2.5 — a cross-tenant leak is the top-severity risk; scoping every repository is real work. | U-001, U-002, U-047 | (1) A non-admin calling any admin endpoint is denied; Support is denied every owner endpoint. (2) Every cross-user read/write returns 404 with no existence hint; the cross-tenant suite covers 100% of endpoints as a merge gate. (3) A role/entitlement change writes actor, target, before/after, reason and timestamp to audit and takes effect on the next request. (4) An admin user lookup, role change, disable and entitlement override each appear in audit. | 4.0 | MVP |
| **U-004** | MFA, device/session control and recovery | MFA before any exchange credential; verified-email prerequisite; device/session list with last-seen and individual/all revocation; revocation after password/MFA/passkey change; new-device alert; provider-led recovery without secret disclosure. New-device alerts use U-052's delivery contract. | Identity (with SPA) | FR-AUTH-12, 15, 16, 17, 18; FR-USR-13; NFR-SEC-11, 17 | Must | 8/3 = 2.67 — MFA gates credentials, but the provider does most of the work. | U-001, U-002 | (1) No MFA blocks exchange-credential add and prompts enrolment; after enrolment the save succeeds. (2) An unverified email blocks credential add. (3) Revoking one device ends that session on the next request; revoking all ends all. (4) A password/MFA/passkey change revokes other sessions; a new-device alert contains no token or full fingerprint. (5) Recovery does not reveal whether an address exists. | 3.0 | MVP |
| **U-005** | Support access and break-glass control | Time-bound, reason-recorded, second-approver support grants and the Q8 break-glass read-only admin view behind the gateway; account metadata only, never credentials or portfolio contents; automatic expiry; every grant, view and expiry audited; external help desk, in-app contact, published support hours and escalation. | Identity (with Audit) | FR-USR-11, 19; FR-ADM-15; NFR-SEC-19; FR-SUP-01, 02, 03, 05, 06, 07, 08, 14, 17, 18 | Must | 5/2 = 2.5 — required for the Q8 support path and audit; the policy layer and surface are small. | U-003, U-047 | (1) A grant requires a reason and a second approver; it expires automatically (≤60 minutes) and grant/view/expiry all appear in audit. (2) No support screen, export or API response contains a credential or portfolio value (contract test enumerates fields). (3) Self-approval and permanent grants are impossible; an expired grant is denied. (4) In-app contact and bug report exclude secrets and return a ticket ID and target response; hours and escalation are published. | 2.5 | MVP |
| **U-006** | Onboarding, consent and jurisdiction/age gate | Fail-closed gate at the edge before logging and before any non-essential cookie, plus a Supabase `before user created` hook or signup proxy and a gateway defence-in-depth check; versioned counsel allowlist (EU/EEA and UK, UK only after recorded sign-off); age attestation; versioned terms/risk/age acceptance before the first source; guided first-run (demo, address, WalletConnect, CSV, exchange key), first valuation and activation events. Account/portfolio contracts are consumed through stubs so the gate ships before the first real user. | Identity (with edge/WAF and SPA) | FR-USR-08, 12, 15, 16; FR-AUTH-19, 20; FR-ONB-01–05, 07–10, 12, 15, 16, 21, 22; FR-PRV-18; NFR-PRV-12; NFR-LEG-02 | Must | 10/7 = 1.43 — collecting data before the gate is unlawful and cannot be undone; onboarding carries the activation metric. Merging the three duplicate gate/consent paths removes divergent implementations. | U-001, U-002, U-047, U-055 | (1) A blocked country is refused at the edge before any personal-data log line or non-essential cookie; no Supabase auth user, email or IP is persisted. (2) An under-age attestation cannot create an account. (3) An allowlist version change applies without deploy, is audited with version and effective date, and an unknown country fails closed; a UK sign-up is refused until recorded counsel sign-off. (4) Terms, risk and age acceptance is versioned and timestamped before the first source; a version bump re-prompts. (5) A demo portfolio opens in under 30 seconds and is deletable; a valid address reaches first valuation in under 2 minutes; a failed sync resumes at the same step. (6) Each activation event is emitted once, without PII. | 7.0 | MVP |

### 2.2 Account — U-007…U-012

| Unit | Name | Description | Launch service | BRD requirement IDs | MoSCoW | WSJF (CD/JS = score — rationale) | Depends on | Acceptance criteria (testable) | ew | Release |
|---|---|---|---|---|---|---|---|---|---|---|
| **U-007** | Account lifecycle, regional coverage and quotas | Create/edit/activate/deactivate/delete five account types with type-specific validation; region-aware venue availability with a reason code; server-side plan quotas; account summary. Sub-account roll-up is a Should tail. | Account | FR-ACC-01, 03, 06, 08, 09, 17, 28, 29; FR-ACC-27 (Should) | Must | 8/3.5 = 2.29 — accounts are the entry point for all holdings; routine CRUD. | U-003, U-014, U-040 | (1) Exchange create requires a name plus complete credentials; on-chain create requires an address and at least one active chain; errors name the field. (2) An inactive account is excluded from sync and aggregation; delete removes portfolio links only. (3) A venue unavailable in the region is absent with a reason code, not a generic error. (4) A Free user exceeding the account quota is blocked server-side before persistence. | 3.5 | MVP |
| **U-008** | Exchange onboarding and read-only key verification | Five venue adapters (OKX, Binance, Coinbase, Kraken, Bybit) behind an `ExchangeReader` port; Vault references only; permission probe at save and on schedule; reject trade/withdraw keys; revocation/permission-change detection with mandatory alert; per-venue circuit breaker and one cluster-wide per-venue/per-IP bucket. | Account | FR-ACC-04, 05, 07, 18, 19; FR-JOB-22; NFR-SEC-04, 05, 06; CON-026 | Must | 9/9 = 1.0 — highest value and a compliance gate, but five venue auth paths are the least predictable build. | U-001, U-004, U-007, U-055 | (1) Saving a valid read-only key stores only a Vault reference; no response, log, trace or error contains the secret. (2) A trade- or withdraw-enabled key is rejected before persistence with actionable guidance. (3) A revoked or permission-changed key is detected within one cycle, the account is marked inactive and an alert is recorded. (4) Adapter fixtures pass for all five venues; 429/418 triggers backoff with no retry storm; a two-replica test proves the shared bucket. | 9.0 | MVP |
| **U-009** | Holdings sync orchestration | Single and bulk sync with per-account results; idempotent replacement; decimal normalisation; zero filtering; spam/dust hiding and exclusion from valuation; partial-sync state; plan cadence; `u128`/bigint parsing before `rust_decimal`. | Account | FR-ACC-10–16, 25, 30; FR-JOB-24 (holdings side); NFR-ACC-04 | Must | 9/5 = 1.8 — makes every account useful; one pipeline serves every source. | U-008, U-010, U-011, U-012, U-014 | (1) A successful sync replaces holdings with normalised decimals, filters zero balances and updates last-synced only on success. (2) One account failure is recorded with a reason and does not stop the bulk run. (3) Re-running the same sync creates no duplicate holdings. (4) Spam/dust is hidden and excluded from valuation but retained in the source of truth. (5) A raw uint256 balance above the mantissa limit is parsed safely and tested; a throttled venue produces a labelled partial-sync gap. | 5.0 | MVP |
| **U-010** | On-chain accounts across ten chains | Address validation and balance readers for seven EVM chains, Solana/SPL, Bitcoin native and Tron/TRC-20 incl. USDT; registry decimals; RPC primary/fallback with source recorded. DeFi/staking/LP labelling is a Should tail. | Account | FR-ACC-02, 23, 24; FR-ACC-26 (Should); NFR-SEC-05 | Must | 8/9 = 0.89 — required coverage and the largest single block; bounded chain families. | U-007, U-014, U-019 | (1) Malformed addresses for each chain family are rejected before any network call. (2) Sync returns native and token balances for all ten chains; USDT on Tron is read as TRC-20. (3) The same asset on two chains appears once with a combined quantity; unknown decimals is a sync error, never a guess. (4) A primary RPC failure falls back and records the source used. | 9.0 | MVP |
| **U-011** | WalletConnect read-only connection | Browser WalletConnect session adds a read-only account by public address; scope restricted to read methods; disconnect/expiry deactivates; reconnect restores without duplication. | Account (with SPA) | FR-ACC-20; CON-005 | Must | 6/3 = 2.0 — a committed Must path, but only a subset of users; bounded SDK integration. | U-007 | (1) A session adds an account with its address and no stored secret. (2) No signing or transaction method is requested; a signing request is refused by construction (scope test). (3) Disconnect or expiry deactivates the account and stops sync; reconnect restores without duplication. | 3.0 | MVP |
| **U-012** | CSV import (xpub fast-follow) | CSV holdings/transaction import with template, column-mapping preview and idempotent commit; ambiguous rows quarantined with a reason; reconciliation against synced holdings. **Must effort is CSV only (2.0); xpub/watch-only is a separate 2.0 Should tail.** | Account | FR-ACC-22; FR-ACC-21 (Should) | Must (CSV) / Should (xpub) | 6/2 = 3.0 — CSV rescues unsupported venues; xpub is a cheap fast-follow but not committed. | U-007, U-013 | (1) Preview shows detected columns, sample rows and target fields before commit. (2) Re-importing the same file creates no duplicates; each batch links to its source-file hash. (3) Unmapped or ambiguous assets are quarantined with a reason, never silently dropped. (4) xpub (Should tail): a valid xpub adds a watch-only account syncing native Bitcoin with no private key. | 2.0 | MVP |

### 2.3 Asset Registry and Market Data — U-013…U-019

| Unit | Name | Description | Launch service | BRD requirement IDs | MoSCoW | WSJF (CD/JS = score — rationale) | Depends on | Acceptance criteria (testable) | ew | Release |
|---|---|---|---|---|---|---|---|---|---|---|
| **U-013** | Canonical asset catalogue | Root asset identity (unique symbol, name, type, rank, aliases); shared read contract over HTTP; admin-only mutation; deactivation retains history; consumers store IDs plus cached display fields. | Asset Registry | FR-AST-01, 02 | Must | 9/3 = 3.0 — every valuation, price and holding depends on canonical identity; the model is small. | U-003 | (1) A new asset requires a unique symbol; a duplicate symbol or alias collision is rejected. (2) An alias resolves to exactly one canonical asset. (3) Consumers resolve identity through a batch HTTP endpoint; a cross-schema read of `registry` fails the boundary test. | 3.0 | MVP |
| **U-014** | Chain/token contract registry and decimals | One representation per asset per chain (contract/mint/address or native symbol), fixed decimals, native/spam flags; duplicate rejection; resolution of synced tokens; quarantine; decimals immutable once used in a valuation. | Asset Registry | FR-AST-03–06; FR-ADM-04, 05, 06 | Must | 9/4 = 2.25 — wrong contracts or decimals corrupt every balance; registry shapes are well bounded. | U-013 | (1) Registering a token requires chain, identifier, decimals and flags; a duplicate contract on the same chain is rejected. (2) A registered token resolves to its asset and decimals and normalises a raw balance exactly. (3) An unregistered token is quarantined, visible, excluded from valuation and never guessed. (4) Changing decimals after a valuation requires explicit review and triggers re-normalisation. | 4.0 | MVP |
| **U-015** | Registry administration and catalogue sync | Admin CRUD for chains and tokens with review-before-activation; chain-ID uniqueness; deactivation warning; changes effective next sync without deploy; canonical token-list import staged and explicitly selected; daily catalogue refresh with run records. | Asset Registry | FR-ADM-02, 03, 07, 10, 11, 12; FR-AST-11 (Should); FR-JOB-01 | Must | 7/3 = 2.33 — operators need it before coverage can be corrected; thin CRUD over U-013/U-014. | U-003, U-013, U-014 | (1) A non-admin mutation is denied; every create/update/deactivate writes actor, timestamp and before/after. (2) A duplicate chain ID is rejected; deactivating a chain with active accounts warns and requires confirmation. (3) A change applies on the next sync without deploy; historical snapshots are unchanged. (4) Import stages results and activates only on explicit selection; the daily refresh leaves a run record with updated/skipped counts. | 3.0 | MVP |
| **U-016** | Multi-source price ingestion and freshness tiers | Provider port with deterministic per-asset/per-region priority and fallback; 1-minute top-asset and 15-minute tail collection; last-stored-price fallback labelled with age; observation history; per-asset failure isolation. Long-tail DEX pricing is a Should tail. | Market Data | FR-AST-07, 08, 12, 15, 18; FR-JOB-02, 10 | Must | 10/6 = 1.67 — freshness is a headline success metric and a licensing proof; multi-source ingestion is substantial. | U-013, U-019 | (1) Collection meets NFR-JOB-01/02 budgets and records succeeded/failed counts per asset. (2) Primary failure falls back to a secondary and records the source; both fail yields an aged last price; no source yields unpriced. (3) One asset's failure does not stop the run. (4) Observations store timestamp, source and confidence and remain queryable. (5) A region without the pair returns an explicit unavailable state, never a fabricated price. | 6.0 | MVP |
| **U-017** | FX collection and historical conversion | Hourly and daily-close FX for every supported base currency; source/time on each observation; historical lookup by transaction/snapshot date; admin-configurable currencies and priority; explicit degradation. | Market Data | FR-AST-16, 17; FR-JOB-17; FR-ADM-17 | Must | 8/3 = 2.67 — base-currency valuation is a Must for global retail; one compact series. | U-019 | (1) Every supported currency stores hourly and daily-close rates with source and timestamp. (2) Historical lookup returns the applicable rate or an explicit missing-rate state. (3) A source failure uses the last-known rate with its age; the system never substitutes 1.0 silently. (4) Admin currency/source changes apply on the next run without deploy. | 3.0 | MVP |
| **U-018** | Provenance, quality and unpriced handling | Provenance contract exposed to Portfolio/Reporting (source, observation time, confidence, staleness tier); stale and unpriced labels; cross-source divergence alert. Watchlist and price thresholds are a Should tail. | Market Data | FR-AST-09, 10, 14; FR-AST-19, 24, 26 (Should) | Must | 9/2.5 = 3.6 — trust depends on provenance and staleness labels; contracts over existing observations. | U-016, U-017 | (1) Every valuation/P&L response carries source, observation time, confidence and staleness state. (2) A price older than its tier threshold is labelled stale and never presented as current. (3) An unpriced asset is excluded with a reason. (4) Divergence beyond threshold raises an alert preserving both observations. (5) A de-peg alert and a watchlist are Should tails and do not affect the priced total. | 2.5 | MVP |
| **U-019** | Licensing, attribution and source administration | Licence register per source covering display, storage, history, derivation, redistribution and region; fail-closed activation gate; attribution payload wherever licensed data appears; source priority, assets, regions, health and last success; audited manual override (Could) excluded from benchmarks and history. | Market Data | FR-AST-13, 22, 25; FR-ADM-16; NFR-LEG-05, 06 | Must | 10/3 = 3.33 — no licensed source means no launch, and attribution is cheap once the register exists. | U-003, U-013 | (1) A source without a licence reference cannot be activated; activation fails closed. (2) Attribution for each displayed source appears wherever its data is shown; a contract test asserts the payload. (3) Priority, assets and regions are configurable and effective next run without deploy. (4) Health exposes last success and alerts when unhealthy; a manual override is audited, visually distinct and excluded from benchmark/history. | 3.0 | MVP |

### 2.4 Portfolio, Ledger, Reporting and Billing — U-020…U-039

| Unit | Name | Description | Launch service | BRD requirement IDs | MoSCoW | WSJF (CD/JS = score — rationale) | Depends on | Acceptance criteria (testable) | ew | Release |
|---|---|---|---|---|---|---|---|---|---|---|
| **U-020** | Portfolio lifecycle and ownership isolation | Create/edit/delete portfolios with case-insensitive unique names per user, exactly one default, owner-only access, and deletion that never touches accounts or transactions; an empty portfolio is valid and shows zero. Archive/restore is a Should tail. | Portfolio | FR-PORT-01–06, 17; FR-PORT-15, 25 (Should) | Must | 8/1.5 = 5.33 — every portfolio surface hangs off this identity; cheap and unblocking. | U-003 | (1) The first portfolio is default; a duplicate name differing only by case is rejected. (2) Redesignation leaves exactly one default. (3) Delete leaves accounts and transactions intact. (4) A non-owner read or write returns 404 with no hint and raises an audit event. | 1.5 | MVP |
| **U-021** | Shared-account membership, overlap and sell availability | Add/remove the user's own accounts as members; reject foreign and duplicate membership; allow one account in many portfolios with an explicit shared warning; show overlapping value; expose account-level sell availability net of pending sell commitments. U-034's pending-sell contract is consumed as a stub, so the build edge is one-way. | Portfolio | FR-PORT-07–10, 18, 19, 27; D-004 | Must | 9/2 = 4.5 — D-004 is a launch rule; without it value double-counts and sells double-spend. | U-007, U-020 | (1) A foreign account is rejected; duplicate membership is rejected with a clear message. (2) A shared account appears once per portfolio and is marked shared. (3) A pending sell in one portfolio reduces availability in another. (4) A user-level total counts the account once. | 2.0 | MVP |
| **U-022** | Target allocation and tolerance bands | Asset-to-percentage target map totalling exactly 100% including an explicit `unallocated` bucket; optional min ≤ target ≤ max band around a single target (never an independent range allocation). | Portfolio | FR-PORT-11, 12, 20 | Must | 8/1 = 8.0 — drift, briefs and guardrails all consume targets; small but blocking. | U-013, U-020 | (1) 40/30/20/10 is accepted; a 99% total is rejected with the shortfall named. (2) A band is accepted only when min ≤ target ≤ max and the single targets total 100%. (3) `unallocated` round-trips and counts toward 100%. | 1.0 | MVP |
| **U-023** | Guardrails and documented defaults | Max deviation (default 5pp), min/max trade size and allowed assets; documented defaults when unset; explicit audited disable; values constrain order creation in U-034. | Portfolio | FR-PORT-13, 24 | Must | 7/1.5 = 4.67 — protects users from oversized orders and makes drift meaningful; cheap once targets exist. | U-020, U-022 | (1) The 5pp default applies when no guardrail is configured. (2) A stricter guardrail is accepted; looser values require an explicit recorded decision. (3) An order below min, above max or on a disallowed asset is rejected with the limit named. (4) Disabling guardrails writes actor, time and reason to audit. | 1.5 | MVP |
| **U-024** | Base-currency valuation and cross-account asset merge | Aggregate active member accounts; merge the same canonical asset across accounts and chains into one line; value quantity × price × FX with `rust_decimal`; per-asset value and weight; unpriced/unmapped excluded and labelled; price/FX/last-sync timestamps; overflow guard at the adapter boundary. | Portfolio | FR-VAL-01, 02, 05, 06, 16, 20; NFR-ACC-01, 02 | Must | 10/3 = 3.33 — the product's core promise and the input to drift, snapshots, briefs and performance. | U-007, U-009, U-013, U-016, U-017, U-020, U-021 | (1) BTC 0.5 @ 50k plus ETH 5 @ 3k equals 40k at 50%/30%. (2) The same asset in two accounts appears once and combined. (3) An unpriced asset is listed, excluded and the exclusion stated; an all-unpriced portfolio shows zero with a label, not an error. (4) A EUR user sees the FX rate and timestamp. (5) A raw uint256 balance above the mantissa limit is parsed safely and tested. | 3.0 | MVP |
| **U-025** | Materialised valuation read model | Per-portfolio read model recomputed by a Job Orchestration poll of Market Data; request-time fan-out only for an explicit user refresh; batch-by-ID reads; freshness/staleness badges with last-known-price fallback. | Portfolio | FR-VAL-14; NFR-PERF-02; CON-051 | Must | 9/1.5 = 6.0 — p95 <2 s and the 99.9% claim survive only if the read path is materialised. | U-024, U-043 | (1) The M0 fixture reproduces the expected valuation exactly. (2) p95 <2 s at 300 RPS for 60 minutes with one worker drained and error rate <0.5% (C10). (3) A stale price shows a staleness badge and never a silently wrong total. (4) An explicit refresh recomputes without creating a snapshot. | 1.5 | MVP |
| **U-026** | Allocation, weights, drift and breach | Allocation totals 100% of priced value; weights to ≥4 decimal places internally; drift = actual − target only where a target exists; breach flag against max deviation, exposed to the portfolio view, briefs and the alert path. | Portfolio | FR-VAL-03, 04; FR-PORT-14; NFR-ACC-03 | Must | 8/1.5 = 5.33 — the decision surface users return for; cheap once valuation and targets exist. | U-022, U-023, U-024 | (1) Target 40%, actual 45% yields drift +5pp. (2) Weights sum to 100% within display rounding. (3) An asset with no target shows no drift. (4) Drift beyond the guardrail is flagged for U-035 and the alert path. | 1.5 | MVP |
| **U-027** | Immutable snapshots, history and trends | Manual and EOD snapshots per portfolio/local date/type; a repeat updates, never duplicates; stores base currency, FX rate and source, per-asset quantity/price/value/weight, account and asset counts, coverage and P&L availability; reverse-chronological history, drill-down, chart, type filter, retention, no back-dating. Exports are a Should tail. | Portfolio | FR-VAL-07–13, 15, 17, 18, 21, 23; NFR-ACC-04 | Must | 8/2.5 = 3.2 — history is the retention hook and cannot be backfilled; large and serial. | U-024, U-026 | (1) Re-submitting today's manual snapshot updates it with no duplicate. (2) Every active portfolio has exactly one EOD snapshot for its local date, including a DST transition. (3) A snapshot opens its recorded per-asset breakdown; a later-deleted account leaves history unchanged. (4) Snapshot coverage equals the priced share at generation time. | 2.5 | MVP |
| **U-028** | Ledger ingest: exchange API, CSV and manual entries | Append-only ledger; incremental import from five venue APIs and CSV with mapping preview and per-exchange templates; manual entry and corrections as adjusting entries; per-row import errors with partial success; immutable retention. | Ledger | FR-TXN-01, 02, 03, 13, 20; FR-JOB-16; CON-039 | Must | 9/2.5 = 3.6 — P&L is impossible without it; the largest Ledger item. | U-007, U-008, U-013 | (1) Importing 100 OKX trades yields 100 unique transactions; re-importing yields none. (2) A CSV with three bad rows imports the good rows and reports the three errors, none silently dropped. (3) A manual correction creates an adjusting entry with actor, time and reason; nothing is hard-deleted. (4) Every row carries asset, quantity, type, source and timestamp, or is rejected. | 2.5 | MVP |
| **U-029** | Dedupe, classification and ledger integrity | Deduplicate by source plus external ID/hash (idempotent imports); classify into the nine transaction types with an audited user correction; flag oversells, negative quantities and impossible timestamps instead of clamping. | Ledger | FR-TXN-04, 05, 19 | Must | 8/1.5 = 5.33 — duplicate and misclassified rows are the credibility failure the product exists to fix. | U-028 | (1) Re-import produces no duplicates. (2) An oversell is flagged and correctable only by an adjusting entry. (3) A classification change records actor, time and before/after. (4) An impossible timestamp is flagged, never silently rewritten. | 1.5 | MVP |
| **U-030** | Transfer pairing and internal-transfer exclusion | Pair deposits and withdrawals between the user's own accounts on asset, quantity and a bounded window; paired legs carry a pair ID and are excluded from P&L; unmatched legs stay visible; never pair across users. | Ledger | FR-TXN-06 | Must | 7/1 = 7.0 — mispaired transfers fabricate P&L, the most damaging ledger error; small and bounded. | U-028, U-029 | (1) Moving BTC between two own wallets creates no realized P&L. (2) A paired transfer shows both legs and one pair ID. (3) An unmatched leg remains queued and is never auto-paired across users. (4) Unpairing restores both legs with an audit event. | 1.0 | MVP |
| **U-031** | Holdings reconciliation, coverage and P&L availability | Reconcile ledger-derived quantity against latest synced holdings per account and asset with states matched, within tolerance, mismatched and unavailable; user-visible queue; coverage percentage governs P&L availability. | Ledger | FR-TXN-11, 12; FR-JOB-24 | Must | 8/2 = 4.0 — the >99% accuracy claim is evidenced here; silent overwrite would destroy trust. | U-009, U-028, U-029, U-030 | (1) A quantity difference is shown as mismatched. (2) Missing pre-import history shows P&L unavailable with an explanation. (3) Coverage equals the history-covered share and is stored with the snapshot. (4) Neither ledger nor synced data is silently overwritten; resolution is an adjusting entry. | 2.0 | MVP |
| **U-032** | Cost basis engine | Remaining cost basis per asset and account under FIFO (launch default) and selectable average cost; audited method change with full recomputation; deterministic and reproducible from the append-only ledger. | Ledger | FR-TXN-07, 14 | Must | 9/3 = 3.0 — the differentiator and the base of every P&L number; highest-churn domain. | U-028, U-029, U-030, U-031 | (1) Buy 1 BTC at 50,000 with a 10 fee at price 60,000 yields unrealized 9,990 under FIFO. (2) Selling half consumes the oldest lots and reduces basis correctly. (3) A FIFO-to-average-cost change recomputes and leaves an audit record. (4) Identical ledger input reproduces an identical basis. | 3.0 | MVP |
| **U-033** | P&L, net invested and tax-ready export | Net invested, current cost basis, realized/unrealized P&L, income, fees, total P&L and simple return per asset, account, portfolio and user in the base currency; transaction-time FX with source and timestamp; CSV/PDF and a jurisdiction-neutral tax-ready dataset (data, not filing or advice); user totals count a shared account once. | Ledger | FR-TXN-08, 09, 10, 15, 16, 17, 18; D-005, D-013 | Must | 9/2 = 4.5 — directly answers "am I up and why", the paid-value proposition. | U-017, U-021, U-032 | (1) Total P&L reconciles to realized + unrealized + income − fees within rounding. (2) A EUR user sees the transaction-date FX rate, source and timestamp. (3) The export carries asset, type, quantity, price, fee, timestamp, source and base-currency value. (4) Missing history yields unavailable, never zero; unpriced assets are excluded and labelled. (5) User-level P&L counts a shared account once; the portfolio view labels the overlap. | 2.0 | MVP |
| **U-034** | Order intents, trade recording and immediate allocation effect | User-initiated buy/sell intents (pending to filled or cancelled) with projected allocation preview; cancel; record a trade with venue, quantity, execution time, price, fees and external reference; apply the allocation effect immediately to every portfolio containing the account; duplicate detection; reconciliation state; shared warning; export. The venue deep link is a Should tail and is never presented as execution. | Portfolio | FR-ORD-01–13, 17, 18, 20; FR-ORD-14 (Should); D-003 | Must | 7/2.5 = 2.8 — completes the decision loop; no Must elsewhere depends on it. | U-021, U-023, U-024, U-026, U-028, U-047 | (1) A Buy creates a pending order in history; cancel creates no trade. (2) Recording the trade moves BTC 10% → 12% immediately in every containing portfolio. (3) An oversell across all portfolios is rejected; a duplicate external reference is rejected. (4) A shared-account trade warns and updates each portfolio. (5) Network inspection shows no venue call from the browser or service. | 2.5 | MVP |
| **U-035** | Brief composition, in-app history and preferences | Deterministic localized brief (header, summary, P&L, performance, allocation vs target, holdings, orders, trades, data quality, disclaimer) on daily/weekly/manual schedules; generation log; in-app history; per-user frequency, channels and portfolio scope; timezone grouping; never reads credentials. | Reporting | FR-REP-01–05, 07, 09, 10, 12, 16–19, 24; D-019 | Must | 7/2.5 = 2.8 — the habitual retention surface and the Q8 alert path; must not slip. | U-024, U-026, U-027, U-033, U-034, U-037, U-049 | (1) Every required section plus the disclaimer is present. (2) Drift beyond a guardrail is highlighted; unpriced assets and their exclusion are stated. (3) The generation log records status and time; a user who disabled reporting gets no scheduled brief. (4) P&L is shown or explicitly unavailable; the brief is localized and date-grouped in the user's timezone. | 2.5 | MVP |
| **U-036** | Brief email delivery and retention | Email delivery with consent and one-click unsubscribe; idempotent retry per owner/portfolio/period; generate-now; retention and user deletion; accessible in-app content. Brief-specific email is owned here; U-052 owns alert transport. PDF/PDF-UA is a Should tail. | Reporting | FR-REP-08, 22; FR-REP-15, 21 (Should) | Must | 6/1.5 = 4.0 — without a channel the brief is invisible, and Q8 makes email mandatory. | U-035, U-048, U-052 | (1) A retried failure creates no duplicate brief for the period. (2) A user receives in-app and, when enabled, email; unsubscribe is honoured immediately. (3) Deleting a brief leaves portfolio data intact. (4) In-app content is accessible and carries the disclaimer; PDF is a Should tail. | 1.5 | MVP |
| **U-037** | Performance core | Simple return, TWR and MWR/IRR over selectable periods in the base currency, excluding external flows from TWR and internal transfers from both; methodology disclosure; unavailable labels; accessible chart with data table; audited recompute; no forecast. Benchmark comparison, attribution, contribution, decomposition, fee drag and export are a Should tail. | Portfolio | FR-PERF-01, 02, 06–09, 12–14, 18; FR-PERF-03, 04, 05, 15–17 (Should); D-005, D-020 | Must | 6/2 = 3.0 — a valuable retention differentiator, but no Must capability depends on it and benchmark rights are externally gated. | U-019, U-027, U-030, U-033 | (1) One deposit yields simple return equal to TWR. (2) A mid-period deposit does not inflate TWR; MWR reflects timing. (3) An internal transfer changes neither. (4) Incomplete history labels the return unavailable; no forecast or recommendation appears. (5) A methodology change triggers an audited recomputation. | 2.0 | MVP |
| **U-038** | Plans, entitlements and quotas | Free/Pro plan catalogue and entitlement matrix (accounts, portfolios, members, refresh cadence, history, exports, API); server-side enforcement contract; plan-change audit; read-only behaviour on downgrade. U-040 owns gateway enforcement and the cache. Usage-versus-quota UI is a Should tail. | Billing | FR-BIL-06, 07, 14, 15, 19; FR-ADM-19 | Must | 9/2 = 4.5 — paid launch is impossible without server-side entitlements and a plan catalogue. | U-003 | (1) A Free user's fourth account is blocked with an upgrade path and no data loss. (2) After downgrade, excess accounts are read-only and visible. (3) A client that bypasses the UI is still refused. (4) Every plan change is audited; an expired plan leaves data intact and readable. | 2.0 | MVP |
| **U-039** | Merchant-of-record checkout and subscription lifecycle | MoR hosted-checkout redirect (no embedded third-party script); tax status shown before checkout; monthly and annual terms; upgrade, downgrade, cancel, reactivate; renewal and payment-failure notices; localized invoices; dunning with a grace period; refunds and chargebacks; billing closure that never deletes portfolio data; revenue export. Local-currency display and trial are a Should tail. | Billing | FR-BIL-01, 02, 03, 08–12, 20; FR-BIL-04, 05, 18 (Should); NFR-LEG-09; D-009, D-018 | Must | 9/3 = 3.0 — revenue start and the consumer-law gate; large but provider-mediated. | U-036, U-038, U-052 | (1) A failed renewal enters `past_due`, notifies and preserves data through the grace period. (2) Tax status, final charge and renewal date are shown before confirmation. (3) Cancellation takes effect at period end and is confirmed by email. (4) A refund is auditable and adjusts entitlements. (5) No pre-checked consent and no hidden cancellation; cancellation-parity test passes. | 3.0 | MVP |

### 2.5 Gateway, Jobs, Audit, SPA, privacy and degraded paths — U-040…U-054

| Unit | Name | Description | Launch service | BRD requirement IDs | MoSCoW | WSJF (CD/JS = score — rationale) | Depends on | Acceptance criteria (testable) | ew | Release |
|---|---|---|---|---|---|---|---|---|---|---|
| **U-040** | Gateway entitlements and quotas | Check Billing entitlements and quotas for every gated action; 60-second cache invalidated on subscription change; machine-readable 402/403 with an upgrade path; shared quota counters; admin override with expiry; fail closed for gated writes, labelled last-known entitlement for reads. | API Gateway | FR-JOB-21; NFR-SEC-12 | Must | 7/2 = 3.5 — revenue and plan promises depend on enforcement, not on a second catalogue. | U-001, U-038 | (1) An over-quota action returns 402/403 with a stable code and no mutation in the owning service. (2) A plan change unlocks or re-locks the action within 60 seconds without redeploy. (3) An override is audited and expires on its stated date. (4) Two gateway replicas admit exactly the configured quota. (5) A Billing outage fails gated writes closed and serves reads with a visible label. | 2.0 | MVP |
| **U-041** | Rate limiting and abuse controls | CDN/WAF with L3/4/7 and bot management; origin restricted to CDN ranges; connection and body limits; per-user/IP/region limits on a shared counter store; stricter auth, credential-add and manual-sync buckets; 429 with `Retry-After`, audit and metric; certificate expiry alerting. | API Gateway (with edge/WAF) | NFR-SEC-01, 12; CON-031 | Must | 6/2.5 = 2.4 — abuse is a DoS and exchange-ban risk; shared counters are cheap insurance. | U-001, U-055 | (1) With two replicas, requests above the limit produce exactly the configured successes and 429s beyond. (2) A direct origin request outside the CDN ranges is refused. (3) Stricter buckets apply to credential-add and manual-sync; no request body is logged on rejection. (4) Every 429 writes one audit event with hashed actor/IP, route and limit. (5) An expiring certificate alerts before expiry. | 2.5 | MVP |
| **U-042** | SPA core screens | Dashboard (value, freshness, coverage, stale and unpriced labels), portfolio detail with allocation and drift, snapshot history, P&L, source/account list with partial-sync state, transaction list with cost basis, order intent create/cancel, trade recording, briefs inbox, settings, checkout and cancellation. Recharts charts ship a text alternative and data table; converted amounts show original value, FX rate and timestamp. | SPA | NFR-PERF-02, 05; NFR-UX-01, 03; NFR-LEG-08; FR-L10N-02, 03, 04, 07, 12, 13, 14, 19; FR-SUP-02 | Must | 10/6 = 1.67 — the value surface and activation proof; largest single build. | U-001, U-024, U-025, U-026, U-027, U-033, U-034, U-035, U-038, U-039 | (1) The dashboard renders total value with valuation timestamp, freshness state, unpriced/unmapped counts and coverage, all through the gateway. (2) Allocation and drift render with an accessible data table and text alternative; axe reports zero critical or serious findings. (3) A stale price shows a stale badge; an empty portfolio shows a labelled empty state. (4) Creating or cancelling an order intent records the decision and shows "not sent to an exchange". (5) Checkout shows tax status and auto-renewal terms; cancellation takes no more steps than sign-up. | 6.0 | MVP |
| **U-043** | Job orchestration control plane and heartbeat | Versioned schedule catalogue; `pg_cron` tick via `pg_net`; advisory-lock single flight; service-token run calls; durable run records; bounded retry and dead-letter row; admin-only rate-limited manual triggers; status API; run-record retention ≥90 days; minute-level heartbeat consumed by an external dead-man evaluator independent of Supabase and the VPS provider; bounded backfill. | Job Orchestration | FR-JOB-04, 05, 06, 08, 09, 12, 13; NFR-JOB-04, 05; NFR-AVL-02 | Must | 10/6 = 1.67 — silent failure is the named operational risk; a missed valuation day is user-visible and a gate failure. | U-001, U-047, U-055 | (1) Two overlapping triggers of one job: one runs, the other is rejected or deferred, and both attempts are recorded. (2) A forced failure leaves a failed run record, one bounded retry and a dead-letter row with no duplicated side effect. (3) Killing the scheduler produces an external dead-man alert within 5 minutes through a path independent of the cluster. (4) A non-admin trigger is denied; an admin trigger is audited and rate-limited. (5) A schedule change applies without deploy at the next tick; run records survive ≥90 days; logs contain no credentials or full addresses. | 6.0 | MVP |
| **U-044** | Price, FX and benchmark scheduling | Schedule and supervise Market Data run endpoints for 1-minute top-asset collection, 15-minute tail collection, hourly FX, daily close and licensed benchmark series; per-asset/source isolation; freshness publication; licence-gated activation; per-source run counts. | Job Orchestration (with Market Data) | FR-JOB-02, 10, 17, 26; NFR-JOB-01, 02; NFR-PERF-01 | Must | 9/2 = 4.5 — freshness is the whole trust claim and the job is small once the scheduler exists. | U-016, U-017, U-019, U-043 | (1) A top-asset cycle completes under 1 minute and a tail cycle under 15 minutes at CON-051 scale, with per-source counts. (2) One failing asset or source does not stop the cycle; the outcome is `partial`. (3) FX is stored with source and timestamp; a missing rate is never fabricated. (4) A source without a licence reference cannot be activated. | 2.0 | MVP |
| **U-045** | Account, transaction and reconciliation scheduling | Schedule plan-cadenced account and transaction sync and hourly reconciliation; idempotent syncs with imported/skipped counts; per-source partial state; plan cadence; revoked-key detection; reconciliation mismatches surfaced, never auto-corrected; bounded backfill. Scheduling only; execution is owned by U-008, U-009, U-028 and U-031. | Job Orchestration (with Account, Ledger, Portfolio) | FR-JOB-16, 21, 22, 24; BR-JOB-05, 06, 12; NFR-ACC-04 | Must | 8/3 = 2.67 — holdings and ledger correctness underpin every number; a dedupe failure corrupts P&L. | U-008, U-009, U-028, U-031, U-043 | (1) Re-running a transaction window imports zero duplicates and records imported/skipped counts. (2) A Free account is not synced more often than its interval and a Pro account follows its interval, tested with a controlled clock. (3) A revoked key is detected within one cycle and the account shows a recovery action. (4) A ledger-versus-holdings mismatch creates a surfaced record and never mutates stored values. (5) A 24-hour outage is backfilled without duplicates. | 3.0 | MVP |
| **U-046** | Snapshot, brief, billing and retention scheduling | Schedule daily catalogue refresh, per-timezone EOD snapshots including DST, brief generation, alert dispatch through U-052, billing renewal/dunning with daily entitlement reconciliation, analytics aggregation through U-053, and retention/deletion with legal-hold checks; idempotent per key; tenant isolation; billing or notification failure never blocks read-only portfolio access. Scheduling only; execution is owned by the consuming units. | Job Orchestration (with Portfolio, Reporting, Billing, Audit) | FR-JOB-01, 03, 11, 18, 19, 20, 23, 25; BR-JOB-03, 10, 11; NFR-JOB-03, 06; NFR-PRV-14; NFR-LEG-10 | Must | 8/3 = 2.67 — snapshots, billing and retention are daily contractual duties; failure is visible and auditable. | U-027, U-035, U-039, U-043, U-048, U-052, U-053 | (1) A user in each launch timezone receives exactly one EOD snapshot per local date across a DST transition; a rerun updates the same row. (2) One portfolio failure does not stop the others and is recorded with scope. (3) Briefs are idempotent per owner/portfolio/period; a provider failure retries without duplicate delivery. (4) Billing renewal and dunning run daily and reconcile entitlements without blocking read-only access. (5) A retention run deletes only records past schedule and not under legal hold, and records counts. | 3.0 | MVP |
| **U-047** | Append-only audit trail | Insert-only Postgres role (no UPDATE/DELETE); write API for synchronous security events and best-effort domain events with local-log fallback; separate audited privileged retention role; canonical envelope (actor/system, action, target, time, context, outcome, correlation ID); security, account, portfolio, credential, sync, import, order, admin, role, entitlement, support, manual-trigger, notification, billing and privacy events; field redaction; admin query/export audited; 12-month partitioned retention with legal hold. | Audit | FR-USR-06, 21; NFR-SEC-13, 19, 23, 24 | Must | 9/3 = 3.0 — append-only evidence is a launch gate and pen-test target; small build. | U-001, U-055 | (1) The application role can INSERT but not UPDATE or DELETE; a tamper attempt fails at the database. (2) Denied access, rate-limit, support access, role/entitlement change and privacy-request events each carry actor, action, target, time, outcome and correlation ID. (3) A PII scan finds no credential, token or full address. (4) Admin query/export exclude credential fields and the export is itself audited. (5) Retention drops only records older than 12 months; legal-hold records survive. (6) When Audit is unavailable, a security-sensitive gated action fails closed and a domain event falls back to the local log with a documented completeness limit. | 3.0 | MVP |
| **U-048** | Privacy export, erasure and consent | Authenticated, audited export (JSON/CSV, PDF where available) of holdings, transactions, snapshots, P&L, preferences and consent with zero credential content; in-app deletion with per-entity cascade, anonymisation, crypto-shredding and legal-hold override; pending-deletion grace window; consent centre with granular withdrawal; privacy request log; retention hooks shared with U-046. | Audit (with Identity) | FR-USR-14, 18; FR-PRV-01, 04, 05, 08–13, 17, 20; FR-JOB-23; NFR-PRV-03, 04, 08, 13, 14, 15; NFR-SEC-24; NFR-LEG-10 | Must | 9/4 = 2.25 — statutory deadlines and a DPIA gate; medium build touching every store over HTTP. | U-002, U-003, U-027, U-028, U-047, U-055 | (1) An export returns a machine-readable archive with holdings, transactions, snapshots, P&L and preferences, contains zero credential or token fields, and is audited. (2) In-app deletion anonymises identity, crypto-shreds credentials and leaves no orphaned user-linked rows outside the legal-hold set. (3) Withdrawing analytics consent stops non-essential events within one page load and is reflected in the consent record. (4) A legal-hold record survives deletion and is reported in the summary. (5) A request by user B never contains user A data; every request is logged and visible. | 4.0 | MVP |
| **U-049** | Localisation and locale-aware formatting | Externalised ICU strings; English (EU) and English (UK) launch locales with English fallback and no raw keys; locale-aware number, currency, percentage and date formatting; original value retained beside conversion with FX rate and timestamp; timezone-aware grouping; translated UI, errors, onboarding, in-app alerts and email templates; translated legal text per launch locale with counsel review; long-string and 360 px QA; RTL-safe logical CSS. German, French and Spanish are a Should tail. | SPA (cross-cutting) | FR-L10N-01–09, 14, 17, 19, 20; FR-ONB-20; FR-PRV-02; NFR-LEG-01; CON-052 | Must | 8/2.5 = 3.2 — legal-language duties attach per market and unfinished localisation blocks a launch; translation is external. | U-001, U-006 | (1) A user sees locale-aware number, currency and date formatting and the FX timestamp. (2) Changing base currency updates all displays without altering stored values. (3) A missing key falls back to English, never a raw key or blank; layout holds at 360 px with strings 30% longer. (4) Every launch-locale legal document exists with recorded counsel sign-off before that locale is enabled. (5) In-app alerts render in the recipient locale; English-only email is labelled per the notification deviation. (6) Date grouping follows the user's timezone across DST. | 2.5 | MVP |
| **U-050** | Accessibility WCAG 2.2 AA and EN 301 549 | Semantic landmarks, keyboard and focus management, visible focus, contrast-checked Parchment tokens, reduced motion, labelled forms, live regions, accessible charts with text alternatives and data tables, PDF/UA-tagged briefs when PDF ships, accessibility statement and feedback channel, axe in CI and manual screen-reader passes, external-audit remediation to zero blockers. Owns the accessibility half of FR-REP-23, verifying the U-035/U-036 outputs. Target is **DECISIONS D-016 (accessibility)**; the platform risk is **RAID D-016 (platform foundation)**, a different record. | SPA (cross-cutting) | FR-REP-23; FR-L10N-11, 12, 13; NFR-A11Y-01–05; FR-ONB-19; FR-SUP-19; NFR-UX-02; DECISIONS D-016; C13 | Must | 8/3 = 2.67 — the EAA makes this a market-access gate; implementation is small, the external audit is the long pole. | U-001, U-035, U-036, U-042, U-049 | (1) Every launch-critical flow (sign-in, onboarding, dashboard, valuation, transaction list, order intent, settings, deletion) passes axe with zero critical or serious findings and a keyboard-only run. (2) Every chart exposes a text alternative and an equivalent data table. (3) Async sync, error and alert states are announced in a live region without focus theft. (4) Brief content is accessible; the PDF/UA half is delivered only when the Should PDF tail ships. (5) An independent EN 301 549 / WCAG 2.2 AA audit has no open blocking finding; statement and feedback channel are live. | 3.0 | MVP |
| **U-051** | PWA shell for mobile delivery | Web app manifest, versioned service worker and cache, offline app shell with explicit offline and last-updated state, safe activation of new bundles, responsive layout at 360 px with 44 px touch targets, documented iOS/Android browser matrix, no personal-data API response cached; web push out of scope. **Must effort is the shell (1.5); installability and the install prompt are a separate 1.5 Should tail.** | SPA | NFR-UX-01; FR-GEN-14 (Should); FR-L10N-20 | Must (shell) / Should (installability) | 5/1.5 = 3.33 — the PWA is the launch mobile path, but installability is a Should and a browser tab still works. | U-001, U-042, U-049 | (1) Offline start shows the shell with an explicit offline and last-updated state; no stale personal data is served beyond a labelled last-known view. (2) A new deploy activates on next launch through a versioned cache; a breaking API change cannot strand a user. (3) Cache inspection shows no personal-data API response. (4) Every core screen is usable at 360 px with 44 px targets and no horizontal scroll. (5) Installability is the Should tail. | 1.5 | MVP |
| **U-052** | Notification degraded path: alert centre and mandatory email | Q8 degraded path in Reporting: in-app alert centre plus mandatory email for security, credential-integrity and billing events; channel per type (FR-NOT-04); basic per-type enable/disable for optional alerts; one-click unsubscribe for non-mandatory email honoured immediately (FR-NOT-08); idempotent per owner/portfolio/period; retry with dedupe through the job path; delivery log; alert ingestion API. No quiet hours, digests, dedupe policies, web push or provider-independent channel; email is English-only at launch and that limitation is stated. Owns FR-NOT-14 and FR-L10N-15, with U-049 supplying in-app templates. | Reporting (Notification deferred) | FR-NOT-01, 02, 04, 07–14, 16, 17, 19; FR-GEN-13 partial; FR-JOB-18; FR-L10N-15 partial | Must (degraded path) | 7/2.5 = 2.8 — mandatory alerts cannot disappear with the service; the degraded path is small but has a fan-out ceiling. | U-035, U-048, U-049 | (1) Security, credential-integrity and billing alerts cannot be fully disabled; optional types can. (2) A user selects a channel per notification type (FR-NOT-04); mandatory alerts are locked to in-app plus email. (3) One-click unsubscribe for non-mandatory email is honoured immediately and does not suppress mandatory alerts (FR-NOT-08). (4) Each alert is delivered at most once per owner/portfolio/period; a rerun produces no duplicate. (5) The in-app centre tracks unread/read and links to the related view. (6) The delivery log records attempt, provider response class and retry count, with no secret. (7) A burst test stays under the configured send rate; email language is stated as English-only. | 2.5 | MVP |
| **U-053** | Analytics degraded path: consent-aware PII-free events and feature flags | Q8 degraded path in the web tier: versioned event taxonomy; consent check against U-048 before any non-essential event; no PII, addresses or credentials in payloads; fixed metric set (activation, first valuation by path, D30 proxy, conversion, coverage, P&L availability, sync success); daily aggregation through U-046; product and operations dashboards; **feature flags with owner, expiry AND rollout percentage**; **experiment exposure without PII**; retention and deletion; missing/duplicate event checks. | SPA (with Reporting, Analytics deferred) | FR-ANL-01–07, 10, 13, 15; FR-JOB-25; FR-ADM-18; FR-PRV-16; D-017 | Must (degraded path) | 6/2.5 = 2.4 — measurement informs growth, not the first valuation; lower fidelity accepted. | U-006, U-048 | (1) A user without analytics consent produces no non-essential event; withdrawal stops collection immediately. (2) A `valuation_viewed` event contains no email, address, portfolio name or credential. (3) Time-to-first-value is reported by onboarding path and the activation funnel identifies the largest drop-off. (4) Coverage and P&L-availability dashboards are available to product and operations. (5) Every feature flag has an owner, an expiry and a rollout percentage in 0–100%; assignment is deterministic by hashed anonymous subject, no PII is used, and an expired flag cannot be served (FR-ADM-18, FR-ANL-10). (6) Every experiment exposure records experiment key, variant, anonymous subject hash and timestamp, with no PII (FR-ANL-13). (7) Analytics data older than the retention period is deleted. | 2.5 | MVP |
| **U-054** | Observability, status page, runbooks and breach response | Self-hosted Loki/Prometheus/Tempo/Grafana on constrained Swarm nodes; structured logs and correlation IDs; health/readiness; RED, job, heartbeat and business metrics; alerts linked to runbooks; collector redaction and encrypted volumes; external synthetic monitor independent of the VPS provider and Supabase; public status page; SEV-1 paging within 15 minutes; five runbooks (control-plane loss, node loss, Supabase down, disk full, certificate expiry); one rehearsed incident drill; **breach-notification runbook and tabletop with C8 evidence**. | Observability (cross-cutting) | NFR-OBS-01–05; NFR-AVL-01, 03, 06; FR-SUP-09, 15, 20; FR-PRV-15; NFR-PRV-11; C4, C8, C11, C12 | Must | 9/4 = 2.25 — without evidence the 99.9% claim and the SEV-1 duty cannot be met; medium build. | U-043, U-047, U-055 | (1) Every service exposes health, readiness and RED metrics; one dashboard shows latency, error rate, saturation and job health. (2) A node kill and a replica kill produce an alert; the external monitor records the event independently of the cluster. (3) A simulated SEV-1 pages on-call and updates the status page within 15 minutes. (4) Five runbooks exist with a recorded walkthrough and one incident drill is completed before launch. (5) Logs, traces and metrics contain no credentials, tokens or full addresses after redaction. (6) The external monitor is green for seven consecutive days before public beta. (7) A breach-notification runbook covers the statutory window, authority and user notification, processor escalation and evidence retention; a tabletop drill is completed with the C8 owner and its evidence is filed against C8 (FR-PRV-15, NFR-PRV-11). | 4.0 | MVP |

### 2.6 Platform and DevSecOps — U-055…U-058

| Unit | Name | Description | Launch service | BRD requirement IDs | MoSCoW | WSJF (CD/JS = score — rationale) | Depends on | Acceptance criteria (testable) | ew | Release |
|---|---|---|---|---|---|---|---|---|---|---|
| **U-055** | Platform foundation and Swarm topology | IaC of record; staging; three managers in three failure domains; workers and replicas; ingress/CDN/WAF; Vault bootstrap; platform ADRs including CON-011. Owns **RAID D-016 (platform foundation)**, which is distinct from **DECISIONS D-016 (accessibility)** owned by U-050. | Platform | NFR-SEC-01, 02, 03, 15; CON-028, 029, 031; RAID D-016 | Must | 10/6 = 1.67 — every launch gate depends on it; large but standard infrastructure. | none | (1) IaC reconcile is clean and staging deploys and rolls back. (2) Three managers sit in three failure domains and the node-kill drill keeps the dashboard up. (3) The origin is restricted to CDN ranges. (4) Vault bootstrap leaves no plaintext and the rotation/fail-closed tests pass. (5) The CON-011 ADR records `search_path`, `REVOKE ALL`, per-service `current_user`, per-subject keys and fail-closed behaviour. | 6.0 | MVP |
| **U-056** | CI supply chain and release engineering | Private registry; digest pinning; SBOM; dependency and image scanning; signed images; deploy and rollback; no secret in build logs. | Platform | NFR-SEC-14; CON-032 | Must | 8/3 = 2.67 — supply-chain scanning is a gate and a pen-test finding if absent. | U-055 | (1) An unsigned or unscanned image is rejected. (2) Every release carries an SBOM. (3) Rollback is proven. (4) Build logs contain no secret. | 3.0 | MVP |
| **U-057** | Backup, PITR and restore | Encrypted, access-controlled backups; PITR; per-component RTO/RPO; restore drills; backup retention; interaction with crypto-shredding recorded. | Platform | NFR-SEC-16; NFR-AVL-04, 05; CON-036 | Must | 8/3 = 2.67 — C15 is a launch gate; restore is engineering work, not evidence. | U-055 | (1) RPO ≤24 h and RTO ≤4 h are measured in a clean environment. (2) A restore drill is signed by Operations. (3) Backup access is controlled. (4) A deleted credential is unrecoverable after restore, and the key-backup versus shred decision is recorded. | 3.0 | MVP |
| **U-058** | Contract tests and workspace bootstrap | Eleven deployable Cargo workspaces and shared crates; HTTP contract tests for the service-to-service edges (Account, Market Data, Identity, Portfolio, Billing, Reporting, Ledger, Audit); fixtures and the no-cross-schema proof. | Platform | NFR-OBS-06; CON-013, 028 | Must | 7/5 = 1.4 — prevents cross-service drift; broad but mechanical. | U-055 | (1) Every service scaffolds clean. (2) Contract tests cover the named edges and fail on a breaking change. (3) The no-cross-schema proof fails a direct `registry`/`market`/other-service table read. (4) CI is green. | 5.0 | MVP |

## 3. Dependency DAG

**Rules.** Edges run from prerequisite to dependent and are exactly the inversion of the §2 `Depends on` columns. A runtime contract that points the other way is consumed through a stub and is not a build edge; the one-way choices are listed in §3.3. Verified mechanically on the register: **58 units, 167 hard edges, 0 cycles, 0 self-edges, every unit in exactly one layer, every dependency in a strictly earlier layer.**

### 3.1 Hard-edge summary (inversion of the `Depends on` columns)

| Prerequisite | Direct dependents |
|---|---|
| U-001 | U-002, U-003, U-004, U-006, U-008, U-040, U-041, U-042, U-043, U-047, U-049, U-050, U-051 |
| U-002 | U-003, U-004, U-006, U-048 |
| U-003 | U-005, U-007, U-013, U-015, U-019, U-020, U-038, U-048 |
| U-004 | U-008 |
| U-006 | U-049, U-053 |
| U-007 | U-008, U-010, U-011, U-012, U-021, U-024, U-028 |
| U-008 | U-009, U-028, U-045 |
| U-009 | U-024, U-031, U-045 |
| U-010 | U-009 |
| U-011 | U-009 |
| U-012 | U-009 |
| U-013 | U-012, U-014, U-015, U-016, U-019, U-022, U-024, U-028 |
| U-014 | U-007, U-009, U-010, U-015 |
| U-016 | U-018, U-024, U-044 |
| U-017 | U-018, U-024, U-033, U-044 |
| U-019 | U-010, U-016, U-017, U-037, U-044 |
| U-020 | U-021, U-022, U-023, U-024 |
| U-021 | U-024, U-033, U-034 |
| U-022 | U-023, U-026 |
| U-023 | U-026, U-034 |
| U-024 | U-025, U-026, U-027, U-034, U-035, U-042 |
| U-025 | U-042 |
| U-026 | U-027, U-034, U-035, U-042 |
| U-027 | U-035, U-037, U-042, U-046, U-048 |
| U-028 | U-029, U-030, U-031, U-032, U-034, U-045, U-048 |
| U-029 | U-030, U-031, U-032 |
| U-030 | U-031, U-032, U-037 |
| U-031 | U-032, U-045 |
| U-032 | U-033 |
| U-033 | U-035, U-037, U-042 |
| U-034 | U-035, U-042 |
| U-035 | U-036, U-042, U-046, U-050, U-052 |
| U-036 | U-039, U-050 |
| U-037 | U-035 |
| U-038 | U-039, U-040, U-042 |
| U-039 | U-042, U-046 |
| U-040 | U-007 |
| U-042 | U-050, U-051 |
| U-043 | U-025, U-044, U-045, U-046, U-054 |
| U-047 | U-003, U-005, U-006, U-034, U-043, U-048, U-054 |
| U-048 | U-036, U-046, U-052, U-053 |
| U-049 | U-035, U-050, U-051, U-052 |
| U-052 | U-036, U-039, U-046 |
| U-053 | U-046 |
| U-055 | U-001, U-006, U-008, U-041, U-043, U-047, U-048, U-054, U-056, U-057, U-058 |

No prerequisite outside this table has a dependent. U-005, U-015, U-018, U-041, U-044, U-045, U-046, U-050, U-051, U-054 and U-056–U-058 are leaves; they appear as dependents above and own no edge not shown in §2.

### 3.2 Topological layers (every dependency strictly earlier)

L0 (1): `U-055`
L1 (4): `U-001, U-056, U-057, U-058`
L2 (3): `U-002, U-041, U-047`
L3 (4): `U-003, U-004, U-006, U-043`
L4 (6): `U-005, U-013, U-020, U-038, U-049, U-054`
L5 (4): `U-014, U-019, U-022, U-040`
L6 (5): `U-007, U-015, U-016, U-017, U-023`
L7 (7): `U-008, U-010, U-011, U-012, U-018, U-021, U-044`
L8 (2): `U-009, U-028`
L9 (2): `U-024, U-029`
L10 (3): `U-025, U-026, U-030`
L11 (3): `U-027, U-031, U-034`
L12 (3): `U-032, U-045, U-048`
L13 (2): `U-033, U-053`
L14 (1): `U-037`
L15 (1): `U-035`
L16 (1): `U-052`
L17 (1): `U-036`
L18 (1): `U-039`
L19 (2): `U-042, U-046`
L20 (2): `U-050, U-051`

Count check: 1+4+3+4+6+4+5+7+2+2+3+3+3+2+1+1+1+1+1+2+2 = 58. Every unit appears exactly once; every §3.1 edge points from a lower layer to a higher one.

### 3.3 Cycles removed by directional choice

The prior artifact declared some cycles "resolved" while leaving the reverse edge in place. Every row below is now enforced by the §2 `Depends on` columns and the §3.2 layers.

| Former cycle or ambiguity | Directed build edge now used | Reverse edge removed and why |
|---|---|---|
| U-021 ↔ U-034 | U-021 → U-034 | U-034 reads the pending-sell model; U-021 consumes a contract stub, so it does not depend on U-034. |
| U-052 ↔ U-046 | U-052 → U-046 | U-052 owns the alert execution endpoint; U-046 schedules it. The scheduler never precedes its endpoint. |
| U-023 ↔ U-034 | U-023 → U-034 | Guardrails constrain order creation; U-023 does not read order state. |
| U-042 ↔ U-050 | U-042 → U-050 | Accessibility verifies the screens; the screens do not depend on the audit. |
| U-052 ↔ U-049 | U-049 → U-052 | U-049 supplies localized templates; U-052 consumes them. U-049 has no transport dependency. |
| U-027 ↔ U-046 | U-027 → U-046 | Snapshot execution exists before the snapshot schedule; U-027 has no scheduler dependency. |
| U-025 ↔ U-043 | U-043 → U-025 | The read model consumes the job control plane; the control plane has no read-model dependency. |
| U-006 ↔ U-049 | U-006 → U-049 | Onboarding strings and legal text consume the gate/consent record; the gate does not depend on locale packaging. |
| U-006 ↔ U-053 | U-006 → U-053 | Analytics consumes onboarding events; onboarding does not depend on analytics. |
| U-008 ↔ U-055 (C3) | U-055 → U-008 | C3 is the platform spike and Vault gate owned by U-055; U-008 consumes it and does not own it. |
| U-042 ↔ U-039 ↔ U-036 ↔ U-052 | U-036 → U-039, U-052 → U-036, U-035 → U-036 | Checkout consumes the brief channel and alert transport; neither consumes checkout. |
| U-046 ↔ U-053 | U-053 → U-046 | Analytics aggregation runs as a scheduled job; the scheduler depends on the analytics execution unit, not the reverse. |
| U-006 ↔ U-051 (old numbering) | Merged into U-006 | The gate and the onboarding journey are one unit, so the old reverse edge no longer exists. |

## 4. WSJF ranking (top 16; all scores in §2)

| Rank | Unit | Score | Rationale |
|---|---|---|---|
| 1 | U-022 Targets | 8.0 | Drift, briefs and guardrails all consume targets; one engineer-week unblocks three surfaces. |
| 2 | U-030 Transfer pairing | 7.0 | Mispaired transfers fabricate P&L; a one-week bounded build removes the most damaging ledger error. |
| 3 | U-025 Read model | 6.0 | p95 <2 s and the 99.9% claim depend on it; small once valuation exists. |
| 4 | U-020 Portfolio lifecycle | 5.33 | Every portfolio surface hangs off it; 1.5 weeks. |
| 5 | U-026 Drift | 5.33 | The decision surface users return for; cheap once valuation and targets exist. |
| 6 | U-029 Dedupe | 5.33 | Duplicate rows are the credibility failure the product exists to fix. |
| 7 | U-002 Provisioning | 5.0 | Nothing can be owned without the user record. |
| 8 | U-023 Guardrails | 4.67 | Protects users from oversized orders; cheap. |
| 9 | U-021 Membership | 4.5 | D-004 launch rule prevents double counting and double spends. |
| 10 | U-033 P&L | 4.5 | The paid-value proposition. |
| 11 | U-038 Plans and entitlements | 4.5 | Paid launch is impossible without a plan catalogue and entitlements. |
| 12 | U-044 Price/FX scheduling | 4.5 | Freshness is the whole trust claim; small job. |
| 13 | U-031 Reconciliation | 4.0 | Evidences the >99% accuracy claim. |
| 14 | U-036 Brief email | 4.0 | Without a channel the brief is invisible; Q8 makes email mandatory. |
| 15 | U-018 Provenance | 3.6 | Trust depends on staleness labels; contracts over existing data. |
| 16 | U-028 Ledger ingest | 3.6 | P&L is impossible without it. |

WSJF does not order the Bolts. Q2 is risk-first, so U-055, U-001, U-043, U-047 and one adapter/price source run in M0 even though their sizes are large.

## 5. MoSCoW and release registers

### 5.1 Must (committed MVP, 186.0 ew)

U-001–U-058 at Must scope, excluding only the named Should tails in §5.2. This is the full Option B retail slice: aggregation, valuation, allocation/drift, ledger/cost basis/P&L, order intents, snapshots, briefs, billing, auth/MFA, jurisdiction/age gate, audit, scheduling, privacy, performance core, accessibility, platform and observability.

### 5.2 Should fast-follow register (14.5 ew, excluded from 186.0)

| # | Should capability | Unit | BRD refs | ew | Why fast-follow, not Post-MVP |
|---|---|---|---|---|---|
| 1 | xpub/watch-only accounts | U-012 | FR-ACC-21 | 2.0 | A workaround exists (WalletConnect, CSV, public address); named in Q1/Q6 coverage. |
| 2 | Benchmark comparison, attribution, contribution, decomposition, fee drag, export | U-037 | FR-PERF-03, 04, 05, 15, 16, 17 | 1.0 | Core return ships; the tail depends on benchmark rights (C1/CON-040). |
| 3 | PWA installability and install prompt | U-051 | FR-GEN-14 | 1.5 | The mobile shell ships; a browser tab works without installation. |
| 4 | PDF/PDF-UA briefs | U-036 | FR-REP-15, 21 | 1.0 | In-app and email briefs ship; PDF/UA needs the accessibility path. |
| 5 | German, French and Spanish locales | U-049 | FR-L10N-05/06/09/19 under C16 | 1.5 | English (EU) and English (UK) ship; extra locales are a scope change at C16. |
| 6 | Portfolio archive and restore | U-020 | FR-PORT-15, 25 | 1.0 | Delete and lifecycle ship; archive is convenience. |
| 7 | Snapshot and performance CSV/PDF exports | U-027 | FR-VAL-22, FR-PERF-17 | 1.0 | History and drill-down ship; export formats are additive. |
| 8 | Local-currency price display and trial | U-039 | FR-BIL-04, 05 | 1.0 | Checkout, monthly/annual and tax status ship; PPP tiers remain post-MVP. |
| 9 | Venue deep links | U-034 | FR-ORD-14 | 0.5 | Order intent and trade recording ship; the link is additive and legally sensitive. |
| 10 | Optional alert types | U-052 | FR-NOT-05/06 class | 0.5 | Mandatory security, credential and billing alerts ship. |
| 11 | Sub-account parent roll-up | U-007 | FR-ACC-27 | 0.5 | Five account types and quotas ship. |
| 12 | DeFi/staking/LP labelling | U-010 | FR-ACC-26 | 0.5 | Ten-chain native/token balances ship; position labelling is additive. |
| 13 | Watchlist and price thresholds | U-018 | FR-AST-24, 26 | 0.5 | Provenance, stale and unpriced handling ship. |
| 14 | Long-tail DEX pricing | U-016 | FR-AST-20 | 0.5 | Top-asset and tail tiers ship. |
| 15 | Non-core SPA screens | U-042 | NFR-UX-01/03 | 1.0 | The launch-critical screens in §2.5 ship. |
| 16 | Usage-versus-quota UI | U-040 | plan usage UI | 0.5 | Server-side quota enforcement ships. |
| **Total** | | | | **14.5** | |

### 5.3 Post-MVP register (32.0 ew indicative, excluded from every total)

These are **not built**. Their indicative effort is shown only to keep them out of the committed and Should totals; it is not a schedule or a promise.

| # | Post-MVP capability | BRD refs | Indicative ew |
|---|---|---|---|
| 1 | Portfolio templates, duplication and tags/views | FR-PORT-16, 22, 26 | 2.0 |
| 2 | Manual rebalancing proposal with projected trades | FR-PORT-23 | 1.5 |
| 3 | Partial fills | FR-ORD-15 | 2.0 |
| 4 | Direct execution via exchange APIs or on-chain signing | FR-ORD-16 | 5.0 |
| 5 | Slack/Notion or external brief publication | FR-REP-25 | 2.0 |
| 6 | On-chain transaction decoding | FR-TXN-21 | 8.0 |
| 7 | Tax lots and specific-ID lot relief | FR-TXN-22 | 3.5 |
| 8 | Volatility and maximum drawdown | FR-PERF-10 | 1.5 |
| 9 | Sharpe and Sortino | FR-PERF-11 | 1.5 |
| 10 | Coupons, referrals and promotional entitlements | FR-BIL-13 | 1.5 |
| 11 | App-store (IAP) billing | FR-BIL-16 | 2.5 |
| 12 | Purchasing-power pricing tiers | FR-BIL-04 (PPP half) | 1.0 |
| **Total** | | | **32.0** |

### 5.4 Deferred service register (Q8) and coverage

| Deferred service | Must requirements affected | Degraded path and coverage | Units |
|---|---|---|---|
| **Notification** | BRD-015's 14 Musts (FR-NOT-01, 02, 04, 07–14, 16, 17, 19); FR-GEN-13; BRD-009 delivery; FR-JOB-18 | In-app alert centre plus mandatory email for security, credential and billing events; channel per type (FR-NOT-04); one-click unsubscribe (FR-NOT-08); per-type on/off for optional alerts; retry, dedupe and delivery log. **13/14 fully met; FR-NOT-14 partial** (email English-only). **FR-GEN-13 partial** (no web push; FR-NOT-03 is Should). | U-052, U-046 |
| **Analytics** | BRD-021's 10 Musts; FR-GEN-20; FR-JOB-25; FR-ADM-18 | Consent-aware PII-free events, fixed metrics, daily aggregation, dashboards, feature flags with owner, expiry **and rollout percentage**, experiment exposure without PII, retention deletion. **All 10 met**, including FR-ANL-10 and FR-ANL-13. No experiment platform. | U-053, U-046, U-048 |
| **Support Console** | BRD-019's 14 Musts; FR-GEN-18 | Time-boxed, second-approver, reason-recorded, audited break-glass read-only view; external help desk; status page. **All 14 met in degraded form.** FR-ADM-14 is owned by U-003 and FR-ADM-15 by U-005; they are **not** part of this deferral. | U-005, U-054, U-047 |

**Deferred-Must wording.** 386/386 Must rows are bound to a unit or gate owner; **4 are partial**; **0 are wholly deferred**. Three services are deferred at service level, but every one of their Must requirements has a named launch-time degraded path.

### 5.5 Partial Must register (approval-gate acceptance)

| # | Partial Must | Units | What is missing | Acceptance owner |
|---|---|---|---|---|
| 1 | FR-GEN-13 (web push) | U-052 | In-app and email exist; web push is absent because Notification is deferred. FR-NOT-03 is Should. | Product |
| 2 | FR-NOT-14 (localized notifications) | U-052, U-049 | In-app alerts are localized; launch email is English-only. | Product + Compliance |
| 3 | FR-L10N-15 (localized notifications) | U-052, U-049 | Same limitation as FR-NOT-14; recorded separately because it is a BRD-018 Must. U-049 supplies templates; U-052 owns delivery. | Product + Compliance |
| 4 | FR-REP-23 (accessible PDF brief) | U-050 (verifier), U-035/U-036 (producers) | In-app brief is accessible; the PDF half is a Should fast-follow and is not shipped at launch. | Product |

## 6. Risk-first Bolt sequence (Q2)

Six Bolts spanning 46 weeks (10.7 months), about 31 engineer-weeks each at five engineers and 80% capacity (six × 31 ≈ 186); B0–B4 are eight weeks and B5 is the six-week hardening window. The Bolt sequence is the global execution order; §3.2 is the dependency proof. Every unit ID appears at least once, and large units appear as a slice and again as a remainder.

| Bolt | Weeks | Units (full or slice) | Definition of done and hypothesis | Checkpoint |
|---|---|---|---|---|
| **B0 — Walking skeleton** | 0–8 | U-055 (slice), U-056 (slice), U-058 (slice), U-001, U-002, U-047, U-043 (slice), U-054 (slice), U-013, U-014 (slice), U-020, U-024 (fixture), U-025 (fixture), U-006 (gate slice) | Gateway trust boundary, fixture valuation, Vault read/rotate/fail-closed, one exchange adapter fixture, one price/FX fixture, audit insert-only, scheduler heartbeat/dead-man, node-kill drill and seven-day monitor. Hypothesis: the architecture and Vault path survive contact with the platform. | **M0 go/no-go week 8.** Kill if C3 is unsafe, C1 has no viable path, or the fixture valuation cannot reproduce exactly. |
| **B1 — Gates and first real user** | 9–16 | U-003, U-004, U-005, U-040, U-041, U-015, U-016, U-017, U-019, U-045, U-049, U-006 (remainder) | First real user behind the fail-closed gate; one read-only venue synced; MFA; entitlements; first valuation; audit trail; C8 evidence. Hypothesis: one venue and one price source prove provider auth and the licence path. | C8 sign-off before any real user; C3 evidence complete. |
| **B2 — Coverage** | 17–24 | U-007, U-008, U-009, U-010 (EVM family), U-011, U-012, U-014 (remaining), U-018, U-021, U-022, U-023, U-026, U-044 | Five venues, ten chains, WalletConnect, CSV, multi-source prices, FX, attribution and the licence activation gate; targets and drift. Hypothesis: the venue/chain matrix is implementable within the estimates; staging catches adapter gaps. | C1 written rights for every active source; coverage matrix signed off. |
| **B3 — Ledger and privacy** | 25–32 | U-010 (remaining chains), U-028, U-029, U-030, U-031, U-032, U-048, U-049 (remaining), U-053 | Ledger ingest, dedupe, transfer pairing, reconciliation, cost basis; privacy centre; analytics consent and flags. Hypothesis: dedupe and reconciliation hold at >99% on fixture and live data. | Reconciliation accuracy test ≥99%; DPIA evidence; FR-ADM-18/FR-ANL-13 tests green. |
| **B4 — Differentiator and retention** | 33–40 | U-027, U-033, U-034, U-035, U-036, U-037, U-038, U-039, U-042, U-046, U-050, U-051, U-052 | P&L, snapshots, order intents, briefs, billing, performance core, degraded Notification path and core screens. Hypothesis: the paid-value loop closes and the Q8 degraded paths carry the deferred Musts. | C9/C10 before paid plans; FR-NOT-14 and FR-GEN-13 acceptance recorded. |
| **B5 — Hardening and launch** | 41–46 | U-054 (full), U-055–U-058 (remaining, incl. U-057), pen-test/accessibility/restore remediation, buffer | All gates green: C4, C11, C12, C13, C14, C15, C16; status page, runbooks, breach tabletop and incident drill; seven-day external monitor; cut-list buffer. Hypothesis: no launch gate is red and the 10.7-month plan holds. | Launch gate review; any red gate blocks release. |

**Streams.** Two streams run through every Bolt: backend services (Rust/Clean Architecture, Postgres, adapters) and SPA plus platform (React, design system, IaC, observability). **Splitting rule:** U-001, U-008, U-010, U-042, U-043 and U-055 exceed 5 engineer-weeks; they split into ≤5 engineer-week slices during Inception with a traceability note, never silently renumbered.

## 7. Written cut list and the 9-month path

The committed baseline already excludes **14.5 engineer-weeks** of Should work (§5.2). That is the first written cut list required by C5; it removes no Must.

Reaching 9 months requires cutting a further **30.0 engineer-weeks** (186.0 − 156.0). The table below is a single additive set that closes the gap exactly. Each row is unique: no unit slice appears twice, U-010 and U-056 are not duplicated, and the total is the literal row sum.

| # | Cut | ew saved | Confirmed decision breached |
|---|---|---|---|
| 1 | Defer order intents and trade recording (U-034) | 2.5 | Q1 |
| 2 | Defer briefs and the degraded notification path (U-035 2.5 + U-036 1.5 + U-052 2.5) | 6.5 | Q1 and Q8 |
| 3 | Defer billing entitlements and checkout (U-038 2.0 + U-039 3.0 + U-040 2.0) | 7.0 | Q1 |
| 4 | Defer immutable snapshots (U-027) | 2.5 | Q1 |
| 5 | Defer WalletConnect (U-011) | 3.0 | Fixed coverage |
| 6 | Defer CSV import (U-012 Must part) | 2.0 | Fixed coverage |
| 7 | Reduce venues from five to three (U-008 slice) | 3.0 | Fixed coverage |
| 8 | Reduce chains from ten to six (U-010 + U-014 slices) | 3.5 | Fixed coverage |
| **Total** | | **30.0** | — |

Check: 2.5 + 6.5 + 7.0 + 2.5 + 3.0 + 2.0 + 3.0 + 3.5 = **30.0**, exactly the gap. This set deliberately does not cut a launch gate (C13 accessibility, C11 operations, C14 pen test), because a gate cut does not produce a launch.

**Conclusion.** The plan of record is **10.7 months at five engineers** (186.0 ew), inside Q6's confirmed 10–11 month envelope. The 9-month target at five engineers is not achievable without the 30.0 ew cut set above, and every row in that set removes a Q1-confirmed capability or fixed launch coverage. The gate may instead choose (a) **seven engineers for 9 months** (7 × 9 × 80% = 50.4 engineer-months > 42.8 required), or (b) a **scope change** that reopens Q1 or fixed coverage. No unquantified optimism replaces the cut list.

## 8. Traceability summary

**Result: 386 Must requirement rows across BRD-001…BRD-021 are bound to a unit or a named gate/process owner. 0 unmapped; 4 partial Musts with documented degraded paths and gate acceptance; 0 wholly deferred Musts.** The three deferred services are a service-level deviation, not a requirement-level gap: every Must has a launch-time degraded path.

### 8.1 BRD-001 Must mapping (the sixteen product-wide Musts)

| Must ID | Capability | Unit | Status |
|---|---|---|---|
| FR-GEN-01 | Multiple accounts combined into portfolios | U-007–U-012, U-020 | Met |
| FR-GEN-02 | One consolidated view per portfolio in base currency | U-024 | Met |
| FR-GEN-03 | Actual allocation vs target | U-026 | Met |
| FR-GEN-04 | Order intents; recorded trade affects allocation immediately; never places orders | U-034 | Met |
| FR-GEN-05 | Immutable snapshots and history | U-027 | Met |
| FR-GEN-07 | More than one portfolio | U-020 | Met |
| FR-GEN-10 | Never require withdrawal permissions | U-008 | Met |
| FR-GEN-11 | Transactions, cost basis, net invested and P&L | U-028–U-033 | Met |
| FR-GEN-12 | User-selected base currency, locale and timezone | U-002, U-049 | Met |
| FR-GEN-13 | In-app, email and web-push notifications with preferences | U-052 | **Partial**: in-app and email; no web push |
| FR-GEN-15 | Launch-locale localisation and WCAG 2.2 AA | U-049, U-050 | Met |
| FR-GEN-16 | Plans, entitlements and quotas enforced server-side | U-038, U-040 | Met |
| FR-GEN-17 | User data export and in-app deletion | U-048 | Met |
| FR-GEN-18 | Help centre, support channel and status page | U-005, U-054 | Met (degraded support path) |
| FR-GEN-19 | Jurisdiction gate and legal disclosures | U-006, U-049 | Met |
| FR-GEN-20 | Consent-aware analytics | U-053 | Met (degraded analytics path) |

### 8.2 BRD coverage

| BRD | Must rows | Owning units | Coverage |
|---|---|---|---|
| BRD-001 Product vision | 16 | U-002, U-005–U-012, U-020, U-024, U-026–U-028, U-033, U-034, U-038, U-040, U-048–U-050, U-052–U-054 | 16/16; FR-GEN-13 partial |
| BRD-002 Users, roles | 20 | U-001–U-006, U-048 | 20/20 |
| BRD-003 Authentication | 19 | U-001–U-004, U-006, U-005, U-048 | 19/19 |
| BRD-004 Accounts | 27 | U-007–U-012 | 27/27 |
| BRD-005 Assets and market data | 18 | U-013–U-019 | 18/18 |
| BRD-006 Portfolios | 19 | U-020–U-023, U-026 | 19/19 |
| BRD-007 Valuation and snapshots | 19 | U-024–U-027 | 19/19 |
| BRD-008 Orders | 16 | U-021, U-034 | 16/16 |
| BRD-009 Reporting | 15 | U-035, U-036, U-050 | 15/15; FR-REP-23 partial (PDF half) |
| BRD-010 Admin | 16 | U-003, U-005, U-014, U-015, U-017, U-019, U-038, U-040, U-053 | 16/16; FR-ADM-18 met by U-053 with rollout percentage |
| BRD-011 Background jobs | 22 | U-015, U-016, U-025, U-027, U-028, U-031, U-039, U-043–U-046, U-048, U-052, U-053 | 22/22; FR-JOB-25 met by U-053 |
| BRD-012 Security/NFRs | 49 | U-001, U-040–U-043, U-047, U-048, U-054, U-055–U-058, service units, gate owners | 49/49; range NFRs cited as ranges |
| BRD-013 Transactions/P&L | 17 | U-028–U-033 | 17/17 |
| BRD-014 Onboarding | 16 | U-001, U-006, U-007–U-012, U-049, U-050 | 16/16 |
| BRD-015 Notifications | 14 | U-052, U-046 | 14/14; FR-NOT-14 partial (email English-only) |
| BRD-016 Billing | 13 | U-038, U-039, U-040 | 13/13 |
| BRD-017 Privacy | 19 | U-046–U-049, U-053, U-054 | 19/19 |
| BRD-018 Localization/a11y | 17 | U-001, U-006, U-042, U-049, U-050, U-052 | 17/17; FR-L10N-15 partial |
| BRD-019 Support/status | 14 | U-005, U-050, U-054 | 14/14 (degraded path) |
| BRD-020 Performance | 10 | U-019, U-037 | 10/10 |
| BRD-021 Analytics | 10 | U-048, U-053, U-054 | 10/10 (degraded path) |
| **Total** | **386** | — | **386/386; 4 partial; 0 wholly deferred** |

**Partial Must register:** see §5.5 (FR-GEN-13, FR-NOT-14, FR-L10N-15, FR-REP-23). No other Must is partial.

### 8.3 Gate/process, not a unit

These Must IDs intentionally have no build unit. They are neither orphaned nor deferred; the named gate or process owner produces the evidence.

| Must IDs | Owner |
|---|---|
| FR-PRV-03 (lawful basis), FR-PRV-06 (subprocessor/DPA), FR-PRV-07 (residency/transfer), NFR-PRV-06, NFR-PRV-07, NFR-PRV-10, NFR-SEC-25 | C8 Compliance |
| NFR-LEG-03, NFR-LEG-04 (copy and marketing review) | C2/C9 Legal |
| NFR-SEC-20 (penetration test) | C14 DevSecOps |
| NFR-SEC-02, NFR-SEC-03, NFR-SEC-15 (Vault and secret-handling evidence) | U-055 with U-047/U-054 redaction evidence |
| NFR-SEC-17 (session/device evidence) | U-004 |
| FR-PRV-19 (data minimisation in logs) | U-047 |
| FR-PRV-15 and NFR-PRV-11 (breach notification) | U-054 runbook and C8 tabletop evidence |
| NFR-PRV-01/02/04/05/08/09/13/14/15 | U-048 except NFR-PRV-02, which is U-049 |

**Notation rule.** In §2, a requirement list is family shorthand: after the first fully qualified ID, bare two-digit numbers and slash lists inherit the same prefix (`FR-ACC-01, 03, 06` means FR-ACC-01, FR-ACC-03 and FR-ACC-06; `NFR-SEC-05/06` means both IDs; `FR-VAL-07–13` means every ID in the range). Range-defined NFRs (NFR-ACC-01–04, NFR-PERF-01–05, NFR-AVL-01–07, NFR-SCL-01–04, NFR-OBS-01–06, NFR-UX-01–03, NFR-A11Y-01–05) are cited as ranges, not fabricated individual IDs.

## 9. Risks and open questions owned by this backlog

| # | Risk / open question | Impact | Owner | Handling |
|---|---|---|---|---|
| 1 | R-010 decimal overflow: `rust_decimal`'s 96-bit mantissa overflows on raw uint256 balances above ~79 billion 18-decimal tokens | Plausible-looking wrong balances and valuations | Architect + Account/Ledger | U-009 and U-024 parse `u128`/bigint at the adapter boundary, scale after, and carry a large-balance and golden-vector test. |
| 2 | CON-026 shared per-venue/per-IP exchange bucket has no named owner | Per-service limiters exhaust the shared egress IP and trigger a venue ban | Account + Architect | U-008 owns the bucket; a dedicated egress proxy and a two-replica proof are acceptance criteria. |
| 3 | CON-011 Vault platform exception is proposed, not yet recorded | U-008's credential path has no approved storage design; C3 fails | Architect | M0 ADR in U-055; pinned `search_path`, `REVOKE ALL`, per-service `current_user`, per-subject keys, fail-closed and backup-versus-shred decisions. |
| 4 | Licence rights are per use, not per source | A display-only source could be activated while Portfolio stores history and publishes derived valuations | Compliance + Architect | U-019 records rights per display/storage/history/derivation/region and fails closed; C1 requires written quotes. |
| 5 | Tron has no named RPC/indexer provider | Ten-chain coverage is not evidence-backed | Architect | C1 requires a named Tron provider; U-010 depends on it. |
| 6 | Region-gated venues (OKX, Binance, Bybit) | Coverage claims and onboarding paths vary by region | Product + Compliance | U-007 and U-008 expose a region-aware availability matrix with reason codes. |
| 7 | Cross-service contracts are unowned | Account holdings, Market Data provenance and Identity principal-resolution APIs drift | Architect | U-058 owns the contract-test harness; U-001 and U-040 own gateway contracts; U-002 owns principal resolution. |
| 8 | Two different records share the label D-016 | "Platform D-016" and "accessibility D-016" can be conflated in scope, budget and evidence | DevSecOps + Compliance | **RAID D-016 (platform foundation)** is owned by U-055–U-058; **DECISIONS D-016 (accessibility)** is owned by U-050. Every reference in this artifact names the register. |
| 9 | FR-L10N-15 / FR-NOT-14 email localisation is partial | A Must is not fully met at launch | Product + Compliance | Recorded in §5.5 and §8; Product accepts at the gate or reinstates Notification localisation. |
| 10 | Large units exceed small-batch sizing (U-001, U-008, U-010, U-042, U-043, U-055) | A whole Bolt can be consumed by one unit; no room to react | Delivery | Split into ≤5 engineer-week slices during Inception; capability boundaries and IDs do not change. |
| 11 | Benchmark licence failure | FR-PERF-03's benchmark source cannot be shown | Product + Compliance | U-037 ships the performance core without benchmarks; the benchmark tail is a Should fast-follow and is hidden if C1 fails. |
| 12 | Cut list cannot close to 9 months | Stakeholders may expect 9 months | Product + Delivery | §7 states 10.7 months at five engineers, or seven engineers, or a Q1 scope change. C5 approval must pick one. |
| 13 | External help desk processes ticket personal data | Processor/DPA/transfer map and C8 evidence are incomplete if the help desk is treated as a tool | Compliance | U-005's help-desk provider is added to the CON-037 processor map, DPA register and transfer mechanism before the first real user; C8 owns the evidence. |
| 14 | pg_cron/pg_net saturation at the one-minute cadence | Overlapping ticks or queue saturation can delay price, FX and snapshot jobs | Architect + Job Orchestration | U-043 adds a one-minute all-jobs load test with a bounded queue and timeout criterion. |
| 15 | Breach-notification path is untested | FR-PRV-15 / NFR-PRV-11 could fail in the statutory window | Compliance + Operations | U-054 carries the runbook, tabletop and C8 evidence as acceptance criterion 7. |

## 10. Verification notes and critic-finding closure

**Unit integrity.** 58 units, IDs U-001…U-058 with no gaps and no duplicates. Every unit names a primary launch service, a MoSCoW value, a WSJF score, its hard dependencies, testable acceptance criteria, committed Must effort and a release category.

**Effort integrity.** §2 rows sum to **186.0 ew**; cluster rows in §1.3 sum to the same 186.0; the Should register §5.2 sums to **14.5 ew**; the Post-MVP register §5.3 sums to **32.0 ew** and is excluded from every total. The reconciliation §1.1 is literal: 81.0 + 47.5 + 74.0 + 17.0 = 219.5; 219.5 − 6.0 − 13.0 = 200.5; 200.5 − 14.5 = 186.0.

**Capacity integrity.** 9 months = 156.0 ew; 11 months = 190.7 ew; 186.0 ew ≈ 42.9 em = 10.7 months. Gap to 9 months = 30.0 ew; headroom at 11 months = 4.7 ew. The former 192.0 total was 6.0 ew over the feasibility upper bound (186) and 1.3 ew over 11-month capacity (190.7); the merge removed exactly that 6.0 ew of duplicated effort.

**DAG integrity.** 58/58 units placed, 167 hard edges, 0 cycles, 0 self-edges, every dependency in a strictly earlier layer. §3.1 is the exact inversion of the §2 `Depends on` columns; §3.2 is the resulting longest-path layering.

**Release integrity.** Mixed units are split explicitly in §1.4; the Must part is in the 186.0 and the Should part in the 14.5, never both. xpub/watch-only, PDF/PDF-UA, benchmark attribution, extra EU locales and PWA installability are **Should fast-follow**, not Post-MVP; direct execution, DeFi accounting, NFTs, native apps and tax filing are **Post-MVP**.

**Critic-finding closure.**

| # | Finding | Closure in this artifact |
|---|---|---|
| 1 | FR-ADM-18 / FR-ANL-10 / FR-ANL-13 unrecorded or untested partial Musts | U-053 acceptance criteria 5 and 6 add owner + expiry + **rollout percentage**, deterministic hashed assignment with no PII, and PII-free experiment exposure; all three are recorded as met, not partial. |
| 2 | 192 ew outside the envelope | Trimmed to 186.0 by removing 6.0 ew of duplicate-unit effort; 192 vs 186 and 190.7, 9-month 156, 11-month 190.7, 30.0 gap and 10.7-month plan are all explicit in §1 and §7. |
| 3 | Cycles and one-way edges | §3.3 lists every directed choice; §3 verifies 0 cycles mechanically. |
| 4 | Layers omitted U-004 and had 61 invalid edges | §3.2 places all 58 units once, with every dependency strictly earlier. |
| 5 | Hard-edge summary did not match rows | §3.1 is the exact inversion of the register; 167 edges, no asserted edge absent from §2. |
| 6 | U-051 / U-012 double-counting | §1.4 gives PWA shell 1.5 Must + 1.5 Should and CSV 2.0 Must + 2.0 xpub Should; both Must parts are in the 186.0. |
| 7 | −15.5 vs −14.5 arithmetic | §1.1 uses 14.5, matching the 16 itemised slices; the total is reproducible. |
| 8 | Cut list 45.0 vs 46.5 and duplicate rows | §7 is one unique additive set summing literally to the 30.0 gap; no unit slice repeats. |
| 9 | Post-MVP vs Should conflation | §1 release taxonomy and §5.2/§5.3 separate them explicitly. |
| 10 | Duplicate units | U-001 (old 001+040+043), U-005 (old 005+058), U-006 (old 006+045+051) merged; all 58 IDs renumbered sequentially and §5/§7/§8 references updated. |
| 11 | Partial-Must ownership | FR-REP-23 assigned to U-050 with U-035/U-036 producers; FR-NOT-14 and FR-L10N-15 assigned to U-052 with U-049 templates; FR-ADM-14 owned by U-003, FR-ADM-15 by U-005, not by the Support Console deferral. |
| 12 | "0 deferred Must" contradiction | Reworded to "386/386 bound; 4 partial; 0 wholly deferred" in §5.4 and §8; BRD-015 shows 14/14 with FR-NOT-14 partial; FR-NOT-04 and FR-NOT-08 acceptance criteria added to U-052. |
| 13 | Breach notification had no runbook/test | U-054 acceptance criterion 7 adds the runbook, tabletop drill and C8 evidence for FR-PRV-15 / NFR-PRV-11. |
| 14 | D-016 ambiguity | §9 risk 8 and the U-050/U-055 rows disambiguate RAID D-016 (platform foundation) from DECISIONS D-016 (accessibility). |
| 15 | Scope-document table vs backlog register | §5.4 is the authoritative register. The scope document now matches it: Should fast-follow is separate from Post-MVP, FR-ADM-14/15 are owned by U-003/U-005 rather than the Support Console deferral, and the effort figures are 186.0 ew / 10.7 months. |

**Scope-document reconciliation.** `scope-document.md` now matches this artifact: 186.0 ew / 10.7 months, Should fast-follow separated from Post-MVP, FR-ADM-14/15 owned by U-003/U-005, and the deferred-service table citing §5.4. This backlog is self-consistent and the two artifacts agree.
