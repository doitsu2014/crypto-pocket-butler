# U1 Identity Auth Domain — Technology Decisions

**Unit:** U1 `u1-identity-auth-domain`  
**Source:** technology-stack.md, project rules, service map, contract summary and requirements.

| Decision | Selected approach | Rationale / constraint |
|---|---|---|
| Backend language and runtime | Rust 2021 with tokio | Fixed project stack; asynchronous request and provider boundaries already use this runtime. |
| HTTP boundary | Axum and tower | Existing Identity service and gateway conventions; preserve RFC 7807 problem handling and middleware behavior. |
| Persistence | Supabase Postgres through sqlx | Identity owns its schema and role; no ORM, no cross-schema grants and no service RLS. Version schema changes as sqlx up/down pairs and run the service migrator before API/worker updates. |
| Domain boundaries | Clean Architecture: domain, application, infrastructure and HTTP crates | Dependencies point inward; domain remains framework-free; ports are defined inward and implemented outward. |
| Provider integration | Existing Identity AuthAdmin/provider ports and configured Supabase Auth API | Supabase Auth remains authority for credentials, token lifecycle, TOTP factors and provider sessions. Provider details are translated to typed errors and never leaked. |
| Internal service communication | Synchronous HTTP with bounded retries and idempotency keys | Required project convention; no message queues. Notification delivery is best-effort after sign-in; audit delivery uses the existing transactional outbox. |
| Secrets | Supabase Vault (pgsodium) materialized through Docker secrets and `*_FILE` settings | No credentials in source, env files, logs, errors or support surfaces. |
| Deployment | Existing Docker Swarm Identity API/worker rolling updates with health checks | No new deployable service. Preserve the configured health-gated update and rollback policy. |
| Testing | Rust unit/use-case tests with port fakes; HTTP boundary tests; Postgres integration and migration tests; browser journey coverage in U4 | TDD is mandatory. Run `cargo test --workspace` with disposable Postgres for ignored suites; auth-surface changes also require `yarn e2e`. Coverage floor is 80% per crate, measured with the workspace or paired `cargo llvm-cov` invocation. |

## Alternatives rejected

- **New Auth/MFA/Session service:** rejected; this Unit extends existing Identity ownership and keeps the account lifecycle cohesive.
- **Shared revocation deny-list or cross-schema lookup:** rejected; conflicts with the accepted stateless JWT residual window and the no-cross-schema rule.
- **Message queue for Notification/Audit:** rejected; internal communication uses synchronous HTTP, bounded retries and idempotency; Audit already uses a transactional outbox.
- **Different language, framework or ORM:** rejected; the project stack is fixed.

No new dependency, deployable component, browser persistence mechanism or CI platform is selected by this Unit.
