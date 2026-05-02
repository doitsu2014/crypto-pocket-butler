# Crypto Pocket Butler Constitution

## Core Principles

### I. Library-First Architecture

Every feature MUST be implemented as a self-contained, independently testable library or service with clear boundaries.

- Bounded contexts (Wallet, Portfolio, Price, Notification, User) define service interfaces
- Libraries MUST be self-contained: no hidden dependencies across bounded contexts
- Each library/service MUST have its own test suite that can run in isolation
- Clear public API contract required — no organizational-only libraries
- Rationale: Crypto Pocket Butler spans multiple chains and complex financial calculations; isolated components prevent cascading failures and enable independent deployment

### II. Test-First Development (NON-NEGOTIABLE)

TDD is mandatory for all implementation work. The Red-Green-Refactor cycle MUST be followed strictly.

- Tests written BEFORE implementation — they must fail before code is written
- Contract tests REQUIRED for all public API endpoints
- Integration tests REQUIRED for: new service contracts, inter-service communication, shared schemas
- Unit tests REQUIRED for: business logic, P&L calculations, cost basis methods
- Rationale: P&L calculations and tax reporting are legally sensitive; bugs have real financial consequences

### III. Observable Systems

All services MUST emit structured logs, traces, and metrics for debuggability and monitoring.

- Structured logging in JSON format with correlation IDs
- Every API endpoint MUST log: request ID, duration, status code, chain (if applicable)
- Price staleness indicators MUST be logged when oracle feeds are stale
- Metrics: request latency p50/p95/p99, error rates, cache hit ratios
- Rationale: Multi-chain portfolio aggregation requires tracing across Wallet, Portfolio, and Price services; debugging production issues requires correlation IDs

### IV. Performance by Design

Performance requirements are non-negotiable and MUST be specified before implementation.

- Portfolio valuation response time: <500ms for up to 20 wallets
- Price refresh interval: 30-60 seconds (configurable)
- API response time: <200ms p95 for read operations
- All long-running operations (blockchain indexing) MUST be async and non-blocking
- Rationale: Retail investors and traders expect real-time data; slow dashboards erode trust

### V. UX Consistency

All user-facing components MUST follow the project design system for consistency.

- Use the established component library — no ad-hoc UI components
- Loading states, error states, and empty states MUST be designed for every user journey
- Wallet connect flows MUST be consistent across MetaMask, WalletConnect, and hardware wallets
- Rationale: Users manage multiple wallets across chains; inconsistent UX creates cognitive load and errors

## Security Requirements

### Non-Custodial by Design

The system MUST never store private keys, seed phrases, or signing credentials.

- Wallets are connected via read-only APIs (ethers.js, web3.js, WalletConnect)
- All wallet operations requiring signing happen client-side only
- Audit trail MUST log every wallet connection, disconnection, and data access event
- Rationale: User trust is paramount; a single security incident would destroy the product

### Data Isolation

User data MUST be isolated per account; no cross-account data access.

- Multi-tenant database schema with strict row-level security
- API endpoints MUST enforce authorization checks on every request
- Session management: 24-hour expiry, active session visibility, revocation capability
- Rationale: Family offices and financial advisors share access; data leakage is catastrophic

## Development Workflow

### Multi-Agent Process

All feature work follows the Speckit → Superpower → Claude Code workflow.

1. **Speckit** generates feature specifications and structure
2. **Superpower** determines correct skill/approach based on context
3. **Claude Code** implements with TDD discipline

### Quality Gates

All PRs MUST pass before merge:

- All contract tests green
- All integration tests green
- Linting and formatting checks pass
- Security scan (Quinn) passed
- Performance regression check (if applicable)

### Complexity Justification

Complex solutions require explicit justification in the PR description:

| Why Needed | Simpler Alternative | Why Rejected |
|------------|---------------------|--------------|
| Document why 4th service | 3-service approach | Specific reason |
| Document repository pattern | Direct DB access | Specific problem |

## Governance

**Constitution Supremacy**: This constitution supersedes all other development practices. Any conflict between practices and this constitution MUST be resolved in favor of this document.

**Amendment Procedure**:
1. Proposed change documented with rationale and migration plan
2. PR review verifies compliance with affected principles
3. Major changes (backward-incompatible): requires team approval
4. Minor changes (new guidance): requires code owner approval
5. Patch changes (clarifications): requires single reviewer

**Compliance Verification**:
- All PRs/reviews MUST verify compliance with this constitution
- The `/speckit-analyze` command checks cross-artifact consistency
- Performance requirements MUST be verified via benchmarks before PR merge

**Runtime Guidance**: Use `.claude/skills/` for development guidance and `.specify/memory/` for project state.

**Version**: 1.0.0 | **Ratified**: 2026-05-02 | **Last Amended**: 2026-05-02
