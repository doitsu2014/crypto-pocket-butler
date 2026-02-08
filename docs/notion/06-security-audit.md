# 06 — Security, Permissions & Audit

# Security, Permissions & Audit
## API key policy
- Start read-only. Disable withdrawals always.
- If trading enabled: allowlist symbols, max order size, IP whitelist.
## Operational security
- Secrets stored encrypted; rotation plan.
- Separate environments: dev vs prod.
## Audit log (required)
1. Snapshot: positions + prices used for decision.
1. Recommendation: proposed orders + rationale.
1. Execution: what was actually placed (if enabled).
