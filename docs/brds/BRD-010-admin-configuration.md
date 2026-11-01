# BRD-010 — Admin Configuration

| Field | Value |
|-------|-------|
| Document ID | BRD-010 |
| Version | 1.0 |
| Status | Draft — requires operations sign-off |
| Owner | Operations / Product |
| Last updated | 2026-11-01 |
| Related documents | BRD-002, BRD-004, BRD-005, BRD-011 |

## 1. Purpose

Define the administrative capabilities that keep the platform's chain and token registries accurate. These registries determine which wallets can be queried, which tokens are detected, and how balances are normalised, so incorrect configuration directly corrupts portfolio valuation.

## 2. Scope

### In scope

- Managing EVM chain configurations.
- Managing the EVM token registry.
- Managing the Solana token registry.
- Looking up token contracts by symbol through an approved data provider.
- Synchronising contract data from the asset catalogue.
- Access control and audit for administrative changes.

### Out of scope

- User management and billing (post-MVP).
- System-wide feature flags or pricing configuration (future).
- Direct editing of user portfolios or recommendations.
- Managing price-provider subscriptions and credentials (operations runbook, not product UI).
- Adding new chain *types* (the product supports EVM and Solana; adding another family is a product change).

## 3. Actors

| Actor | Interaction |
|-------|-------------|
| Admin | Creates and updates chains and tokens; runs lookups and syncs |
| System jobs | Use the registry during account sync and asset resolution |
| Non-admin user | Cannot access administrative capabilities |

## 4. EVM chain configuration

| Field | Required | Description |
|-------|----------|-------------|
| Name | Yes | Display/technical name (for example, `ethereum`) |
| Chain ID | Yes | Numeric chain identifier used by RPC |
| RPC URL | Yes | JSON-RPC endpoint used for balance queries |
| Native symbol | Yes | Native token symbol (for example, ETH) |
| Explorer URL | No | Block-explorer link for user reference |
| Active status | Yes | Whether the chain may be used by wallet sync |

Rules:

- A chain must have a unique chain ID.
- Only active chains may be enabled on a wallet account.
- Changing an RPC URL must take effect on the next sync without code deployment.
- At least one EVM chain must be configured for EVM wallet support.
- Solana is a distinct non-EVM chain with its own configuration and token registry.

## 5. Token registry

### 5.1 EVM tokens

| Field | Required | Description |
|-------|----------|-------------|
| Chain | Yes | The chain the token exists on |
| Symbol | Yes | Canonical token symbol |
| Contract address | Yes | Token contract on that chain |
| Decimals | Yes | Precision used to normalise balances |
| Native flag | Yes | Whether the token is the chain's native asset |
| Active status | Yes | Whether the token is queried during sync |

### 5.2 Solana tokens

| Field | Required | Description |
|-------|----------|-------------|
| Mint address | Yes | SPL token mint |
| Symbol | Yes | Canonical token symbol |
| Decimals | Yes | Precision used to normalise balances |
| Active status | Yes | Whether the token is queried during sync |

### 5.3 Lookup and sync

- **Lookup by symbol:** an Admin enters a symbol; the system queries an approved data provider for contract addresses across configured chains; the Admin reviews and imports selected results.
- **Sync from contracts:** the system reads the asset catalogue and adds or updates known contract mappings in the token registry.
- Imports must be reviewable before becoming active and must not silently overwrite an existing mapping.

## 6. Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-ADM-01 | Only users with the admin role may access chain and token administration. | Must |
| FR-ADM-02 | An Admin must be able to list, create, edit, and deactivate EVM chains. | Must |
| FR-ADM-03 | Chain records must validate required fields and unique chain IDs. | Must |
| FR-ADM-04 | An Admin must be able to list, create, edit, and deactivate EVM tokens per chain. | Must |
| FR-ADM-05 | An Admin must be able to list, create, edit, and deactivate Solana tokens. | Must |
| FR-ADM-06 | Token records must reject duplicate contract/mint addresses on the same chain. | Must |
| FR-ADM-07 | An Admin must be able to look up token contracts by symbol through an approved provider. | Should |
| FR-ADM-08 | An Admin must be able to import selected lookup results into the registry. | Should |
| FR-ADM-09 | An Admin must be able to trigger a synchronisation of contract mappings from the asset catalogue. | Should |
| FR-ADM-10 | Registry changes must take effect on subsequent syncs without a deployment. | Must |
| FR-ADM-11 | Every administrative create/update/deactivate must be recorded in an audit log with actor and timestamp. | Should |
| FR-ADM-12 | The system must prevent deactivation of a chain that would leave a wallet's only enabled chain unusable without warning the affected users. | Should |
| FR-ADM-13 | The administrative API must be documented. | Could |

## 7. Business rules

| ID | Rule |
|----|------|
| BR-ADM-01 | Administrative actions require the `admin` role; ownership of a portfolio does not grant administrative access. |
| BR-ADM-02 | Chain IDs and contract addresses are unique within their scope. |
| BR-ADM-03 | A token's decimals are required and must match the on-chain token; changing decimals requires explicit confirmation and re-sync. |
| BR-ADM-04 | Inactive chains/tokens are excluded from account sync but retained for historical reference. |
| BR-ADM-05 | Only configured, active chains may be queried by wallet sync. |
| BR-ADM-06 | The native asset of a chain must be resolvable even if no ERC-20/SPL entry exists for it. |
| BR-ADM-07 | Lookup results must be reviewed by an Admin before activation; no automatic bulk activation. |
| BR-ADM-08 | Administrative changes must not retroactively alter stored portfolio snapshots. |
| BR-ADM-09 | RPC URLs may contain provider secrets and must be masked in user-facing output. |

## 8. Data requirements

| ID | Data element | Description | Sensitivity |
|----|--------------|-------------|-------------|
| DR-ADM-01 | Chain record | Name, chain ID, RPC URL, native symbol, explorer, status | RPC URL may be secret |
| DR-ADM-02 | EVM token record | Chain, symbol, contract, decimals, native flag, status | Internal |
| DR-ADM-03 | Solana token record | Mint, symbol, decimals, status | Internal |
| DR-ADM-04 | Lookup results | Provider-supplied contract candidates | Internal |
| DR-ADM-05 | Registry change audit | Actor, change, timestamp, previous/new value | Audit |
| DR-ADM-06 | Active status | Whether the chain/token participates in sync | Internal |

## 9. Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-ADM-01 | Given a non-admin user, when they call an administrative endpoint, then access is denied. |
| AC-ADM-02 | Given valid chain data, when an Admin adds a chain, then it is available to wallet accounts on the next configuration refresh. |
| AC-ADM-03 | Given a duplicate chain ID, when an Admin adds a chain, then the request is rejected. |
| AC-ADM-04 | Given a token contract already registered on a chain, when an Admin adds it again, then the duplicate is rejected. |
| AC-ADM-05 | Given a symbol lookup, when results are returned, then the Admin can select and import contracts without automatic activation. |
| AC-ADM-06 | Given a registry change, when the next wallet sync runs, then the new configuration is used. |
| AC-ADM-07 | Given an administrative change, when the audit log is inspected, then the actor, timestamp, and change are recorded. |
| AC-ADM-08 | Given a deactivated token, when a wallet is synced, then the token is not queried but historical snapshots remain intact. |

## 10. Assumptions, dependencies, constraints

- The `admin` role is provided by the identity provider (BRD-002).
- An approved asset-data provider is available for contract lookup (BRD-005).
- RPC providers impose rate limits; configuration should support provider rotation.
- Solana support uses a dedicated RPC endpoint and SPL mint registry.
- Administrative UI is part of the web application; API-only administration is acceptable as a fallback.

## 11. Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Which users receive the admin role, and is it environment-specific? | Operations |
| OQ-002 | Should lookup imports support bulk selection? | Product |
| OQ-003 | What is the process for removing a chain/token that has active user data? | Operations |
| OQ-004 | Are RPC provider keys managed per chain or globally? | Operations |
| OQ-005 | Should automatical registry sync be scheduled, or remain manual? | Operations |
