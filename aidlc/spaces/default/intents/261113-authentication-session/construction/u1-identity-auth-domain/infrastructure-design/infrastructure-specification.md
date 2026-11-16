# U1 Identity Auth Domain — Infrastructure Specification

**Unit:** U1 `u1-identity-auth-domain`  
**Inputs:** `nfr-design/performance-design.md`, `security-design.md`, `scalability-design.md`, `reliability-design.md`, `observability-design.md`, `logical-components.md`, `functional-design/functional-spec.md`, `inception/domain-design/components.md`, `inception/contract-design/contract-summary.md`. NFR coverage: NFR1.1, NFR1.2, NFR1.3, NFR1.4, NFR1.5, NFR6.1, NFR6.2, NFR6.3, NFR6.4, NFR6.5, NFR6.6, NFR6.7, NFR6.8, NFR6.9, NFR2.1, NFR2.2, NFR2.3, NFR9.1, NFR9.2, NFR3.1, NFR3.2, NFR3.3, NFR3.4, NFR3.5, NFR3.6, NFR3.7, NFR3.8.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute model | Existing Identity API, worker and migration one-shot in the current Docker Swarm stack | This Unit extends Identity; it does not introduce a new service or compute platform. |
| Network topology | Existing production ingress/gateway and internal service network; SPA continues through declared gateway routes | Preserve deny-by-default routing, unchanged user JWT forwarding, service-token protection for privileged internal calls and synchronous service communication. |
| Storage | Existing Identity Supabase Postgres schema and role; Identity-owned session, policy, gate and recovery state | Service owns its schema. No shared schema, new database, cache, read replica or cross-schema grant. |
| Environments | Production is the only environment tier; `deployment/localhost` remains developer convenience | No staging environment or promotion step is allowed. |
| Infrastructure as code | Update the existing Swarm stack and service deployment definitions only where the Identity service configuration changes | Docker Swarm is the fixed production platform; no AWS/CDK/Terraform migration is proposed. |
| Resource sizing | Preserve current replica/update configuration; bound DB connections and external HTTP concurrency per replica and measure against NFR load | No new capacity numbers are invented; validate the 50 requests/second/replica target and existing database connection budget. |
| Deployment strategy | Health-gated rolling update, `parallelism: 1`, `failure_action: rollback`, `monitor: 30s`, configured rollback behavior and service-specific start order | Must match `infra/swarm-stack.yml`; health checks gate service updates. No blue-green/canary. |
| Image identity | Immutable commit-SHA tag for release images | Continuous release from `main`; no semantic versioning until the first production release. |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| Identity API | Publicly reachable only through Gateway; owns auth/session handlers and principal policy | Existing replicated `identity-api`; preserve `/healthz`, `/readyz`, `/metrics`; independent JWT validation | Add/extend Identity routes only with corresponding gateway declarations and handlers. |
| Identity worker | Existing background service for Identity-owned outbox/session maintenance tasks | Existing worker replica/order and health behavior; bounded processing and idempotent tasks | Session-retention purge executor must be selected/confirmed during implementation; no new worker is assumed. |
| Identity Postgres schema | Durable authoritative Identity state | Existing database, Identity role, sqlx migrations as paired up/down files; indexes for owner-scoped session reads | Run migration one-shot before API/worker update; rollback applies matching down migration. |
| Supabase Auth | External identity provider for credentials, factors, token lifecycle and provider session validity | Existing configured endpoint/JWKS; provider secret through Vault-backed Docker secret and `*_FILE` | No fallback auth. Provider-specific calls have bounded timeout/concurrency and safe typed errors. |
| Notification service | Existing owner of in-app notification persistence/delivery | Existing internal HTTP and service-token boundary; stable idempotency key, bounded retries | Notification failure does not fail sign-in. |
| Audit service | Existing append-only owner for security events | Identity transactional outbox drained over existing internal HTTP contract; insert-only Audit role | No direct cross-schema access and no new message queue. |
| Secret source | Supabase Vault/pgsodium materialized as Docker secrets | Files mounted to containers and referenced through `*_FILE`; service token remains separate from user JWT | No credentials in source, image, environment value, error or log. |
| Metrics/logs/traces collector | Not deployed | Identity emits structured stdout logs and `/metrics`; no collector/scrape config is added here | State accurately: signals are exposed but not currently collected. |

No shared infrastructure resource is added by U1. The only persistent data owner remains Identity for its own schema; Notification and Audit retain their own service schemas and APIs.

## NFR infrastructure mapping

| Requirement | Infrastructure decision |
|---|---|
| NFR1.1, NFR1.2, NFR1.5 | Identity Postgres owner-scoped indexes, bounded cursor pagination and bounded pools; verify p95/p99 at 50 requests/second per replica. |
| NFR1.3, NFR3.4 | Bounded provider/HTTP concurrency, configured timeouts and transient-only idempotent retries. |
| NFR1.4, NFR2.1, NFR2.2, NFR2.3 | Authoritative Identity Postgres state; no stale cache, atomic constraints and idempotency; bounded session history pages. |
| NFR6.1, NFR6.2, NFR6.5, NFR6.6, NFR6.7, NFR6.8, NFR6.9 | Existing gateway/JWT/service-token boundary, Identity role and Vault-backed secret files; safe failures and route budgets. |
| NFR6.3, NFR6.4, NFR3.7 | Secret-free container/log surface and transactional Identity outbox to the append-only Audit service. |
| NFR3.1, NFR3.2, NFR3.3, NFR3.5, NFR3.6, NFR3.8 | Existing replicated Identity service, health-gated Swarm rollback, provider/notification isolation and documented residual token window. |
| NFR9.1, NFR9.2 | Existing Identity storage and stateless API replicas; 90-day session retention, with purge executor assigned during implementation. |
