# U1 Identity Auth Domain — CI/CD and Release Requirements

**Unit:** U1 `u1-identity-auth-domain`  
**Inputs:** `nfr-design/reliability-design.md`, `security-design.md`, `observability-design.md`, `nfr-requirements/tech-stack-decisions.md`, `inception/requirements-analysis/requirements.md`, team release practices. NFR coverage: NFR6.3–NFR6.4, NFR3.1–NFR3.8, NFR5.1–NFR5.3, NFR7.1–NFR7.2, NFR9.1.

## Platform-neutral release flow

The repository has no current CI platform configuration. This Unit defines ordered gates and release behavior without selecting GitHub Actions, GitLab, Jenkins or another provider, and without adding a staging tier.

| Order | Gate / action | Required evidence |
|---|---|---|
| 1 | `cargo fmt --all -- --check` | Formatting passes. |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | No lint warnings. |
| 3 | `bash scripts/check-boundaries.sh` | Clean Architecture dependency direction passes. |
| 4 | Provision a disposable `supabase/postgres:15.8.1.060` database and run `cargo test --workspace` with `TEST_DATABASE_URL` | Unit, HTTP, adapter and migration tests pass, including ignored Postgres suites. |
| 5 | Run `cargo llvm-cov` under the project coverage measurement contract | At least 80% line coverage per crate; quote the exact command beside every result. |
| 6 | Run `yarn test`, `yarn lint`, `yarn build`, `yarn format:check` in `web/` and `support-console/` | Frontend gates pass using Yarn 4.18.1. |
| 7 | Run `yarn e2e` in `web/` for any auth/sign-in surface change | Sign-in, MFA, session and settings browser journeys pass. |
| 8 | Run `yarn size` in `web/` | Initial bundle, route chunks and stylesheet remain within recorded budgets. |
| 9 | Dependency and container-image vulnerability scans (`cargo audit`, Trivy or approved equivalents) | Required by BRD-012 NFR-SEC-14 before release; scanner selection is platform-neutral and must not be silently skipped. |
| 10 | Build and tag release images with the merge commit SHA | No mutable `:latest` release deployment. |
| 11 | Run the Identity migration one-shot against the migrator DSN before updating API/worker | Versioned sqlx up/down migration pair is present; migration succeeds before application rollout. |
| 12 | Roll out Identity API/worker using existing Docker Swarm health-gated rolling policy | `parallelism: 1`, failure-triggered rollback, monitor period and configured service update order remain as in `infra/swarm-stack.yml`. |
| 13 | Run post-deploy Identity smoke/health verification | `/healthz`, `/readyz` and authorized Identity paths pass; use `scripts/smoke-identity.sh` with throwaway tokens where applicable. |

## Rollback

- A failed post-update health check triggers the Swarm-configured rollback for the Identity service.
- The release procedure applies the matching migration `.down.sql` automatically before/with the application rollback according to the release-job contract; it must not leave the older application against an incompatible schema.
- After rollback, repeat health/readiness and the Identity smoke checks. Do not declare the rollback complete until the service and schema are verified.
- Keep image tag and migration version paired in release metadata; retain enough prior image/migration provenance to reverse the same release.

## Security and secret handling

- CI obtains database/service credentials through approved secret provisioning; it never writes secrets into source, pipeline logs, image layers or plain env files.
- Provider/service secrets are runtime Docker secrets materialized from Supabase Vault and read via `*_FILE` settings.
- Scan source dependencies and built container images before production rollout; block on the project's defined vulnerability policy once the scanner baseline is configured.

## Deployment constraints

- Production is the only environment tier. Local Compose remains a developer convenience and is not a staging promotion target.
- Releases are continuous from `main`, images use commit-SHA tags, and no semantic-version ceremony is required before the first production release.
- Preserve the existing rolling-update policy and health gates; no blue-green, canary, new AWS platform, CI platform, or new environment is introduced by U1.
