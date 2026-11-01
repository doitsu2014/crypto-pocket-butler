# BRD-004 — Account Management

| Field | Value |
|-------|-------|
| Document ID | BRD-004 |
| Version | 1.0 |
| Status | Draft — requires product and security sign-off |
| Owner | Product |
| Last updated | 2026-11-01 |
| Related documents | BRD-002, BRD-005, BRD-006, BRD-010, BRD-011 |

## 1. Purpose

Define how users connect external holdings sources ("accounts"), how those connections are secured and synchronised, and what the business guarantees about freshness, normalisation, and ownership. An **account** is a read-only connection to an exchange or a public wallet address.

## 2. Scope

### In scope

- Creating, editing, deactivating, and deleting accounts.
- Exchange accounts with read-only API credentials (OKX is the MVP exchange).
- Wallet accounts identified by public address, with one or more enabled chains.
- Single-account and bulk synchronisation.
- Normalisation of balances into asset + quantity + availability.
- Sync status, timestamps, and error reporting.

### Out of scope

- Withdrawals, deposits, transfers, or trading through the product.
- Private-key or seed-phrase import.
- Exchange accounts beyond the MVP exchange(s) — later BRD revision.
- Real-time streaming balance updates (MVP uses on-demand and scheduled sync).
- Cost basis and transaction history (post-MVP).

## 3. Actors

| Actor | Interaction |
|-------|-------------|
| Portfolio Owner | Creates and manages own accounts, triggers sync |
| Admin | Can also manage own accounts; may trigger jobs |
| System jobs | Refresh holdings/prices on a schedule |
| Exchange / blockchain RPC | Provides balances to the product |

## 4. Account types

| Type | Required data | Authentication | Balance source |
|------|---------------|----------------|----------------|
| **Exchange** | Exchange name, API key, API secret, passphrase (where required) | Read-only API credentials, signed requests | Exchange REST API |
| **Wallet** | Public wallet address, enabled chains | None (public data) | Blockchain JSON-RPC |

### 4.1 Exchange account requirements

- Credentials must be created by the user with **read-only** permissions.
- The product must never request or accept withdrawal or trading permissions as a prerequisite.
- Secrets must be encrypted before persistence and never returned in any API response.
- The account must record the exchange name and the last successful sync time.

### 4.2 Wallet account requirements

- Only a public address is required; no private key material may ever be accepted.
- The user must select one or more enabled chains from the configured chain registry.
- Native balances and registered token balances must be read for each enabled chain.
- Balances for the same asset across multiple chains must be merged for portfolio calculations, while chain-level detail may be retained.

## 5. Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-ACC-01 | A user must be able to create an exchange account with exchange name and read-only credentials. | Must |
| FR-ACC-02 | A user must be able to create a wallet account with a public address and at least one enabled chain. | Must |
| FR-ACC-03 | The system must validate the account type and required fields before saving. | Must |
| FR-ACC-04 | The system must encrypt exchange credentials at rest before persistence. | Must |
| FR-ACC-05 | The system must never return credential fields in API responses. | Must |
| FR-ACC-06 | A user must be able to edit an account's name, credentials, and enabled chains. | Must |
| FR-ACC-07 | Editing credentials must re-encrypt them and record the change. | Must |
| FR-ACC-08 | A user must be able to activate or deactivate an account; inactive accounts must be excluded from sync and from portfolio aggregation. | Must |
| FR-ACC-09 | A user must be able to delete an account; deletion must remove portfolio links without deleting the portfolios. | Must |
| FR-ACC-10 | A user must be able to trigger a sync for one account. | Must |
| FR-ACC-11 | A user must be able to trigger a sync for all their active accounts and receive per-account results. | Should |
| FR-ACC-12 | Sync must replace the account's stored holdings with the freshly retrieved set and update the last-synced timestamp. | Must |
| FR-ACC-13 | Sync must filter zero balances and normalise quantities to human-readable decimal values. | Must |
| FR-ACC-14 | Sync failures must be reported per account without aborting other accounts in a bulk operation. | Must |
| FR-ACC-15 | The UI must show holdings count and last-synced time per account. | Should |
| FR-ACC-16 | The system should retain a record of the last sync attempt outcome (success/failure and reason). | Should |

## 6. Business rules

| ID | Rule |
|----|------|
| BR-ACC-01 | An account must belong to exactly one user. |
| BR-ACC-02 | Exchange accounts require exchange name and complete credentials; wallet accounts require an address. |
| BR-ACC-03 | All holdings quantities are stored as normalised decimal values, not raw on-chain integers. |
| BR-ACC-04 | Token decimals must be applied when converting raw balances; unknown decimals must be treated as a sync error, not guessed silently. |
| BR-ACC-05 | Available + frozen must equal the total quantity for exchange holdings. |
| BR-ACC-06 | Frozen quantity must not exceed total quantity. |
| BR-ACC-07 | Inactive accounts are not synchronised and do not contribute holdings to portfolios. |
| BR-ACC-08 | Deleting an account cascades only to its portfolio links and sync metadata; portfolio records remain. |
| BR-ACC-09 | A sync must be idempotent for a given point in time: repeating it must not create duplicate holdings. |
| BR-ACC-10 | Exchange API credentials must never be logged, returned, or included in error messages. |
| BR-ACC-11 | Wallet address validation must reject malformed addresses before any network call. |

## 7. Data requirements

| ID | Data element | Description | Sensitivity |
|----|--------------|-------------|-------------|
| DR-ACC-01 | Account ID | Stable identifier | Internal |
| DR-ACC-02 | Owner user ID | Account owner | Internal |
| DR-ACC-03 | Account type | Exchange or wallet | Internal |
| DR-ACC-04 | Display name | User-defined label | Personal |
| DR-ACC-05 | Exchange name | Provider identifier for exchange accounts | Internal |
| DR-ACC-06 | Encrypted API key / secret / passphrase | Credentials for exchange access | Secret |
| DR-ACC-07 | Wallet address | Public address for wallet accounts | Personal |
| DR-ACC-08 | Enabled chains | List of chain identifiers for wallet accounts | Internal |
| DR-ACC-09 | Holdings | Asset, total, available, frozen, decimals | Sensitive |
| DR-ACC-10 | Last synced at | Timestamp of last successful sync | Internal |
| DR-ACC-11 | Active flag | Whether the account participates in sync | Internal |

## 8. Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-ACC-01 | Given valid exchange credentials, when a user creates an exchange account, then the account is saved with encrypted credentials and no secret is returned. |
| AC-ACC-02 | Given a wallet address and selected chains, when the account is created, then it is saved as a wallet account with those chains enabled. |
| AC-ACC-03 | Given missing credentials for an exchange account, when creation is attempted, then validation fails with a specific error. |
| AC-ACC-04 | Given an exchange account with failing authentication, when sync runs, then the account is reported as failed and other accounts still process. |
| AC-ACC-05 | Given a successful sync, when holdings are stored, then zero balances are excluded, quantities are normalised, and last-synced time is updated. |
| AC-ACC-06 | Given a wallet with two enabled chains holding the same asset, when synced, then the asset appears once with the combined quantity. |
| AC-ACC-07 | Given an inactive account linked to a portfolio, when allocation is calculated, then the account's holdings are excluded. |
| AC-ACC-08 | Given an account linked to two portfolios, when deleted, then both links are removed and both portfolios remain. |
| AC-ACC-09 | Given any API response, when inspected, then no credential field or encrypted secret value appears. |

## 9. Assumptions, dependencies, constraints

- OKX exposes read-only spot balances via its REST API with signed requests.
- Blockchain RPC endpoints are configured by an Admin (BRD-010).
- The token registry determines which tokens are queried for wallet accounts (BRD-005/BRD-010).
- Exchange rate limits and RPC rate limits may require batching and backoff.
- The MVP stores holdings as cached snapshots; it does not maintain a transaction ledger.

## 10. Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Which additional exchanges are required after OKX, and in what order? | Product |
| OQ-002 | Is bulk sync a launch requirement or a fast-follow? | Product |
| OQ-003 | What is the maximum number of accounts/chains per user? | Product |
| OQ-004 | Should account sync failures notify the user proactively? | Product |
| OQ-005 | How should an unknown token with no registered decimals be handled (skip, exclude, alert)? | Data |
