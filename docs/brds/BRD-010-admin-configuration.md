# BRD-010 — Admin Configuration & Operations

Draft v1.2 · Operations/Product · 2026-11-04. Administrative management of the chain and token registries, market-data sources, users, entitlements, and operational controls. Bad configuration corrupts valuation; over-broad admin power corrupts trust. v1.2 adds user management, support tooling, feature flags, quotas, and source health.

**In scope:** EVM chain configuration · EVM, Solana, Bitcoin, and Tron token registries · manual entry with reviewed import · catalogue sync · market-data source configuration and health · FX source configuration · user and role management · entitlement overrides · feature flags · quotas · audit · support tooling.
**Out of scope:** editing user portfolios · exchange/RPC credential management (runbook, not UI) · billing price setting (BRD-016) · legal/regulatory decisions · new chain *types*.

## Registry shapes

**Chain:** name, unique chain ID, RPC URL, native symbol, explorer URL (optional), active flag. RPC URL changes take effect on the next sync without deployment. Solana, Bitcoin, and Tron are their own non-EVM configurations.

**Token:** chain, symbol, contract/mint/address, decimals, native flag, active, spam flag. Contract/mint addresses are unique per chain; decimals must match the on-chain token.

**Market-data source:** name, priority, supported assets/regions, licence reference, active flag, health status.

**Entry:** manual from a trusted reference, reviewed before activation; or synced from the asset catalogue. Imports never silently overwrite an existing mapping and never bulk-activate.

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-ADM-01 | Only `admin`-role users reach administration. | Must |
| FR-ADM-02 | List, create, edit, deactivate chains. | Must |
| FR-ADM-03 | Validate chain fields and unique chain IDs. | Must |
| FR-ADM-04 | List, create, edit, deactivate tokens per chain. | Must |
| FR-ADM-05 | List, create, edit, deactivate Solana, Bitcoin, and Tron tokens/address formats. | Must |
| FR-ADM-06 | Reject duplicate contract/mint/address registrations per chain. | Must |
| FR-ADM-07 | Manual contract entry with review before activation. | Must |
| FR-ADM-08 | Import from a chain's canonical token list. | Should |
| FR-ADM-09 | Trigger contract-mapping sync from the asset catalogue. | Should |
| FR-ADM-10 | Changes take effect on the next sync, no deployment. | Must |
| FR-ADM-11 | Audit every admin create/update/deactivate with actor and timestamp. | Must |
| FR-ADM-12 | Warn before deactivating a chain that would break on-chain accounts. | Must |
| FR-ADM-13 | Documented administrative API. | Could |
| FR-ADM-14 | Look up users, change roles, disable accounts, and override entitlements with audit. | Must |
| FR-ADM-15 | Provide support read-only, time-bound access with a recorded reason (BRD-019). | Must |
| FR-ADM-16 | Configure market-data sources, priority, and licences. | Must |
| FR-ADM-17 | Configure FX sources and currency support. | Must |
| FR-ADM-18 | Manage feature flags with owner, expiry, and rollout percentage. | Must |
| FR-ADM-19 | Manage plan quotas and entitlement defaults. | Must |
| FR-ADM-20 | View source health, job health, and data-coverage dashboards. | Should |
| FR-ADM-21 | Manage spam-token lists and user reports. | Should |
| FR-ADM-22 | Export admin audit records for compliance review. | Should |

## Business rules

| ID | Rule |
|----|------|
| BR-ADM-01 | Admin actions require the `admin` role; portfolio ownership grants nothing. |
| BR-ADM-02 | Chain IDs and contract/mint addresses unique within their scope. |
| BR-ADM-03 | Decimals are required, match on-chain, and change only with explicit confirmation and re-sync. |
| BR-ADM-04 | Inactive chains/tokens are excluded from sync but retained for history. |
| BR-ADM-05 | Only configured, active chains are queried by on-chain account sync. |
| BR-ADM-06 | A chain's native asset resolves even without a token entry. |
| BR-ADM-07 | Lookup results are reviewed before activation; no automatic bulk activation. |
| BR-ADM-08 | Admin changes never alter stored snapshots retroactively. |
| BR-ADM-09 | RPC URLs may embed provider secrets and are masked in user-facing output. |
| BR-ADM-10 | Admin and support access is least-privilege, time-bound, and audited. |
| BR-ADM-11 | A feature flag has an owner and an expiry; an orphaned flag is removed. |
| BR-ADM-12 | Entitlement overrides are recorded and reconciled with billing. |
| BR-ADM-13 | Market-data sources cannot be activated without a licence reference. |
| BR-ADM-14 | Quota changes are audited and communicated to affected users. |
| BR-ADM-15 | Admin changes never expose another user's credentials. |

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
| AC-ADM-09 | Support opens a user record with a reason; the access expires and is audited. |
| AC-ADM-10 | A role or entitlement change is audited and reconciled with billing. |
| AC-ADM-11 | A feature flag without an owner or expiry is rejected or removed. |
| AC-ADM-12 | A market-data source without a licence reference cannot be activated. |
| AC-ADM-13 | An admin export contains audit records without credentials. |

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Who holds the admin role; environment-specific? | Operations |
| OQ-002 | Bulk selection for lookup imports? | Product |
| OQ-003 | Process for removing a chain/token with active user data? | Operations |
| OQ-004 | RPC provider keys per chain or global? | Operations |
| OQ-005 | Which feature-flag system is used across services? | Engineering |
| OQ-006 | Who approves market-data licences? | Legal/Finance |
