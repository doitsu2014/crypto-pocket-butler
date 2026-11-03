# BRD-010 — Admin Configuration

Draft v1.1 · Operations/Product · 2026-11-02. Administrative management of the chain and token registries. These registries decide which on-chain accounts are queried, which tokens are detected, and how balances are normalised — bad configuration corrupts valuation.

**In scope:** EVM chain configuration · EVM and Solana token registries · manual entry with reviewed import · catalogue sync · access control and audit.
**Out of scope:** user management/billing · feature flags · editing user portfolios · exchange/RPC credential management (runbook, not UI) · new chain *types*.

## Registry shapes

**EVM chain:** name, unique chain ID, RPC URL, native symbol, explorer URL (optional), active flag. RPC URL changes take effect on the next sync without deployment. Solana is its own non-EVM configuration.

**EVM token:** chain, symbol, contract address, decimals, native flag, active. **Solana token:** mint address, symbol, decimals, active. Contract/mint addresses are unique per chain; decimals must match the on-chain token.

**Entry:** manual from a trusted reference (canonical token list or explorer), reviewed before activation; or synced from the asset catalogue. Imports never silently overwrite an existing mapping and never bulk-activate.

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-ADM-01 | Only `admin`-role users reach chain/token administration. | Must |
| FR-ADM-02 | List, create, edit, deactivate EVM chains. | Must |
| FR-ADM-03 | Validate chain fields and unique chain IDs. | Must |
| FR-ADM-04 | List, create, edit, deactivate EVM tokens per chain. | Must |
| FR-ADM-05 | List, create, edit, deactivate Solana tokens. | Must |
| FR-ADM-06 | Reject duplicate contract/mint addresses per chain. | Must |
| FR-ADM-07 | Manual contract entry with review before activation. | Should |
| FR-ADM-08 | Import from a chain's canonical token list. | Should |
| FR-ADM-09 | Trigger contract-mapping sync from the asset catalogue. | Should |
| FR-ADM-10 | Changes take effect on the next sync, no deployment. | Must |
| FR-ADM-11 | Audit every admin create/update/deactivate with actor and timestamp. | Should |
| FR-ADM-12 | Warn before deactivating a chain that would break on-chain accounts. | Should |
| FR-ADM-13 | Documented administrative API. | Could |

## Business rules

| ID | Rule |
|----|------|
| BR-ADM-01 | Admin actions require the `admin` role; portfolio ownership grants nothing. |
| BR-ADM-02 | Chain IDs and contract addresses unique within their scope. |
| BR-ADM-03 | Decimals are required, match on-chain, and change only with explicit confirmation and re-sync. |
| BR-ADM-04 | Inactive chains/tokens are excluded from sync but retained for history. |
| BR-ADM-05 | Only configured, active chains are queried by on-chain account sync. |
| BR-ADM-06 | A chain's native asset resolves even without an ERC-20/SPL entry. |
| BR-ADM-07 | Lookup results are reviewed before activation; no automatic bulk activation. |
| BR-ADM-08 | Admin changes never alter stored snapshots retroactively. |
| BR-ADM-09 | RPC URLs may embed provider secrets and are masked in user-facing output. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-ADM-01 | A non-admin calling an admin endpoint is denied. |
| AC-ADM-02 | A valid new chain is usable by on-chain accounts on the next refresh. |
| AC-ADM-03 | A duplicate chain ID is rejected. |
| AC-ADM-04 | Re-registering an existing contract on a chain is rejected. |
| AC-ADM-05 | Lookup results import only on explicit Admin selection. |
| AC-ADM-06 | The next on-chain account sync uses the new registry configuration. |
| AC-ADM-07 | The audit log records actor, timestamp, and change. |
| AC-ADM-08 | A deactivated token is not queried; historical snapshots remain intact. |

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Who holds the admin role; environment-specific? | Operations |
| OQ-002 | Bulk selection for lookup imports? | Product |
| OQ-003 | Process for removing a chain/token with active user data? | Operations |
| OQ-004 | RPC provider keys per chain or global? | Operations |
| OQ-005 | Scheduled or manual registry sync? | Operations |
