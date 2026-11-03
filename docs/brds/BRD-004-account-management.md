# BRD-004 — Account Management

Draft v1.0 · Product · 2026-11-01. How users connect read-only holdings sources and how those connections are secured and synced.

**In scope:** create/edit/deactivate/delete accounts · OKX exchange accounts with read-only credentials · on-chain accounts by public address · single and bulk sync · normalised balances, status, error reporting.
**Out of scope:** withdrawals/transfers/trading · private-key import · additional exchanges · realtime streaming · cost basis (post-MVP).

## Account types

Two types; each account owns its own holdings.

| Type | Data | Balances from |
|------|------|---------------|
| **Exchange account** (OKX in MVP) | Exchange name, API key/secret/passphrase — user-created, **read-only** | Exchange REST API |
| **On-chain account** | Public address + enabled chains — never private keys | Blockchain JSON-RPC |

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
| FR-ACC-11 | Bulk-sync all active accounts with per-account results. | Should |
| FR-ACC-12 | Sync replaces stored holdings and updates last-synced time. | Must |
| FR-ACC-13 | Sync filters zero balances and normalises to decimals. | Must |
| FR-ACC-14 | Per-account failure reporting; bulk sync continues. | Must |
| FR-ACC-15 | Show holdings count and last-synced time per account. | Should |
| FR-ACC-16 | Record last sync attempt outcome (success/failure + reason). | Should |

## Business rules

| ID | Rule |
|----|------|
| BR-ACC-01 | An account belongs to exactly one user and owns its own holdings. |
| BR-ACC-02 | Exchange accounts need name + complete credentials; on-chain accounts need an address. |
| BR-ACC-03 | Quantities are stored as normalised decimals, never raw on-chain integers. |
| BR-ACC-04 | Token decimals are applied on conversion; unknown decimals are a sync error, never a guess. |
| BR-ACC-05 | Available + frozen = total for exchange holdings; frozen ≤ total. |
| BR-ACC-06 | Inactive accounts don't sync and don't contribute holdings. |
| BR-ACC-07 | Deleting an account cascades only to its portfolio links and sync metadata. |
| BR-ACC-08 | Sync is idempotent per point in time — no duplicate holdings. |
| BR-ACC-09 | Credentials are never logged, returned, or included in errors. |
| BR-ACC-10 | Malformed on-chain addresses are rejected before any network call. |

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

## Notes

- RPC endpoints are configured by an Admin (BRD-010); the token registry drives on-chain account queries (BRD-005/010).
- Rate limits may require batching/backoff. Holdings are cached snapshots, not a transaction ledger.

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Which exchanges after OKX, in what order? | Product |
| OQ-002 | Bulk sync at launch or fast-follow? | Product |
| OQ-003 | Max accounts/chains per user? | Product |
| OQ-004 | Notify users proactively on sync failure? | Product |
| OQ-005 | Unknown token with no decimals: skip, exclude, or alert? | Data |
