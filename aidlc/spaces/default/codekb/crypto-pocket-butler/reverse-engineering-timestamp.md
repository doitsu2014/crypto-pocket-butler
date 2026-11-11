# Reverse Engineering Timestamp

## Run Record

- Date: 2026-11-09
- Commit: 14742cb9626e0277df057db016a3ca73fa81b990

Full rescan of the project-root repository `crypto-pocket-butler` for intent
`brd-002-users-roles`. The scan covered the whole repository except build
output, dependency folders, IDE/tool caches, generated Structurizr artefacts,
and AI-DLC's own install (`./.claude/` and `./aidlc/`). Deep analysis focused on
the Rust service crates (especially Identity and the API Gateway), the frontend
packages, the migrations and schema bootstraps, and the architecture and BRD
documents; the remaining service bodies were read at route/layout/representative
granularity. See `../../inception/reverse-engineering/developer-scan.md` in the
intent record for the full coverage report.

## Scope of Analysis

```yaml
scope_version: 1
kind: full
intent: brd-002-users-roles
fingerprint: 7c8881eb6f944c30773d9a7ab635551e668e94ee
analyzed:
  paths:
    - ./
  components:
    - API Gateway (BFF)
    - Identity Service
    - Account Service
    - Asset Registry Service
    - Market Data Service
    - Portfolio Service
    - Ledger Service
    - Reporting Service
    - Notification Service
    - Analytics Service
    - Job Orchestration Service
    - Audit Service
    - React Web Client (SPA)
    - Support Console
    - Shared Four-Crate Architecture
    - Local Docker Compose Stack
    - Swarm Production Stack
    - Observability Stack
    - Design System
    - Documentation Set
    - Build and Boundary Tooling
    - Billing Service (planned, not implemented)
shallow:
  paths:
    - services/analytics/
    - services/asset-registry/
    - services/ledger/
    - services/market-data/
    - services/notification/
    - services/reporting/
    - web/src/features/
    - web/src/components/
    - web/src/i18n/
    - support-console/src/components/
    - support-console/src/domain/
    - deployment/localhost/scripts/
    - docs/brds/
    - docs/architecture/workspace.dsl
    - design-system/
    - supabase/
```
