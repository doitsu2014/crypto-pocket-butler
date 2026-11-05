# BRD-004 — Account Management & Coverage

Draft v1.2 · Product/Data · 2026-11-04. How users connect read-only holdings sources, how those connections are secured and synced, and which venues and chains the product covers globally. v1.2 adds the coverage matrix, key-permission verification, WalletConnect/xpub, CSV import, spam filtering, and DeFi position labelling.

**In scope:** create/edit/deactivate/delete accounts · exchange accounts with verified read-only credentials · on-chain accounts by public address across EVM, Solana, Bitcoin, and Tron · WalletConnect and watch-only/xpub imports · CSV imports · sub-accounts · single and bulk sync · normalised balances, status, error reporting · spam/dust filtering · DeFi/staking position labelling · quotas.
**Out of scope:** withdrawals/transfers/trading · private-key import · realtime streaming · full DeFi position accounting (post-MVP) · NFT valuation (post-MVP) · exchange OAuth where the exchange does not offer it.

## Coverage matrix (launch target)

| Venue | Type | Launch | Regional notes |
|-------|------|--------|----------------|
| OKX | Exchange | Yes | Where available; check API terms |
| Binance | Exchange | Yes | Ex-US; region restrictions apply |
| Coinbase | Exchange | Yes | US/EU; API availability by account type |
| Kraken | Exchange | Yes | US/EU |
| Bybit | Exchange | Yes, where available | Region-gated |
| Bitget / KuCoin | Exchange | Post-MVP | Region-gated |
| EVM chains | On-chain | Yes | Ethereum, Base, Arbitrum, Optimism, Polygon, BNB, Avalanche |
| Solana | On-chain | Yes | SOL and SPL |
| Bitcoin | On-chain | Yes | Native addresses and xpub watch-only |
| Tron | On-chain | Yes | TRX and TRC-20, especially USDT |
| Additional chains | On-chain | Post-MVP | Demand-driven, registry-configured |

Coverage is a **product commitment by region**, not a marketing claim. A venue unavailable in the user's region is not offered, and the absence is explained.

## Account types

Each account owns its holdings; portfolio membership is defined in BRD-006.

| Type | Data | Balances from |
|------|------|---------------|
| **Exchange account** | Exchange, API key/secret/passphrase — user-created, **verified read-only** | Exchange REST API |
| **On-chain account** | Public address + enabled chains — never private keys | Blockchain JSON-RPC |
| **WalletConnect account** | Read-only wallet session or public address | Wallet provider / RPC |
| **Watch-only account** | xpub or public address | RPC / indexer |
| **Imported account** | CSV transaction/holdings file | Import pipeline (BRD-013) |

Same asset across multiple chains is merged for portfolio math; chain detail is retained.

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-ACC-01 | Create an exchange account with name and read-only credentials. | Must |
| FR-ACC-02 | Create an on-chain account with a public address and ≥1 enabled chain. | Must |
| FR-ACC-03 | Validate type and required fields before saving. | Must |
| FR-ACC-04 | Encrypt exchange credentials at rest. | Must |
| FR-ACC-05 | Never return credential fields in API responses. | Must |
| FR-ACC-06 | Edit name, credentials, and enabled chains. | Must |
| FR-ACC-07 | Editing credentials re-encrypts and records the change. | Must |
| FR-ACC-08 | Activate/deactivate; inactive accounts excluded from sync and aggregation. | Must |
| FR-ACC-09 | Delete removes portfolio links, not portfolios. | Must |
| FR-ACC-10 | Sync one account on demand. | Must |
| FR-ACC-11 | Bulk-sync all active accounts with per-account results. | Must |
| FR-ACC-12 | Sync replaces stored holdings and updates last-synced time. | Must |
| FR-ACC-13 | Sync filters zero balances and normalises to decimals. | Must |
| FR-ACC-14 | Per-account failure reporting; bulk sync continues. | Must |
| FR-ACC-15 | Show holdings count and last-synced time per account. | Must |
| FR-ACC-16 | Record last sync attempt outcome (success/failure + reason). | Must |
| FR-ACC-17 | Offer only exchanges available in the user's region; explain unavailable venues. | Must |
| FR-ACC-18 | Verify exchange key permissions at save and periodically; reject withdrawal or trading permissions. | Must |
| FR-ACC-19 | Detect revoked, expired, or permission-changed keys and mark the account inactive with a mandatory alert. | Must |
| FR-ACC-20 | Connect a wallet through WalletConnect or an equivalent read-only session. | Must |
| FR-ACC-21 | Add a watch-only xpub or address for Bitcoin and supported UTXO chains. | Should |
| FR-ACC-22 | Import holdings and transactions from CSV with a template and mapping preview. | Must |
| FR-ACC-23 | Sync native Bitcoin addresses without exposing keys. | Must |
| FR-ACC-24 | Sync Tron addresses and TRC-20 balances, including USDT. | Must |
| FR-ACC-25 | Hide spam and dust tokens by default; never value them; let the user reveal or report them. | Must |
| FR-ACC-26 | Detect and label DeFi, staking, and LP positions as supported, unsupported, or unpriced. | Should |
| FR-ACC-27 | Support exchange sub-accounts and reconcile them to one parent account. | Should |
| FR-ACC-28 | Enforce per-plan account, sync-frequency, and RPC quotas server-side. | Must |
| FR-ACC-29 | Honour exchange API terms and market-data licensing; never redistribute restricted data. | Must |
| FR-ACC-30 | Run sync at the cadence allowed by the plan, with a manual re-sync. | Must |

## Business rules

| ID | Rule |
|----|------|
| BR-ACC-01 | An account belongs to exactly one user and owns its own holdings. |
| BR-ACC-02 | Exchange accounts need name + complete credentials; on-chain accounts need an address. |
| BR-ACC-03 | Quantities are stored as normalised decimals, never raw on-chain integers. |
| BR-ACC-04 | Token decimals are applied on conversion; unknown decimals are a sync error, never a guess. |
| BR-ACC-05 | Available + frozen = total for exchange holdings; frozen ≤ total. Venue-specific buckets are mapped explicitly. |
| BR-ACC-06 | Inactive accounts don't sync and don't contribute holdings. |
| BR-ACC-07 | Deleting an account cascades only to its portfolio links and sync metadata. |
| BR-ACC-08 | Sync is idempotent per point in time — no duplicate holdings. |
| BR-ACC-09 | Credentials are never logged, returned, or included in errors. |
| BR-ACC-10 | Malformed on-chain addresses are rejected before any network call. |
| BR-ACC-11 | A key is trusted only after its permissions are verified read-only; the user's claim is not enough. |
| BR-ACC-12 | A revoked or over-permissioned key is deactivated immediately and the user is alerted. |
| BR-ACC-13 | Spam/dust tokens are hidden and excluded from valuation; they are never silently deleted from the source of truth. |
| BR-ACC-14 | DeFi/staking/LP positions are labelled; the product never values an unsupported position as a plain token balance. |
| BR-ACC-15 | Sub-accounts roll up to one parent; double-counting is forbidden. |
| BR-ACC-16 | Quotas are enforced server-side and shown before they are hit. |
| BR-ACC-17 | Exchange availability and data licensing are enforced per region. |
| BR-ACC-18 | CSV imports are idempotent and reconcile against synced holdings (BRD-013). |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-ACC-01 | Creating an exchange account saves encrypted credentials and returns no secret. |
| AC-ACC-02 | An on-chain account saves with its address and selected chains. |
| AC-ACC-03 | Missing exchange credentials fail validation with a specific error. |
| AC-ACC-04 | A failing exchange auth is reported per-account; other accounts still sync. |
| AC-ACC-05 | A successful sync stores normalised, zero-filtered holdings and updates last-synced. |
| AC-ACC-06 | The same asset on two enabled chains appears once with the combined quantity. |
| AC-ACC-07 | An inactive account's holdings are excluded from allocation. |
| AC-ACC-08 | Deleting a linked account removes the link; the portfolio remains. |
| AC-ACC-09 | No API response contains a credential field or secret value. |
| AC-ACC-10 | A withdrawal-enabled key is rejected with a read-only key instruction. |
| AC-ACC-11 | A revoked key marks the account inactive and sends a mandatory alert. |
| AC-ACC-12 | A WalletConnect wallet adds a read-only account without exposing keys. |
| AC-ACC-13 | A CSV import previews mappings and imports without duplicates. |
| AC-ACC-14 | A Bitcoin address and a Tron address sync their native balances. |
| AC-ACC-15 | A spam token is hidden by default and excluded from valuation. |
| AC-ACC-16 | A staking position is labelled unsupported or unpriced, not counted as a liquid token. |
| AC-ACC-17 | A sub-account rolls up to the parent without double-counting. |
| AC-ACC-18 | A Free user exceeding the account quota is blocked server-side. |
| AC-ACC-19 | An exchange unavailable in the user's region is not offered. |

## Notes

- RPC endpoints are configured by an Admin (BRD-010); the token registry drives on-chain account queries (BRD-005/010).
- Rate limits may require batching/backoff. Holdings are cached snapshots, not a transaction ledger; the ledger is BRD-013.
- Exchange API terms and market-data licensing must be reviewed before launch in each region.
- Spam filtering is heuristic and user-overridable; it never destroys data.

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Which exchanges and chains are committed by region for the first public release? | Product/Data |
| OQ-002 | Are WalletConnect and xpub launch requirements or fast-follow? | Product |
| OQ-003 | Max accounts/chains per plan? | Product |
| OQ-004 | Spam-token policy: hide-only, user-report, or provider list? | Data/Product |
| OQ-005 | Which DeFi positions are detected at launch: staking only, or LP/lending too? | Data |
| OQ-006 | Exchange API commercial-use terms per venue? | Legal |
