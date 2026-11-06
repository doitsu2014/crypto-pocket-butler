# CI/CD Pipeline — u1-walking-skeleton (Review artifact)

Pipeline: **GitHub Actions**, running the same commands developers run locally. Images are
pushed to a **public registry** with digest-pinned references (the interview's choice) and a
vulnerability scan gate; deployment is a health-gated Swarm rolling update to **production
only**.

## Stages and gates

| Stage | Command / action | Gate |
|---|---|---|
| 1. Checkout and toolchain | pinned Rust toolchain, Node and corepack, Yarn ≥6 | toolchain versions match the repository |
| 2. Format | `cargo fmt --all -- --check` | must pass |
| 3. Lint | `cargo clippy --workspace --all-targets -- -D warnings`; ESLint and Prettier for TypeScript | must pass |
| 4. Unit and integration tests | `cargo test --workspace` with TDD tests; one adapter integration test against a Postgres schema | must pass; coverage ≥80% per Rust crate |
| 5. Build images | multi-stage build per service, digest recorded | image builds |
| 6. Scan | dependency and image vulnerability scan | no critical or high without an accepted risk |
| 7. Push | push to the public registry, tag by commit SHA and digest | immutable digest recorded |
| 8. Migrate | apply versioned up/down sqlx migrations as a pre-deploy step | migration succeeds; down path recorded |
| 9. Deploy | `docker stack deploy` with the new digest | rolling update; health check must pass |
| 10. Smoke | health and readiness endpoints plus one asset-resolution request | must pass; otherwise roll back |

## Deployment strategy

- Rolling update with Swarm defaults, health-gated; a failed health check rolls back to the
  previous digest automatically.
- The rollback path applies the matching down migration when the release changed the schema.
- No staging tier; the smoke test is the production check.

## Secrets in CI/CD

- The pipeline reads deployment credentials from GitHub Actions secrets, never from the
  repository.
- Runtime secrets stay in Supabase Vault; the pipeline never holds them.
- Images are pulled by digest; the registry credential is scoped to push/pull only.

## Recorded risks

- **Public registry:** images are world-readable; digest pinning and the scan gate are the only
  integrity controls. A private registry is the recommended hardening step.
- **Single-node Swarm:** a node failure takes the service down; there is no failover at launch.
- **No staging tier:** the smoke test is the only pre-production verification.
