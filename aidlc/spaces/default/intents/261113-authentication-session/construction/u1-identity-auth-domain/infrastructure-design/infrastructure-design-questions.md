# Infrastructure Design — U1 Identity Auth Domain Questions

Answer each with a letter (A–D), or **X** plus your own words if none fits.

## Q1. [operations engineer] Where should Identity auth records be hosted and deployed?

Identity already runs as replicated Rust API/worker services on the production Docker Swarm and owns one Postgres schema/role. This Unit extends that service; production is the only environment tier.

- A. Extend existing Identity API/worker and Identity Postgres schema on Docker Swarm; use the existing production-only deployment topology
- B. Create a new Auth service and database so auth can scale independently
- C. Move Identity to AWS-managed compute and database as part of this Unit
- D. Keep all auth policy in the browser and add no Identity infrastructure
- X. Other (please specify)

[Answer]: A. Extend existing Identity API/worker and Identity Postgres schema on Docker Swarm; use the existing production-only deployment topology. **Mode:** guided

## Q2. [security engineer] How should provider credentials and internal service credentials reach Identity?

Project rules require Vault-backed secrets, no credentials in source/env files/logs, and gateway-to-Identity privileged calls use a separate service token from the unchanged user JWT.

- A. Use existing Supabase Vault/pgsodium materialized as Docker secrets and mounted through `*_FILE`; retain the separate existing service-token mechanism
- B. Put credentials directly in Swarm environment variables for easier rotation
- C. Bake provider/service credentials into the Identity image
- D. Use the user's JWT as the only authority for privileged gateway-to-Identity operations
- X. Other (please specify)

[Answer]: A. Use existing Supabase Vault/pgsodium materialized as Docker secrets and mounted through `*_FILE`; retain the separate existing service-token mechanism. **Mode:** guided

## Q3. [operations engineer] How should the Identity serving component expose health and observability signals?

NFR design requires `/healthz`, `/readyz`, `/metrics`, 99.9% monthly SLI and separate non-provider auth error alerting, but metrics/traces are not currently collected.

- A. Preserve existing endpoints and expose bounded-cardinality auth metrics; document collector/dashboard configuration as outside this Unit and do not claim signals are collected
- B. Add Prometheus, Loki, Tempo and Grafana services and scrape configuration to production Swarm in this Unit
- C. Emit logs only; remove the metrics endpoint because there is no collector
- D. Make provider availability a readiness dependency so all Identity requests stop during provider outages
- X. Other (please specify)

[Answer]: A. Preserve existing endpoints and expose bounded-cardinality auth metrics; document collector/dashboard configuration as outside this Unit and do not claim signals are collected. **Mode:** guided

## Q4. [release engineer] What CI/CD and rollback responsibility belongs to this Unit?

Production deployment is Docker Swarm, release images use commit SHA, health checks gate rolling updates, schema changes need paired sqlx migrations, and the release job's automatic down-migration path is an explicit intent deliverable. No CI platform is selected.

- A. Specify the Identity unit's build/test/scan and migration requirements for the platform-neutral release pipeline; preserve health-gated Swarm rolling updates and require matching down migration on rollback
- B. Choose GitHub Actions and create a new staging environment to validate Identity before production
- C. Skip dependency/container scanning because no pipeline platform is selected
- D. Require blue-green deployment and keep `:latest` image tags
- X. Other (please specify)

[Answer]: A. Specify the Identity unit's build/test/scan and migration requirements for the platform-neutral release pipeline; preserve health-gated Swarm rolling updates and require matching down migration on rollback. **Mode:** guided
