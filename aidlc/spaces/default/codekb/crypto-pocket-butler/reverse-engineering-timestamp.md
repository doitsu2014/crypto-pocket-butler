# Reverse Engineering Timestamp

## Run Record

- Date: 2026-11-15
- Commit: 112b3541355aa1c38d9370d49142a2d3fb9e5866

Focused merge of the CURRENT shared CodeKB. The verified existing scope is retained and unioned with this run's focused `services/reporting/domain/` scan. The source snapshot fingerprint covers the complete merged analyzed path set. No builds, tests, or programs were run.

## Scope of Analysis

```yaml
scope_version: 1
kind: partial
intent: 261115-remove-reporting-deps
fingerprint: 4c865a1c3273c8998f986dfa90d7674daccd83e4
analyzed:
  paths:
    - Cargo.toml
    - package.json
    - web/package.json
    - web/src/auth/session.ts
    - web/src/auth/storage.ts
    - web/src/auth/client.ts
    - web/src/auth/session.test.ts
    - web/src/features/auth/SignInPage.tsx
    - web/src/api/client.ts
    - services/gateway/domain/src/route.rs
    - services/gateway/infrastructure/src/auth.rs
    - services/identity/Cargo.toml
    - services/identity/http/src/routes.rs
    - services/identity/http/src/middleware.rs
    - services/identity/application/src/sessions.rs
    - services/identity/tests/http.rs
    - services/identity/tests/authorisation_adapters.rs
    - docs/brds/BRD-003-authentication-and-session-management.md
    - .gitignore
    - aidlc/spaces/default/knowledge/aidlc-shared/domain-glossary.md
    - aidlc/spaces/default/knowledge/aidlc-shared/problem-details.md
    - aidlc/spaces/default/knowledge/aidlc-shared/service-map.md
    - aidlc/spaces/default/knowledge/aidlc-shared/stack-facts.md
    - aidlc/spaces/default/knowledge/aidlc-developer-agent/rust-conventions.md
    - aidlc/spaces/default/knowledge/aidlc-developer-agent/sqlx-migrations.md
    - aidlc/spaces/default/knowledge/aidlc-developer-agent/test-layout.md
    - services/reporting/domain/
  components:
    - SPA Authentication Session
    - SPA Gateway Client
    - Gateway Route Table
    - Gateway JWT Validator
    - Identity HTTP API
    - Identity Device Sessions
    - Reporting Domain
shallow:
  paths:
    - services/
    - web/src/
    - support-console/src/
    - deployment/
    - infra/
    - supabase/
    - scripts/
    - docs/
    - design-system/
```
