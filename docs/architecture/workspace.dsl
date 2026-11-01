workspace "Crypto Pocket Butler" "Read-only crypto portfolio aggregation and decision support" {

    model {

        portfolioOwner = person "Portfolio Owner" "Authenticated retail investor who connects read-only exchange and wallet accounts, builds portfolios, reviews valuation, drift, snapshots and recommendations." "Person"
        administrator = person "Admin" "Holder of the Supabase `app_metadata.role` admin claim who manages the chain and token registry, job schedules, audit queries and privacy requests." "Person"

        okxExchange = softwareSystem "OKX Exchange API" "Read-only exchange API supplying balances and holdings with API-key, secret and passphrase HMAC signing." "External"
        evmRpcProviders = softwareSystem "EVM JSON-RPC Providers" "Primary and fallback JSON-RPC endpoints per active EVM chain, with provider rotation, per-provider health, circuit breakers and shared rate-limit budgets, supplying native and ERC-20 balances and token metadata." "External"
        solanaRpc = softwareSystem "Solana RPC Provider" "JSON-RPC provider supplying SOL and SPL token balances and mint metadata." "External"
        priceDataProviders = softwareSystem "Price Data Providers (CoinGecko / CoinPaprika)" "Approved USD market data providers for spot price, volume, market capitalisation, rank and 24h change, with a shared per-provider request budget and provider fallback." "External"
        openClawAgent = softwareSystem "OpenClaw Agent" "External agent that composes the narrative portfolio brief from a versioned, redacted single-owner payload and publishes it to Notion on the product's behalf under an idempotency key." "External"
        notion = softwareSystem "Notion" "One-way destination workspace for daily and weekly portfolio briefs, reachable only through the OpenClaw Agent." "External"
        alertingChannel = softwareSystem "Operational Alerting Channel" "Pager/on-call alert consumer for missed runs, failed schedules, error-rate and stale-price alerts." "External"

        group "Supabase" {
            supabaseAuth = softwareSystem "Supabase Auth" "GoTrue identity provider: email/password, magic link, OAuth providers, MFA, JWT issuance and refresh, sessions, the JWKS endpoint, a custom access-token hook that projects roles into `app_metadata.role`, and auth hooks/webhooks for user lifecycle events." "External,Supabase"
            supabasePostgres = softwareSystem "Supabase Postgres" "Managed Postgres with logical database-per-service isolation: one schema and one role per service (identity, account, registry, market, portfolio, reporting, job, audit), no cross-schema grants, pgmq queues, transactional outbox and inbox dedup tables, and the Supavisor connection pooler." "External,Supabase"
            supabaseQueues = softwareSystem "Supabase Queues (pgmq)" "Durable Postgres-backed queues with visibility timeouts, archive and dead-letter handling and at-least-once delivery for domain events, job commands, run outcomes and privacy requests." "External,Supabase"
            supabaseStorage = softwareSystem "Supabase Storage" "Private buckets for audit export and retention archives, accessed only through signed, access-controlled URLs." "External,Supabase"
            supabaseVault = softwareSystem "Supabase Vault" "pgsodium-backed secrets store for exchange credentials, provider keys and service credentials, with versioned encryption and no plaintext exposure to logs or events." "External,Supabase"
            supabaseCron = softwareSystem "Supabase Cron (pg_cron)" "Scheduled SQL that enqueues job.tick into Supabase Queues, runs retention triggers and hosts the dead-man watchdog that calls the alerting channel through pg_net." "External,Supabase"
            supabaseLogs = softwareSystem "Supabase Logs & Metrics" "Platform logs, metrics, log drains and alerts for every service, with correlation and trace context." "External,Supabase"
        }

        cpb = softwareSystem "Crypto Pocket Butler" "Single-tenant MVP platform that aggregates read-only holdings, values portfolios in USD, computes drift, proposes human-reviewed recommendations and publishes scheduled briefs." {

            reactSpa = container "React Web Client (SPA)" "Authenticated user interface for dashboard, accounts, portfolios, valuation, drift, snapshots, recommendations, reporting preferences and admin screens. Uses Supabase Auth with @supabase/supabase-js for sign-up, sign-in, OAuth, magic link, MFA and PKCE, keeps tokens in memory only, refreshes sessions directly against Supabase Auth and calls only the API Gateway for application data." "React 18 + TypeScript 5, Vite, TanStack Query, @supabase/supabase-js, Recharts" "Container"

            apiGateway = container "API Gateway (BFF)" "Single public entry point and stateless edge BFF. Validates Supabase JWTs locally against the Supabase Auth JWKS endpoint with a cached JWKS in memory and Supabase Postgres, resolves principals from a Supabase Postgres cache and calls Identity's ensure-user endpoint only on a cache miss, enforces deny-by-default routing and admin-claim gating, applies rate limits, CORS, security headers and correlation IDs, mints a signed internal principal assertion, and routes to owning services over mTLS. Owns no business data and holds no server-side session store." "Rust, Axum, tower, jsonwebtoken, reqwest" "Container" {
                group "Transport" {
                    gatewayHttpRouter = component "Public HTTP Router" "Exposes versioned /api/v1 routes, health and readiness endpoints and transparently proxies requests to owning services." "Axum Router" "Transport"
                    gatewayHealthHandler = component "Health and Readiness Endpoints" "Exposes liveness and readiness probes and reports dependency readiness for the Supabase Auth JWKS and Identity." "Axum" "Transport"
                    gatewayAuthMiddleware = component "JWT Authentication Middleware" "Extracts Supabase bearer tokens, rejects unauthenticated access with 401 and attaches the validated principal." "Axum middleware, tower" "Transport"
                    gatewayAdminGuard = component "Admin Role Guard Middleware" "Gates administrative routes on the Supabase `app_metadata.role` admin claim projected by the Custom Access Token Hook and never treats portfolio ownership as administrative rights." "Axum middleware" "Transport"
                    gatewaySseProxy = component "SSE Progress Proxy" "Authenticates the SSE stream with a short-lived signed stream ticket (never a token in the URL) and proxies the bulk account sync progress stream." "Axum SSE" "Transport"
                    gatewayIdentityEventConsumer = component "Identity Event Consumer" "Consumes identity user, role and status events from Supabase Queues and evicts cached principals." "Supabase Queues (pgmq)" "Transport"
                }
                group "Application" {
                    gatewayValidateTokenUseCase = component "Validate Token Use Case" "Validates Supabase JWT signature, issuer, audience and expiry locally with a documented clock-skew allowance." "Rust" "Application"
                    gatewayValidatePrincipalUseCase = component "Validate Principal Status Use Case" "Resolves the principal from the Supabase Postgres cache. On first sighting it calls Identity's ensure-user endpoint over mTLS with the Supabase access token. On a later cache miss it presents the signed internal principal assertion carrying the Supabase user UUID and owner UUID. Denies disabled internal users despite a valid token." "Rust" "Application"
                    gatewayEnforceRoleUseCase = component "Enforce Role Use Case" "Applies deny-by-default route and role policy, including admin-claim gating and ownership passthrough." "Rust" "Application"
                    gatewayRouteRequestUseCase = component "Route Request Use Case" "Routes the request to the owning service over mTLS with correlation, rate-limit policy and signed principal context." "Rust" "Application"
                    gatewayIssueStreamTicketUseCase = component "Issue Stream Ticket Use Case" "Issues and verifies short-lived signed stream tickets so EventSource connections are authenticated without a URL token." "Rust" "Application"
                }
                group "Domain" {
                    gatewayAuthenticatedPrincipal = component "Authenticated Principal" "Validated Supabase subject, internal owner UUID, roles and status carried on every request." "Rust" "Domain"
                    gatewayPrincipalStatus = component "Principal Status" "Active or disabled status used to block disabled users at the edge." "Rust" "Domain"
                    gatewayRolePolicy = component "Role Policy" "Portfolio Owner, Admin and System role rules derived from the Supabase `app_metadata.role` claim for protected and administrative routes." "Rust" "Domain"
                    gatewayRateLimitPolicy = component "Rate Limit Policy" "Per-user and per-IP limits, with stricter limits on admin and security-sensitive routes." "Rust" "Domain"
                    gatewayInternalPrincipalAssertion = component "Signed Principal Assertion" "Short-lived signed owner UUID, roles, status and correlation id asserted to every internal callee for per-endpoint authorization. User-originated service-to-service hops forward this assertion unchanged. Scheduled and recovery hops carry a System-role assertion bound to the run key, attempt, lease epoch, target audience and permitted owner set; the owning service obtains it from the Job Orchestration run-assertion exchange and it never travels inside an event payload." "Rust" "Domain"
                    gatewayFirstSightingPolicy = component "First-Sighting Policy" "A cache hit returns immediately; a cache miss calls Identity's ensure-user endpoint over mTLS with the Supabase access token; a lookup error fails closed." "Rust" "Domain"
                }
                group "Infrastructure" {
                    gatewayJwksCache = component "Cached JWKS Store" "Caches Supabase Auth signing keys in memory and in Supabase Postgres with stale-while-revalidate and a bounded grace period for Supabase Auth outages." "Rust, Supabase Postgres, jsonwebtoken" "Infrastructure"
                    gatewayIdentityClient = component "Identity Service Client" "Calls Identity's internal ensure-user endpoint over mTLS with the Supabase access token on first sighting and the lookup endpoint with the signed principal assertion on later cache misses." "Rust, reqwest, mTLS" "Infrastructure"
                    gatewayPrincipalCache = component "Supabase Postgres Principal Cache" "Caches owner UUID, status and roles by Supabase user UUID with a short TTL, and is evicted by identity events." "Supabase Postgres" "Infrastructure"
                    gatewayRateLimiter = component "Postgres Atomic-Counter Rate Limiter" "Atomic-counter rate limiting in Supabase Postgres for per-user and per-IP limits." "Supabase Postgres" "Infrastructure"
                    gatewayAssertionSigner = component "Principal Assertion Signer" "Mints the signed internal principal assertion attached to every routed request. Resolves signing keys through an in-memory cache with a rotation overlap window and fails closed when no signing key is available." "Rust, Ed25519" "Infrastructure"
                    gatewayAuditPublisher = component "Supabase Queues Audit Publisher" "Publishes security.access_denied and security.rate_limited audit events to Supabase Queues." "Supabase Queues (pgmq)" "Infrastructure"
                    gatewayAuditBuffer = component "Fail-Closed Audit Channel" "Bounded in-memory channel with synchronous publish-and-acknowledge: a request does not complete until Supabase Queues has acknowledged its security audit record, and the gateway fails closed rather than silently dropping it. pgmq visibility timeouts and archive provide dead-letter handling. It holds no durable state, so there is no local-disk spool to lose; the residual window is the bounded publish timeout before the fail-closed response." "Rust, in-memory channel" "Infrastructure"
                    gatewayStreamTicketSigner = component "Stream Ticket Signer" "Signs short-lived stream tickets bound to the owner and sync run with the private half of a dedicated stream-ticket key pair resolved from Supabase Vault; only the public verification key is readable by Account." "Rust, Ed25519" "Infrastructure"
                    gatewaySecretsClient = component "Supabase Vault Secret Client" "Resolves internal-call credentials and assertion- and ticket-signing keys from Supabase Vault. Signing-key paths are readable only by the gateway and Job Orchestration; verification-key paths are readable by every `Principal Assertion Verifier`." "Supabase Vault (pgsodium)" "Infrastructure"
                }
                gatewayHttpRouter -> gatewayValidateTokenUseCase "Delegates every protected request for token validation" "in-process"
                gatewayHttpRouter -> gatewayValidatePrincipalUseCase "Resolves the principal status and roles after token validation" "in-process"
                gatewayHealthHandler -> gatewayJwksCache "Reports JWKS cache readiness" "in-process"
                gatewayHealthHandler -> gatewayIdentityClient "Reports Identity reachability" "in-process"
                gatewayAuthMiddleware -> gatewayValidateTokenUseCase "Invokes token validation on each request" "in-process"
                gatewayAdminGuard -> gatewayEnforceRoleUseCase "Requires the admin claim for administrative routes" "in-process"
                gatewaySseProxy -> gatewayIssueStreamTicketUseCase "Verifies the short-lived stream ticket" "in-process"
                gatewaySseProxy -> gatewayRouteRequestUseCase "Routes the SSE sync-progress stream" "in-process"
                gatewayHttpRouter -> gatewayIssueStreamTicketUseCase "Issues a short-lived stream ticket to an authenticated sync request" "in-process"
                gatewayHttpRouter -> gatewayRouteRequestUseCase "Routes every non-SSE public request through the shared rate-limit, assertion and routing path" "in-process"
                gatewayValidateTokenUseCase -> gatewayJwksCache "Reads cached signing keys" "in-process"
                gatewayValidateTokenUseCase -> gatewayAuthenticatedPrincipal "Produces the authenticated principal" "in-process"
                gatewayValidatePrincipalUseCase -> gatewayPrincipalCache "Reads cached principal status and roles" "Supabase Postgres client"
                gatewayValidatePrincipalUseCase -> gatewayIdentityClient "Ensures the internal user on first sighting and resolves later cache misses with the signed principal assertion" "mTLS REST client"
                gatewayValidatePrincipalUseCase -> gatewayFirstSightingPolicy "Applies the cache-hit or ensure-user branch" "in-process"
                gatewayValidatePrincipalUseCase -> gatewayPrincipalStatus "Checks active or disabled status" "in-process"
                gatewayIdentityEventConsumer -> gatewayPrincipalCache "Evicts cached principals on identity changes" "Supabase Queues consumer" {
                    tags "Async"
                }
                gatewayEnforceRoleUseCase -> gatewayRolePolicy "Evaluates deny-by-default role policy" "in-process"
                gatewayEnforceRoleUseCase -> gatewayAuditPublisher "Publishes access-denied audit events" "Supabase Queues publisher" {
                    tags "Async"
                }
                gatewayRouteRequestUseCase -> gatewayEnforceRoleUseCase "Evaluates the deny-by-default route and role policy before routing every protected request" "in-process"
                gatewayRouteRequestUseCase -> gatewayRateLimiter "Applies per-user and per-IP limits" "Supabase Postgres-backed limiter"
                gatewayRouteRequestUseCase -> gatewayRateLimitPolicy "Evaluates per-user and per-IP rate-limit policy" "in-process"
                gatewayRouteRequestUseCase -> gatewayPrincipalStatus "Re-checks status for sensitive routes" "in-process"
                gatewayRouteRequestUseCase -> gatewayAssertionSigner "Attaches the signed principal assertion" "in-process"
                gatewayAssertionSigner -> gatewayInternalPrincipalAssertion "Mints the internal assertion" "in-process"
                gatewayIssueStreamTicketUseCase -> gatewayStreamTicketSigner "Signs the stream ticket bound to owner and run" "in-process"
                gatewayAssertionSigner -> gatewaySecretsClient "Resolves the assertion-signing key" "Supabase Vault client"
                gatewayStreamTicketSigner -> gatewaySecretsClient "Resolves the shared ticket-signing key" "Supabase Vault client"
                gatewaySecretsClient -> supabaseVault "Reads internal-call credentials and signing keys" "Supabase Vault (pgsodium)"
                gatewayAuditPublisher -> gatewayAuditBuffer "Publishes synchronously and waits for the Supabase Queues acknowledgement before the request completes" "in-process"
                gatewayJwksCache -> supabaseAuth "Fetches and caches Supabase Auth signing keys" "HTTPS JWKS"
            }

            identityService = container "Identity Service" "Owns the internal identity model and trust boundary. Validates Supabase JWTs as defence in depth, receives Supabase Auth hooks for sign-up, sign-in, sign-out and user-update, verifies the standard-webhooks signature and idempotency, lazily upserts the internal user keyed by the Supabase auth.users.id UUID through an ensure-user endpoint that accepts the Supabase access token over mTLS, denies disabled users, projects roles from the `app_metadata.role` claim, issues the stable owner UUID, records session events, revokes sessions through the Supabase Admin API, applies a documented retention period to personal data, and owns user deletion and anonymisation policy. Holds no passwords, MFA state or server-side session store." "Rust, Axum, jsonwebtoken, sqlx" "Container" {
                group "Transport" {
                    identityRestHandlers = component "Public REST Handlers" "Serves GET /api/v1/me, logout, admin user-status endpoints and user deletion/anonymisation requests." "Axum" "Transport"
                    identityInternalHandlers = component "Internal REST Handlers" "Serves internal user lookup endpoints over mTLS for callers that already hold a signed assertion." "Axum, mTLS" "Transport"
                    identityInternalAuthGuard = component "Internal Principal Guard" "Validates the signed internal assertion (gateway user assertion or the Job Orchestration run-scoped System-role assertion) and enforces owner scoping and admin roles on internal lookup endpoints. Downstream services forward the inbound assertion unchanged; they never re-mint their own." "Axum middleware, mTLS" "Transport"
                    identityEnsureUserHandler = component "Internal Ensure-User Handler" "Serves the first-sighting ensure-user endpoint over mTLS. Accepts the Supabase access token plus the gateway's mTLS service identity and idempotently upserts the internal user keyed by the Supabase auth.users.id UUID." "Axum, mTLS" "Transport"
                    identityAuthHookHandler = component "Supabase Auth Hook Handler" "Receives sign-up, sign-in, sign-out and user-update hooks from Supabase Auth only after verifying the standard-webhooks signature and idempotency." "Axum" "Transport"
                    identityBearerGuard = component "Supabase JWT Bearer Guard" "Extracts and validates Supabase JWT bearer tokens for every public REST call and maps failures to non-disclosing responses." "Axum extractor" "Transport"
                    identityAdminGuard = component "Admin Role Guard Middleware" "Re-checks the Supabase `app_metadata.role` admin claim on user-status and deletion routes and never treats portfolio ownership as administrative rights." "Axum middleware" "Transport"
                    identityPrivacyConsumer = component "Privacy Request Consumer" "Consumes export and erasure requests from Supabase Queues and routes them to identity export and deletion use cases." "Supabase Queues (pgmq)" "Transport"
                    identityJobConsumer = component "Job Command Consumer" "Consumes identity.retention commands from Supabase Queues with logical run-key and attempt idempotency, exchanges the command for a short-lived run-scoped assertion over mTLS before executing, and checks for a cancel command between subject batches." "Supabase Queues (pgmq)" "Transport"
                    identityHealthHandler = component "Health and Readiness Endpoints" "Exposes liveness and readiness probes and reports repository and Supabase Admin API readiness." "Axum" "Transport"
                }
                group "Application" {
                    identityEnsureUserUseCase = component "Ensure User Use Case" "Validates the Supabase access token and idempotently upserts the internal user keyed by the Supabase auth.users.id UUID on first sighting; concurrent requests for the same UUID return the same mapping." "Rust" "Application"
                    identityProjectRolesUseCase = component "Project Roles Use Case" "Projects Portfolio Owner, Admin and System roles from the Supabase `app_metadata.role` claim issued by the Custom Access Token Hook." "Rust" "Application"
                    identityRecordSessionEventUseCase = component "Record Session Event Use Case" "Records verified sign-up, sign-in, sign-in-error, sign-out and user-update events from Supabase Auth hooks for the audit trail." "Rust" "Application"
                    identityRevokeSessionsUseCase = component "Revoke Sessions Use Case" "Revokes all sessions and refresh tokens for a user through the Supabase Admin API." "Rust" "Application"
                    identitySetUserStatusUseCase = component "Set User Status Use Case" "Disables, deactivates or re-enables a user through the Supabase Admin API and publishes the status-change event consumed by the gateway." "Rust" "Application"
                    identityDeleteUserUseCase = component "Delete or Anonymise User Use Case" "Applies user-deletion policy, anonymises the internal identity, deletes the Supabase auth user through the Admin API and coordinates erasure requests to owning services." "Rust" "Application"
                    identityExportUserDataUseCase = component "Export User Data Use Case" "Assembles the identity portion of a personal-data export." "Rust" "Application"
                    identityLookupUserUseCase = component "Lookup User Use Case" "Resolves a user by Supabase user UUID for gateway principal resolution, or by owner UUID for internal export, anonymisation and audit callers, with owner scoping." "Rust" "Application"
                    identityApplyRetentionUseCase = component "Apply Identity Retention Use Case" "Deletes or anonymises identity records past their documented personal-data retention period, subject to legal hold, checks for cancellation between subject batches, and emits retention-applied and run-outcome events." "Rust" "Application"
                    identityEnforceAdminRoleUseCase = component "Enforce Admin Role Use Case" "Applies the identity admin policy to the verified assertion before any administrative operation runs." "Rust" "Application"
                }
                group "Domain" {
                    identityUserAggregate = component "Internal User Aggregate" "User keyed by the Supabase auth.users.id UUID with stable owner UUID, profile and active or disabled status." "Rust" "Domain"
                    identityRoleAssignment = component "Role Assignment" "Portfolio Owner, Admin and System role projection from the Supabase `app_metadata.role` claim with explicit assignment rules." "Rust" "Domain"
                    identityTokenPolicy = component "Token Validation Policy" "Supabase JWT issuer, audience, expiry and clock-skew rules applied to every token." "Rust" "Domain"
                    identityProvisioningPolicy = component "Principal Provisioning Policy" "First-sighting upsert keyed by the Supabase auth.users.id UUID and disabled-user denial rules." "Rust" "Domain"
                    identityAuthHookPolicy = component "Auth Hook Verification Policy" "Standard-webhooks signature, replay-window and idempotency rules for Supabase Auth hooks." "Rust" "Domain"
                    identityPrivacyPolicy = component "User Privacy Policy" "User-deletion cascade, anonymisation and legal-hold rules applied across owning services." "Rust" "Domain"
                    identityAdminPolicy = component "Admin Authorization Policy" "Requires the Supabase `app_metadata.role` admin claim projected by the Custom Access Token Hook for identity administration such as user status changes and deletion, and rejects portfolio-ownership-only callers." "Rust" "Domain"
                }
                group "Infrastructure" {
                    identityUserRepository = component "Supabase Postgres User Repository" "Stores users, Supabase user UUID mapping, roles, status and retention state with an outbox and inbox dedup table in the identity schema." "Rust, sqlx" "Infrastructure"
                    identitySupabaseAdminClient = component "Supabase Admin API Client" "Calls the Supabase Admin API to revoke sessions, update `app_metadata` and delete users." "Rust, reqwest" "Infrastructure"
                    identityPrincipalCache = component "Principal Cache Eviction Signal" "Publishes identity-change eviction signals for the gateway-owned principal cache namespace in Supabase Postgres; Identity does not write the gateway cache directly." "Supabase Queues (pgmq)" "Infrastructure"
                    identityEventPublisher = component "Supabase Queues Identity Event Publisher" "Writes identity events through a transactional outbox and publishes user, role, session and deletion events with pgmq at-least-once delivery." "Supabase Queues (pgmq)" "Infrastructure"
                    identityRunOutcomePublisher = component "Supabase Queues Run Outcome Publisher" "Publishes identity.retention run outcomes and heartbeats." "Supabase Queues (pgmq)" "Infrastructure"
                    identityAssertionVerifier = component "Principal Assertion Verifier" "Resolves the gateway user-assertion and Job Orchestration System-assertion public keys from Supabase Vault, caches them with a bounded refresh, verifies signature, issuer, expiry, audience and correlation id, and rejects any other issuer." "Rust, Ed25519" "Infrastructure"
                    identitySecretsClient = component "Supabase Vault Secret Client" "Resolves Supabase Auth Admin API credentials, trust material and database credentials from Supabase Vault." "Supabase Vault (pgsodium)" "Infrastructure"
                }
                identityRestHandlers -> identityLookupUserUseCase "Resolves the authenticated owner for public profile routes" "in-process"
                identityRestHandlers -> identityBearerGuard "Authenticates every public REST call including /me, logout and admin endpoints" "in-process"
                identityRestHandlers -> identityAdminGuard "Re-checks the admin claim for user-status and deletion routes" "in-process"
                identityRestHandlers -> identityAssertionVerifier "Verifies the gateway assertion on public REST routes" "in-process"
                identityAdminGuard -> identityEnforceAdminRoleUseCase "Delegates the admin check to the application use case" "in-process"
                identityEnforceAdminRoleUseCase -> identityAdminPolicy "Applies the admin authorization policy" "in-process"
                identityEnforceAdminRoleUseCase -> identityRoleAssignment "Reads the projected admin role" "in-process"
                identityRestHandlers -> identityProjectRolesUseCase "Returns roles for the current principal" "in-process"
                identityRestHandlers -> identityRevokeSessionsUseCase "Revokes the user's sessions through the Supabase Admin API" "in-process"
                identityRestHandlers -> identitySetUserStatusUseCase "Disables, deactivates or re-enables a user" "in-process"
                identityRestHandlers -> identityDeleteUserUseCase "Deletes or anonymises a user" "in-process"
                identityInternalHandlers -> identityInternalAuthGuard "Requires a signed service principal" "in-process"
                identityInternalAuthGuard -> identityAssertionVerifier "Verifies the assertion with the Supabase Vault-resolved public key and cache" "in-process"
                identityInternalAuthGuard -> identityLookupUserUseCase "Allows owner-scoped internal lookups" "in-process"
                identityEnsureUserHandler -> identityEnsureUserUseCase "Upserts the first-sighted Supabase user keyed by auth.users.id" "in-process"
                identityAuthHookHandler -> identityAuthHookPolicy "Verifies the standard-webhooks signature, replay window and idempotency" "in-process"
                identityAuthHookHandler -> identityRecordSessionEventUseCase "Records verified sign-up, sign-in, sign-out and user-update hooks" "in-process"
                identityAuthHookHandler -> identitySetUserStatusUseCase "Applies user-update hooks that disable or re-enable a user" "in-process"
                identityJobConsumer -> identityApplyRetentionUseCase "Runs scheduled identity retention idempotently per run key and attempt and stops between subject batches on cancel" "Supabase Queues consumer" {
                    tags "Async"
                }
                identityBearerGuard -> identityLookupUserUseCase "Validates the Supabase token and resolves the existing principal; provisioning is restricted to the ensure-user endpoint" "in-process"
                identityPrivacyConsumer -> identityExportUserDataUseCase "Routes identity export requests" "Supabase Queues consumer" {
                    tags "Async"
                }
                identityPrivacyConsumer -> identityDeleteUserUseCase "Routes erasure and anonymisation requests" "Supabase Queues consumer" {
                    tags "Async"
                }
                identityHealthHandler -> identityUserRepository "Reports repository readiness" "in-process"
                identityHealthHandler -> identitySupabaseAdminClient "Reports Supabase Admin API reachability" "in-process"
                identityEnsureUserUseCase -> identityUserRepository "Upserts the internal user keyed by the Supabase auth.users.id UUID" "repository port"
                identityLookupUserUseCase -> identityUserRepository "Loads the user by Supabase user UUID or owner UUID" "repository port"
                identityEnsureUserUseCase -> identityTokenPolicy "Validates Supabase issuer, audience and expiry" "in-process"
                identityEnsureUserUseCase -> identityProvisioningPolicy "Applies first-sighting upsert rules" "in-process"
                identityEnsureUserUseCase -> identityEventPublisher "Publishes user provisioned events" "Supabase Queues publisher" {
                    tags "Async"
                }
                identityProjectRolesUseCase -> identityRoleAssignment "Projects roles from the app_metadata.role claim" "in-process"
                identityProjectRolesUseCase -> identityPrincipalCache "Publishes a role-change eviction signal" "Supabase Queues publisher" {
                    tags "Async"
                }
                identityRecordSessionEventUseCase -> identityAuthHookPolicy "Verifies signature, replay and idempotency" "in-process"
                identityRecordSessionEventUseCase -> identityEventPublisher "Publishes session events" "Supabase Queues publisher" {
                    tags "Async"
                }
                identityRevokeSessionsUseCase -> identitySupabaseAdminClient "Revokes refresh tokens and sessions through the Supabase Admin API" "HTTPS"
                identityRevokeSessionsUseCase -> identityEventPublisher "Publishes session revoked events" "Supabase Queues publisher" {
                    tags "Async"
                }
                identitySetUserStatusUseCase -> identityUserAggregate "Applies the status change" "in-process"
                identitySetUserStatusUseCase -> identityRoleAssignment "Re-checks the admin role" "in-process"
                identitySetUserStatusUseCase -> identityUserRepository "Persists status" "repository port"
                identitySetUserStatusUseCase -> identitySupabaseAdminClient "Updates app_metadata through the Supabase Admin API" "HTTPS"
                identitySetUserStatusUseCase -> identityEventPublisher "Publishes the status-change event consumed by the gateway" "Supabase Queues publisher" {
                    tags "Async"
                }
                identityDeleteUserUseCase -> identityUserAggregate "Anonymises the internal identity" "in-process"
                identityDeleteUserUseCase -> identityPrivacyPolicy "Applies the deletion cascade and legal-hold rules" "in-process"
                identityDeleteUserUseCase -> identityRoleAssignment "Re-checks the admin role" "in-process"
                identityDeleteUserUseCase -> identityUserRepository "Anonymises and persists the user" "repository port"
                identityDeleteUserUseCase -> identitySupabaseAdminClient "Deletes the Supabase auth user through the Admin API" "HTTPS"
                identityDeleteUserUseCase -> identityEventPublisher "Publishes user.deleted and erasure requests" "Supabase Queues publisher" {
                    tags "Async"
                }
                identityExportUserDataUseCase -> identityUserRepository "Reads identity data for export" "repository port"
                identityExportUserDataUseCase -> identityEventPublisher "Emits the identity export part" "Supabase Queues publisher" {
                    tags "Async"
                }
                identityApplyRetentionUseCase -> identityUserRepository "Deletes or anonymises expired identity records" "repository port"
                identityApplyRetentionUseCase -> identityPrivacyPolicy "Applies retention and legal-hold rules" "in-process"
                identityApplyRetentionUseCase -> identityAdminPolicy "Re-checks the admin claim for interactive runs or the run-scoped System assertion for scheduled runs" "in-process"
                identityApplyRetentionUseCase -> identityEventPublisher "Publishes retention-applied and erasure events" "Supabase Queues publisher" {
                    tags "Async"
                }
                identityApplyRetentionUseCase -> identityRunOutcomePublisher "Publishes the retention run outcome and heartbeat" "Supabase Queues publisher" {
                    tags "Async"
                }
                identityAssertionVerifier -> supabaseVault "Resolves the assertion verification public keys" "Supabase Vault (pgsodium)"
                identityUserAggregate -> identityProvisioningPolicy "Applies provisioning and denial rules" "in-process"
                identityUserRepository -> identitySecretsClient "Resolves database credentials from Supabase Vault" "Supabase Vault client"
                identitySecretsClient -> supabaseVault "Reads auth, trust and database secret references" "Supabase Vault (pgsodium)"
            }

            accountService = container "Account Service" "Owns read-only exchange and wallet accounts and the complete holdings sync pipeline: type-specific validation, Supabase Vault-backed credential encryption and masking, public-address-only wallets, create/update/activate/deactivate/delete semantics, idempotent single and bulk syncs, versioned holdings and lifecycle events, decimal normalisation, zero filtering, cross-chain merging, fault isolation, sync-status reporting and an admin-triggered registry-projection rebuild. Never accepts private keys or seed phrases." "Rust, Axum, sqlx, rust_decimal, alloy, solana-client" "Container" {
                group "Transport" {
                    accountRestHandlers = component "Account REST Handlers" "Serves account create, update, activation, deletion, single sync and sync-status endpoints." "Axum" "Transport"
                    accountInternalHandlers = component "Internal Account Handlers" "Serves internal owner account and holdings queries over mTLS." "Axum, mTLS" "Transport"
                    accountInternalAuthGuard = component "Internal Principal Guard" "Validates the signed internal assertion (gateway user assertion or Job Orchestration System-role assertion) and enforces owner scoping and admin roles on internal endpoints. Forwarding rule: user-originated hops forward the gateway assertion unchanged; scheduled and recovery hops carry the System-role assertion from the run command." "Axum middleware, mTLS" "Transport"
                    accountRebuildHandler = component "Operations Rebuild Handler" "Serves the admin-triggered, assertion-authorised registry-projection rebuild endpoint over mTLS." "Axum, mTLS" "Transport"
                    accountSseHandler = component "SSE Sync Progress Handler" "Verifies the short-lived stream ticket through the shared verifier and streams bulk sync progress and per-account outcomes." "Axum SSE" "Transport"
                    accountJobConsumer = component "Job Command Consumer" "Consumes scheduled account-sync commands from Supabase Queues with idempotency keyed on the logical run key (job + scheduled period) plus attempt number, honours cancel commands between shards, and hands them to the all-accounts fan-out; pgmq visibility timeout and archive provide dead-letter handling." "Supabase Queues (pgmq)" "Transport"
                    accountRegistryConsumer = component "Registry Event Consumer" "Consumes versioned registry asset, contract and chain events from Supabase Queues into the account registry projection, applying only newer aggregate versions with inbox dedup." "Supabase Queues (pgmq)" "Transport"
                    accountPrivacyConsumer = component "Privacy Request Consumer" "Consumes export and erasure requests for account-owned data from Supabase Queues." "Supabase Queues (pgmq)" "Transport"
                    accountHealthHandler = component "Health and Readiness Endpoints" "Exposes liveness and readiness probes and reports repository readiness." "Axum" "Transport"
                }
                group "Application" {
                    accountCreateAccountUseCase = component "Create Account Use Case" "Validates type-specific input, encrypts credentials through Supabase Vault and persists exchange or wallet accounts." "Rust" "Application"
                    accountUpdateAccountUseCase = component "Update Account Use Case" "Edits display name, enabled chains and credentials and re-encrypts changed credentials through the Supabase Vault credential cipher." "Rust" "Application"
                    accountSetActiveUseCase = component "Set Account Active Use Case" "Activates or deactivates an account and publishes the lifecycle event." "Rust" "Application"
                    accountSyncAccountUseCase = component "Sync Account Use Case" "Retrieves, normalises and atomically replaces holdings for one account with fault isolation, and reports item-level progress; it does not publish the run outcome, which belongs to the run owner." "Rust" "Application"
                    accountGetSyncStatusUseCase = component "Get Sync Status Use Case" "Returns the owner's per-account last-sync outcome, timestamps and error summary from the recorded sync runs." "Rust" "Application"
                    accountAllAccountsFanOutUseCase = component "All-Accounts Sync Fan-Out Use Case" "Enumerates (and shards) every active account for accounts.sync, reuses the same enumeration policy for interactive bulk sync, runs bounded per-account syncs with per-owner fairness and idempotency on (run key, attempt), isolates per-account failures so one account cannot abort siblings, publishes the single aggregated run outcome and heartbeat for the run, and stops between per-owner batches when the run is cancelled." "Rust" "Application"
                    accountBulkSyncUseCase = component "Bulk Sync Use Case" "Runs the shared all-accounts fan-out without aborting sibling accounts and streams aggregate progress; an interactive bulk sync reports item progress and never emits a scheduled-run outcome." "Rust" "Application"
                    accountDeleteAccountUseCase = component "Delete Account Use Case" "Loads the account, re-checks the asserted owner, deletes it, marks its data for erasure and publishes account.deleted; portfolio links are owned and marked inactive by Portfolio Service on deactivation and removed only on deletion." "Rust" "Application"
                    accountInternalQueryUseCase = component "Internal Account Query Use Case" "Serves owner-scoped account and holdings queries to internal callers." "Rust" "Application"
                    accountApplyRegistryEventUseCase = component "Apply Registry Event Use Case" "Applies registry asset, token and chain events to the local registry projection only when the aggregate version is newer than the stored one, and on chain.deactivated excludes that chain from subsequent syncs and marks its holdings stale." "Rust" "Application"
                    accountRebuildRegistryProjectionUseCase = component "Rebuild Registry Projection Use Case" "Rebuilds the registry projection from the registry snapshot with timeout, bounded retry, circuit breaker and a degraded read-only mode." "Rust" "Application"
                    accountExportDataUseCase = component "Export Account Data Use Case" "Assembles the account and holdings portion of a personal-data export." "Rust" "Application"
                    accountEraseDataUseCase = component "Erase Account Data Use Case" "Anonymises account, credential and sync data subject to legal hold and emits an erasure confirmation." "Rust" "Application"
                }
                group "Domain" {
                    accountAggregate = component "Account Aggregate" "Exchange or wallet account with owner, type, display name, active flag and chain associations." "Rust" "Domain"
                    accountHolding = component "Holding" "Normalised per-asset quantity with total, available and frozen values, chain scope and a mapping state of mapped or unmapped. Price-derived status (priced, stale, unpriced) belongs to Portfolio's Valuation Line Status because Account owns no price data." "Rust, rust_decimal" "Domain"
                    accountBalanceNormalizer = component "Balance Normaliser" "Applies registered decimals, filters zero balances, merges same assets across chains and validates available plus frozen equals total." "Rust" "Domain"
                    accountCredentialPolicy = component "Read-Only Credential Policy" "Rejects private keys and seed phrases, requires read-only permissions and masks credential material." "Rust" "Domain"
                    accountHoldingsEventPolicy = component "Holdings Event Versioning Policy" "Assigns accountId, the account aggregate version incremented by every sync, and occurredAt to holdings events so consumers apply only newer state. Holdings and lifecycle events share the same account aggregate version sequence." "Rust" "Domain"
                    accountLifecycleEventPolicy = component "Account Lifecycle Event Versioning Policy" "Assigns accountId, the same account aggregate version incremented by every sync and lifecycle change, and occurredAt to account.created, account.updated, account.activated, account.deactivated and account.deleted, so Portfolio compares both event families against one monotonic account version." "Rust" "Domain"
                    accountAdminPolicy = component "Admin Authorization Policy" "Requires the Supabase `app_metadata.role` admin claim projected by the Custom Access Token Hook for operations rebuild and other administrative Account routes, and rejects portfolio-ownership-only callers." "Rust" "Domain"
                }
                group "Infrastructure" {
                    accountRepository = component "Supabase Postgres Account Repository" "Stores accounts, encrypted credentials, wallet chains, holdings, sync runs and the registry projection in the account schema; never cascades into other services' data." "Rust, sqlx" "Infrastructure"
                    accountSyncLock = component "Per-Account Sync Lock" "Supabase Postgres advisory lock plus compare-and-set on the account aggregate version, shared by scheduled and interactive sync, so overlapping syncs for one account cannot lose an update or emit the same aggregate version twice." "Supabase Postgres advisory locks" "Infrastructure"
                    accountCredentialCipher = component "Supabase Vault Credential Cipher" "Envelope-encrypts credentials with a per-credential DEK stored in Supabase Vault (pgsodium), records key versions and re-encrypts on edit." "Supabase Vault (pgsodium)" "Infrastructure"
                    accountOkxConnector = component "OKX Read-Only Connector" "Signs read-only requests and reads balances, and never trades or withdraws." "Rust, reqwest, HMAC" "Infrastructure"
                    accountEvmConnector = component "EVM JSON-RPC Connector" "Batches native and ERC-20 balance and metadata reads with primary/fallback provider rotation, retries and backoff." "Rust, alloy" "Infrastructure"
                    accountSolanaConnector = component "Solana RPC Connector" "Reads SOL and SPL balances and mint metadata for registered wallets." "Rust, solana-client" "Infrastructure"
                    accountEventPublisher = component "Supabase Queues Account Event Publisher" "Writes lifecycle, versioned holdings, unmapped, export and audit events through a transactional outbox to Supabase Queues." "Supabase Queues (pgmq)" "Infrastructure"
                    accountRunOutcomePublisher = component "Supabase Queues Run Outcome Publisher" "Publishes run start, end, outcome, affected counts and heartbeats for job runs; per-item progress travels on a distinct item-progress queue." "Supabase Queues (pgmq)" "Infrastructure"
                    accountRegistrySnapshotClient = component "Registry Snapshot Client" "Fetches a registry snapshot over mTLS for cold-start projection rebuild with timeout, bounded retry, circuit breaker and a degraded read-only mode." "Rust, reqwest, mTLS" "Infrastructure"
                    accountStreamTicketVerifier = component "Stream Ticket Verifier" "Verifies the gateway-signed short-lived stream ticket with the public half of a dedicated stream-ticket key pair resolved from Supabase Vault before any SSE progress is streamed; it never holds the signing key." "Rust, Ed25519" "Infrastructure"
                    accountAssertionVerifier = component "Principal Assertion Verifier" "Resolves the gateway user-assertion and Job Orchestration System-assertion public keys from Supabase Vault, caches them with a bounded refresh, verifies signature, issuer, expiry and correlation id, and rejects any other issuer." "Rust, Ed25519" "Infrastructure"
                }
                accountRestHandlers -> accountCreateAccountUseCase "Creates exchange and wallet accounts" "in-process"
                accountRestHandlers -> accountUpdateAccountUseCase "Updates accounts and re-encrypts credentials" "in-process"
                accountRestHandlers -> accountSetActiveUseCase "Activates or deactivates accounts" "in-process"
                accountRestHandlers -> accountDeleteAccountUseCase "Deletes accounts and publishes the lifecycle event" "in-process"
                accountRestHandlers -> accountSyncAccountUseCase "Triggers a single account sync on demand" "in-process"
                accountRestHandlers -> accountGetSyncStatusUseCase "Serves per-account last-sync outcome and status" "in-process"
                accountRestHandlers -> accountAssertionVerifier "Verifies the gateway assertion on public REST routes" "in-process"
                accountInternalHandlers -> accountInternalAuthGuard "Requires a signed service principal" "in-process"
                accountInternalAuthGuard -> accountAssertionVerifier "Verifies the assertion with the Supabase Vault-resolved public key and cache" "in-process"
                accountInternalAuthGuard -> accountInternalQueryUseCase "Allows asserted, owner-scoped internal queries" "in-process"
                accountInternalAuthGuard -> accountRebuildRegistryProjectionUseCase "Allows asserted admin rebuild requests" "in-process"
                accountRebuildHandler -> accountInternalAuthGuard "Requires a signed admin principal for the rebuild endpoint" "in-process"
                accountSseHandler -> accountStreamTicketVerifier "Verifies the stream ticket with the shared key" "in-process"
                accountSseHandler -> accountAssertionVerifier "Verifies the gateway assertion carried on the SSE request" "in-process"
                accountSseHandler -> accountBulkSyncUseCase "Streams bulk sync progress" "SSE" {
                    tags "Async"
                }
                accountJobConsumer -> accountAllAccountsFanOutUseCase "Runs scheduled account sync as an all-accounts fan-out idempotently per logical run key and attempt" "Supabase Queues consumer" {
                    tags "Async"
                }
                accountRegistryConsumer -> accountApplyRegistryEventUseCase "Applies registry changes to the local projection" "Supabase Queues consumer" {
                    tags "Async"
                }
                accountPrivacyConsumer -> accountExportDataUseCase "Routes export requests" "Supabase Queues consumer" {
                    tags "Async"
                }
                accountPrivacyConsumer -> accountEraseDataUseCase "Routes erasure requests" "Supabase Queues consumer" {
                    tags "Async"
                }
                accountHealthHandler -> accountRepository "Reports repository readiness" "in-process"
                accountCreateAccountUseCase -> accountAggregate "Creates the account aggregate" "in-process"
                accountCreateAccountUseCase -> accountRepository "Persists validated accounts" "repository port"
                accountCreateAccountUseCase -> accountCredentialPolicy "Enforces read-only credential policy" "in-process"
                accountCreateAccountUseCase -> accountCredentialCipher "Encrypts credentials before persistence" "Supabase Vault (pgsodium)"
                accountCreateAccountUseCase -> accountEventPublisher "Publishes account.created events" "Supabase Queues publisher" {
                    tags "Async"
                }
                accountCreateAccountUseCase -> accountLifecycleEventPolicy "Stamps aggregate id, version and occurredAt" "in-process"
                accountUpdateAccountUseCase -> accountAggregate "Applies updates to the aggregate" "in-process"
                accountUpdateAccountUseCase -> accountRepository "Persists account changes" "repository port"
                accountUpdateAccountUseCase -> accountCredentialPolicy "Re-validates read-only credentials" "in-process"
                accountUpdateAccountUseCase -> accountCredentialCipher "Re-encrypts changed credentials" "Supabase Vault (pgsodium)"
                accountUpdateAccountUseCase -> accountEventPublisher "Publishes account.updated events" "Supabase Queues publisher" {
                    tags "Async"
                }
                accountUpdateAccountUseCase -> accountLifecycleEventPolicy "Stamps the incremented aggregate version and occurredAt" "in-process"
                accountSetActiveUseCase -> accountAggregate "Flips the active flag" "in-process"
                accountSetActiveUseCase -> accountRepository "Persists activation state" "repository port"
                accountSetActiveUseCase -> accountEventPublisher "Publishes account.activated or account.deactivated events" "Supabase Queues publisher" {
                    tags "Async"
                }
                accountSetActiveUseCase -> accountLifecycleEventPolicy "Stamps the incremented aggregate version and occurredAt" "in-process"
                accountSyncAccountUseCase -> accountOkxConnector "Reads OKX balances" "HTTPS client"
                accountSyncAccountUseCase -> accountEvmConnector "Reads EVM balances" "JSON-RPC client"
                accountSyncAccountUseCase -> accountSolanaConnector "Reads Solana balances" "JSON-RPC client"
                accountSyncAccountUseCase -> accountBalanceNormalizer "Normalises quantities with registered decimals" "in-process"
                accountSyncAccountUseCase -> accountHolding "Produces normalised holdings" "in-process"
                accountSyncAccountUseCase -> accountHoldingsEventPolicy "Stamps accountId, version and occurredAt" "in-process"
                accountSyncAccountUseCase -> accountRepository "Atomically replaces holdings" "repository port"
                accountSyncAccountUseCase -> accountSyncLock "Acquires the per-account lock and allocates the next aggregate version" "advisory lock"
                accountSyncAccountUseCase -> accountEventPublisher "Publishes versioned account.holdings.replaced events" "Supabase Queues publisher" {
                    tags "Async"
                }
                accountSyncAccountUseCase -> accountRunOutcomePublisher "Publishes per-account sync progress on the item-progress queue, never the run outcome" "Supabase Queues publisher" {
                    tags "Async"
                }
                accountBulkSyncUseCase -> accountAllAccountsFanOutUseCase "Reuses the all-accounts enumeration and sharding while streaming progress" "in-process"
                accountAllAccountsFanOutUseCase -> accountRepository "Enumerates active accounts with sharding and per-owner fairness" "repository port"
                accountAllAccountsFanOutUseCase -> accountSyncAccountUseCase "Runs bounded per-account syncs, recording a failed account and continuing with the remaining accounts" "in-process"
                accountAllAccountsFanOutUseCase -> accountSyncLock "Uses the same per-account lock for scheduled and interactive bulk sync" "advisory lock"
                accountAllAccountsFanOutUseCase -> accountRunOutcomePublisher "Publishes the single aggregated run outcome, affected counts and heartbeat for the run" "Supabase Queues publisher" {
                    tags "Async"
                }
                accountGetSyncStatusUseCase -> accountRepository "Reads sync runs and per-account last outcomes" "repository port"
                accountRebuildRegistryProjectionUseCase -> accountAdminPolicy "Re-checks the admin claim before rebuilding" "in-process"
                accountRebuildRegistryProjectionUseCase -> accountRegistrySnapshotClient "Fetches the registry snapshot for a cold-start rebuild" "mTLS REST client"
                accountRebuildRegistryProjectionUseCase -> accountRepository "Replaces the local registry projection" "repository port"
                accountStreamTicketVerifier -> supabaseVault "Resolves the stream-ticket public verification key" "Supabase Vault (pgsodium)"
                accountAssertionVerifier -> supabaseVault "Resolves the assertion verification public keys" "Supabase Vault (pgsodium)"
                accountDeleteAccountUseCase -> accountRepository "Deletes the account and its local data" "repository port"
                accountDeleteAccountUseCase -> accountAggregate "Loads the account and re-checks the asserted owner before deletion" "in-process"
                accountDeleteAccountUseCase -> accountEventPublisher "Publishes account.deleted events" "Supabase Queues publisher" {
                    tags "Async"
                }
                accountDeleteAccountUseCase -> accountLifecycleEventPolicy "Stamps the terminal aggregate version and occurredAt" "in-process"
                accountInternalQueryUseCase -> accountRepository "Reads owner-scoped accounts and holdings" "repository port"
                accountApplyRegistryEventUseCase -> accountRepository "Persists projection updates" "repository port"
                accountApplyRegistryEventUseCase -> accountHolding "Marks holdings on a deactivated chain stale and excludes them from sync" "in-process"
                accountExportDataUseCase -> accountRepository "Reads account and holdings data for export" "repository port"
                accountExportDataUseCase -> accountEventPublisher "Emits the account export part" "Supabase Queues publisher" {
                    tags "Async"
                }
                accountEraseDataUseCase -> accountRepository "Anonymises account-owned data" "repository port"
                accountEraseDataUseCase -> accountEventPublisher "Emits an erasure confirmation" "Supabase Queues publisher" {
                    tags "Async"
                }
                accountBalanceNormalizer -> accountHolding "Applies decimals, zero filtering and cross-chain merge" "in-process"
                accountCredentialPolicy -> accountAggregate "Guards read-only credential invariants" "in-process"
            }

            assetRegistryService = container "Asset Registry Service" "Single authority for canonical asset identity and admin configuration of chains and tokens. Owns unique asset symbols, at most one contract or mint per asset per chain with decimals immutable after mapping and native assets resolved from the chain native symbol (a product decision recorded in README section 15), EVM chain records with primary/fallback RPC references, EVM and Solana token registries, review-before-activation lookup and import, catalogue-to-registry sync, a local affected-account index fed by events, and versioned registry change audit published to Audit Service." "Rust, Axum, sqlx, rust_decimal" "Container" {
                group "Transport" {
                    registryAssetHandlers = component "Asset REST Handlers" "Serves canonical asset catalogue queries only; asset and catalogue administration is routed through the admin handlers." "Axum" "Transport"
                    registryAdminHandlers = component "Registry Admin Handlers" "Serves EVM chain, EVM token, Solana token, lookup, import, catalogue-sync and chain-deactivation endpoints with masked secret values." "Axum" "Transport"
                    registryAdminGuard = component "Admin Role Guard Middleware" "Re-checks the Supabase `app_metadata.role` admin claim on every registry admin route and rejects portfolio-ownership-only callers." "Axum middleware" "Transport"
                    registryInternalSnapshot = component "Internal Registry Snapshot Handler" "Serves the registry snapshot to Account and Portfolio services for cold starts and reconciliation." "Axum, mTLS" "Transport"
                    registryInternalAuthGuard = component "Internal Principal Guard" "Validates the signed internal assertion (gateway user assertion or Job Orchestration System-role assertion) and enforces owner scoping and admin roles on internal endpoints. Forwarding rule: user-originated hops forward the gateway assertion unchanged; scheduled and recovery hops carry the System-role assertion from the run command." "Axum middleware, mTLS" "Transport"
                    registryUnmappedConsumer = component "Unmapped Observation Consumer" "Consumes unmapped-token observations from Account Service via Supabase Queues into a quarantine queue for review." "Supabase Queues (pgmq)" "Transport"
                    registryAccountEventConsumer = component "Account Event Consumer" "Consumes account lifecycle events, including the enabled-chain changes carried on account.updated, from Supabase Queues into the affected-account projection." "Supabase Queues (pgmq)" "Transport"
                    registryJobConsumer = component "Job Command Consumer" "Consumes catalogue.refresh commands from Supabase Queues with idempotency keyed on the logical run key (job + scheduled period) plus attempt number, exchanges the command for a short-lived run-scoped assertion over mTLS before executing, and honours a cancel command." "Supabase Queues (pgmq)" "Transport"
                    registryPrivacyConsumer = component "Privacy Request Consumer" "Consumes export and erasure requests for registry-owned data from Supabase Queues." "Supabase Queues (pgmq)" "Transport"
                    registryHealthHandler = component "Health and Readiness Endpoints" "Exposes liveness and readiness probes and reports repository readiness." "Axum" "Transport"
                }
                group "Application" {
                    registryRegisterAssetUseCase = component "Register Asset Use Case" "Creates and updates canonical assets with unique-symbol enforcement, always as an explicit admin action." "Rust" "Application"
                    registryQueryAssetsUseCase = component "Query Assets Use Case" "Serves canonical asset catalogue queries for authenticated users." "Rust" "Application"
                    registryMapContractUseCase = component "Map Asset Contract Use Case" "Maps one contract or mint per asset per chain and enforces decimals immutability." "Rust" "Application"
                    registryManageChainUseCase = component "Manage Chain Use Case" "Creates, edits and deactivates EVM chains and token registry entries, resolving RPC secrets and warning about affected accounts." "Rust" "Application"
                    registryImportLookupUseCase = component "Import Lookup Use Case" "Presents provider lookup candidates for admin review and imports selected results without bulk activation." "Rust" "Application"
                    registrySyncContractsUseCase = component "Sync Contracts from Catalogue Use Case" "Reads the canonical catalogue and proposes mapping updates for admin review, never auto-overwriting existing mappings. The admin-triggered path re-checks the admin claim; the scheduled path is authorised by the run-scoped System assertion bound to the matching run key." "Rust" "Application"
                    registryQuarantineUnmappedUseCase = component "Quarantine Unmapped Observation Use Case" "Stores unmapped observations in a review queue without auto-creating, auto-activating or auto-mapping assets." "Rust" "Application"
                    registryEvaluateChainDeactivationUseCase = component "Evaluate Chain Deactivation Use Case" "Reads the local affected-account projection to warn the Admin before a chain is deactivated." "Rust" "Application"
                    registrySnapshotUseCase = component "Snapshot Registry Use Case" "Reads a consistent registry snapshot for projection rebuilds." "Rust" "Application"
                    registryExportDataUseCase = component "Export Registry Data Use Case" "Assembles the registry portion of a personal-data export, limited to admin/owner attributable records." "Rust" "Application"
                    registryEraseDataUseCase = component "Erase Registry Data Use Case" "Anonymises owner-attributable registry observations subject to legal hold and emits an erasure confirmation." "Rust" "Application"
                }
                group "Domain" {
                    registryAssetAggregate = component "Asset Aggregate" "Canonical asset with unique symbol, name, type, market-cap rank and stablecoin classification." "Rust" "Domain"
                    registryAssetContract = component "Asset Contract" "At most one contract or mint per asset per chain with token decimals; a chain's native asset has no contract or mint and is resolved from the chain native symbol." "Rust" "Domain"
                    registryEvmChain = component "EVM Chain" "Chain record with unique chain ID, primary and fallback RPC secret references, native symbol, explorer URL and active status." "Rust" "Domain"
                    registryIdentityResolver = component "Asset Identity Resolver" "Resolves contract or mint plus chain to a canonical asset without heuristic symbol inference, and resolves a chain's native asset from its native symbol when no contract or mint exists." "Rust" "Domain"
                    registrySecretReferencePolicy = component "Secret Reference Policy" "Stores RPC URLs and provider keys as Supabase Vault secret references, exposes masked values only and never logs them." "Rust" "Domain"
                    registryAdminPolicy = component "Admin Authorization Policy" "Requires the Supabase `app_metadata.role` admin claim projected by the Custom Access Token Hook for interactive registry administration, and accepts a Job Orchestration run-scoped System assertion bound to the matching run key for scheduled catalogue refresh; rejects portfolio-ownership-only callers." "Rust" "Domain"
                    registryEventVersioningPolicy = component "Registry Event Versioning Policy" "Assigns the asset or chain aggregate id, a monotonically increasing aggregate version and occurredAt to registry asset, contract and chain events, and gives each token registry entry its own aggregate id and version, so consumers apply only newer state and deduplicate through their inbox." "Rust" "Domain"
                }
                group "Infrastructure" {
                    registryAssetRepository = component "Supabase Postgres Asset Repository" "Stores assets, contracts, decimals and quarantined unmapped observations with uniqueness constraints in the registry schema." "Rust, sqlx" "Infrastructure"
                    registryChainRepository = component "Supabase Postgres Chain Repository" "Stores EVM chains and EVM and Solana token registries with per-scope duplicate rejection in the registry schema." "Rust, sqlx" "Infrastructure"
                    registryLookupClient = component "Provider Token Lookup Client" "Queries the approved data provider for contract candidates by symbol." "Rust, reqwest" "Infrastructure"
                    registryAffectedAccountProjection = component "Affected-Account Projection" "Event-maintained index of wallet accounts by chain, fed by account lifecycle events and explicitly rebuildable." "Rust, sqlx" "Infrastructure"
                    registrySecretsReader = component "Supabase Vault Secret Reader" "Resolves RPC provider credentials and API keys from Supabase Vault references at runtime." "Supabase Vault (pgsodium)" "Infrastructure"
                    registryEventPublisher = component "Supabase Queues Registry Event Publisher" "Writes registry asset, contract, chain, token and registry-change audit events through a transactional outbox to Supabase Queues." "Supabase Queues (pgmq)" "Infrastructure"
                    registryRunOutcomePublisher = component "Supabase Queues Run Outcome Publisher" "Publishes catalogue.refresh run outcomes and heartbeats." "Supabase Queues (pgmq)" "Infrastructure"
                    registryAssertionVerifier = component "Principal Assertion Verifier" "Resolves the gateway user-assertion and Job Orchestration System-assertion public keys from Supabase Vault, caches them with a bounded refresh, verifies signature, issuer, expiry and correlation id, and rejects any other issuer." "Rust, Ed25519" "Infrastructure"
                }
                registryAssetHandlers -> registryQueryAssetsUseCase "Serves canonical asset catalogue queries" "in-process"
                registryAssetHandlers -> registryAssertionVerifier "Verifies the gateway assertion on public REST routes" "in-process"
                registryAdminHandlers -> registryAdminGuard "Re-checks the admin claim on every admin route" "in-process"
                registryAdminGuard -> registryAssertionVerifier "Verifies the gateway assertion before the admin check" "in-process"
                registryAdminHandlers -> registryRegisterAssetUseCase "Registers or updates assets as an explicit admin action" "in-process"
                registryAdminHandlers -> registryMapContractUseCase "Maps contracts and mints to assets" "in-process"
                registryAdminHandlers -> registryManageChainUseCase "Manages chains and token registries" "in-process"
                registryAdminHandlers -> registryImportLookupUseCase "Reviews and imports lookup candidates" "in-process"
                registryAdminHandlers -> registrySyncContractsUseCase "Runs the admin-only catalogue-to-registry sync" "in-process"
                registryAdminHandlers -> registryEvaluateChainDeactivationUseCase "Warns about affected accounts before deactivation" "in-process"
                registryInternalSnapshot -> registryInternalAuthGuard "Requires a signed service principal" "in-process"
                registryInternalAuthGuard -> registryAssertionVerifier "Verifies the assertion with the Supabase Vault-resolved public key and cache" "in-process"
                registryInternalAuthGuard -> registrySnapshotUseCase "Allows asserted internal snapshot reads" "in-process"
                registryUnmappedConsumer -> registryQuarantineUnmappedUseCase "Queues unmapped observations for admin review" "Supabase Queues consumer" {
                    tags "Async"
                }
                registryAccountEventConsumer -> registryAffectedAccountProjection "Maintains the affected-account index" "Supabase Queues consumer" {
                    tags "Async"
                }
                registryJobConsumer -> registrySyncContractsUseCase "Runs scheduled catalogue refresh idempotently per run key and attempt" "Supabase Queues consumer" {
                    tags "Async"
                }
                registryPrivacyConsumer -> registryExportDataUseCase "Routes export requests" "Supabase Queues consumer" {
                    tags "Async"
                }
                registryPrivacyConsumer -> registryEraseDataUseCase "Routes erasure requests" "Supabase Queues consumer" {
                    tags "Async"
                }
                registryHealthHandler -> registryAssetRepository "Reports repository readiness" "in-process"
                registryRegisterAssetUseCase -> registryAssetAggregate "Creates or updates canonical assets" "in-process"
                registryRegisterAssetUseCase -> registryIdentityResolver "Confirms canonical identity before persistence" "in-process"
                registryRegisterAssetUseCase -> registryAssetRepository "Persists assets and uniqueness checks" "repository port"
                registryRegisterAssetUseCase -> registryEventPublisher "Publishes registry change events and admin audit events" "Supabase Queues publisher" {
                    tags "Async"
                }
                registryRegisterAssetUseCase -> registryEventVersioningPolicy "Stamps the asset aggregate version and occurredAt" "in-process"
                registryRegisterAssetUseCase -> registryAdminPolicy "Re-checks the asserted admin claim" "in-process"
                registryQueryAssetsUseCase -> registryAssetRepository "Reads the canonical asset catalogue" "repository port"
                registryMapContractUseCase -> registryAssetContract "Enforces one contract per asset per chain" "in-process"
                registryMapContractUseCase -> registryAdminPolicy "Re-checks the asserted admin claim" "in-process"
                registryMapContractUseCase -> registryAssetRepository "Persists contract mappings and decimals" "repository port"
                registryMapContractUseCase -> registryEventPublisher "Publishes contract and admin audit events" "Supabase Queues publisher" {
                    tags "Async"
                }
                registryMapContractUseCase -> registryEventVersioningPolicy "Stamps the asset aggregate version and occurredAt" "in-process"
                registryManageChainUseCase -> registryEvmChain "Validates chain identifiers, status and RPC references" "in-process"
                registryManageChainUseCase -> registryChainRepository "Persists chains and tokens" "repository port"
                registryManageChainUseCase -> registrySecretReferencePolicy "Stores RPC URLs as masked secret references" "in-process"
                registryManageChainUseCase -> registrySecretsReader "Resolves provider credentials at runtime" "Supabase Vault client"
                registryManageChainUseCase -> registryEventPublisher "Publishes chain events and admin audit events" "Supabase Queues publisher" {
                    tags "Async"
                }
                registryManageChainUseCase -> registryEventVersioningPolicy "Stamps the chain aggregate version and occurredAt" "in-process"
                registryManageChainUseCase -> registryAdminPolicy "Re-checks the asserted admin claim" "in-process"
                registryImportLookupUseCase -> registryLookupClient "Queries the approved provider for contract candidates" "HTTPS client"
                registryImportLookupUseCase -> registryAssetRepository "Persists the reviewed and selected mappings" "repository port"
                registryImportLookupUseCase -> registryEventPublisher "Publishes imported mappings and the admin audit event" "Supabase Queues publisher" {
                    tags "Async"
                }
                registryImportLookupUseCase -> registryAdminPolicy "Re-checks the asserted admin claim" "in-process"
                registrySyncContractsUseCase -> registryLookupClient "Reads the canonical catalogue" "HTTPS client"
                registrySyncContractsUseCase -> registryAssetRepository "Proposes mapping updates without overwriting" "repository port"
                registrySyncContractsUseCase -> registryRunOutcomePublisher "Publishes catalogue.refresh run outcome" "Supabase Queues publisher" {
                    tags "Async"
                }
                registrySyncContractsUseCase -> registryAdminPolicy "Re-checks the admin claim for admin-triggered runs or the run-scoped System assertion for scheduled runs" "in-process"
                registryQuarantineUnmappedUseCase -> registryAssetRepository "Stores quarantined observations" "repository port"
                registryEvaluateChainDeactivationUseCase -> registryAffectedAccountProjection "Reads affected wallet accounts" "projection port"
                registryEvaluateChainDeactivationUseCase -> registryAdminPolicy "Re-checks the asserted admin claim" "in-process"
                registrySnapshotUseCase -> registryAssetRepository "Reads canonical registry state" "repository port"
                registryExportDataUseCase -> registryAssetRepository "Reads attributable registry data" "repository port"
                registryExportDataUseCase -> registryEventPublisher "Emits the registry export part" "Supabase Queues publisher" {
                    tags "Async"
                }
                registryEraseDataUseCase -> registryAssetRepository "Anonymises attributable observations" "repository port"
                registryEraseDataUseCase -> registryEventPublisher "Emits an erasure confirmation" "Supabase Queues publisher" {
                    tags "Async"
                }
                registryIdentityResolver -> registryAssetContract "Resolves contract to canonical asset" "in-process"
                registrySecretsReader -> supabaseVault "Reads RPC provider and lookup API credentials" "Supabase Vault (pgsodium)"
                registryAssertionVerifier -> supabaseVault "Resolves the assertion verification public keys" "Supabase Vault (pgsodium)"
            }

            marketDataService = container "Market Data Service" "Collects and serves point-in-time USD market observations from a deployment-configured primary provider, falls back to the other approved provider and then to the last known stored price under a shared provider budget. Records source and retrieval time, retains history, exposes freshness and provider health, enforces a per-asset-class maximum stale age after which an asset is unpriced, and publishes run outcomes." "Rust, Axum, sqlx, rust_decimal" "Container" {
                group "Transport" {
                    marketRestHandlers = component "Market REST Handlers" "Serves latest prices, batch prices and freshness endpoints." "Axum" "Transport"
                    marketAdminHandlers = component "Market Admin Handlers" "Configures primary and fallback price providers." "Axum" "Transport"
                    marketAdminGuard = component "Admin Role Guard Middleware" "Re-checks the Supabase `app_metadata.role` admin claim on provider configuration routes." "Axum middleware" "Transport"
                    marketJobConsumer = component "Price Collection Command Consumer" "Consumes scheduled price-collection and retention commands from Supabase Queues with idempotency keyed on the logical run key (job + scheduled period) plus attempt number, exchanges the command for a short-lived run-scoped assertion over mTLS before executing, and honours a cancel command." "Supabase Queues (pgmq)" "Transport"
                    marketInternalPriceHandler = component "Internal Price Handler" "Serves latest prices to Portfolio Service for cold-start projection rebuilds." "Axum, mTLS" "Transport"
                    marketInternalAuthGuard = component "Internal Principal Guard" "Validates the signed internal assertion (gateway user assertion or Job Orchestration System-role assertion) and enforces owner scoping on internal endpoints. Forwarding rule: user-originated hops forward the gateway assertion unchanged; scheduled and recovery hops carry the System-role assertion from the run command." "Axum middleware, mTLS" "Transport"
                    marketRegistryConsumer = component "Registry Event Consumer" "Consumes registry asset events from Supabase Queues into the tracked asset catalogue." "Supabase Queues (pgmq)" "Transport"
                    marketPrivacyConsumer = component "Privacy Request Consumer" "Consumes export and erasure requests for market-owned data from Supabase Queues." "Supabase Queues (pgmq)" "Transport"
                    marketHealthHandler = component "Health and Readiness Endpoints" "Exposes liveness and readiness probes and reports repository and provider readiness." "Axum" "Transport"
                }
                group "Application" {
                    marketCollectPricesUseCase = component "Collect Prices Use Case" "Collects observations per asset with provider fallback, source attribution, provider budgets and fault tolerance." "Rust" "Application"
                    marketGetLatestPricesUseCase = component "Get Latest Prices Use Case" "Returns the latest stored price observations and their timestamps." "Rust" "Application"
                    marketEvaluateFreshnessUseCase = component "Evaluate Freshness Use Case" "Evaluates staleness thresholds, enforces maximum stale age and marks assets stale or unpriced." "Rust" "Application"
                    marketUpdateProvidersUseCase = component "Update Providers Use Case" "Updates primary and fallback provider configuration without deployment." "Rust" "Application"
                    marketUpdateAssetCatalogUseCase = component "Apply Registry Event Use Case" "Applies registry asset events to the tracked asset catalogue only when the aggregate version is newer than the stored one." "Rust" "Application"
                    marketExportDataUseCase = component "Export Market Data Use Case" "Assembles the market-data portion of a personal-data export." "Rust" "Application"
                    marketEraseDataUseCase = component "Erase Market Data Use Case" "Anonymises owner-attributable market data subject to legal hold and emits an erasure confirmation." "Rust" "Application"
                    marketApplyRetentionUseCase = component "Apply Price Retention Use Case" "Archives or drops expired price partitions under a privileged retention role, authorised by the admin claim or a run-scoped System assertion bound to the run key." "Rust" "Application"
                }
                group "Domain" {
                    marketPriceObservation = component "Price Observation" "Spot USD price, 24h volume, market capitalisation, rank, 24h change, source, retrieval time and a per-asset monotonic observation sequence." "Rust" "Domain"
                    marketPriceFreshness = component "Price Freshness" "Fresh, stale or unpriced classification derived from observation age, configured thresholds and a maximum stale age per asset class." "Rust" "Domain"
                    marketFallbackPolicy = component "Price Fallback Policy" "Primary provider, then approved fallback provider, then last known stored price, with a hard maximum stale age." "Rust" "Domain"
                    marketProviderStatus = component "Provider Status" "Provider health, failure state, circuit-breaker state and shared request budget." "Rust" "Domain"
                    marketAdminPolicy = component "Admin Authorization Policy" "Requires the Supabase `app_metadata.role` admin claim projected by the Custom Access Token Hook for provider configuration changes, and accepts a run-scoped System assertion bound to the run key for scheduled retention." "Rust" "Domain"
                    marketProviderSecretPolicy = component "Provider Secret Reference Policy" "Requires provider credentials to be Supabase Vault references, masks them in admin responses and never logs or publishes them." "Rust" "Domain"
                    marketPriceEventVersioningPolicy = component "Price Event Versioning Policy" "Assigns each asset a monotonically increasing observation sequence, the provider source and occurredAt to price and freshness events, so consumers compare price ordering separately from registry aggregate versions." "Rust" "Domain"
                }
                group "Infrastructure" {
                    marketPriceRepository = component "Supabase Postgres Price Repository" "Stores partitioned price observations, retained history, provider mapping, outbox and inbox dedup records in the market schema, with a separate privileged retention role for partition archival and drop." "Rust, sqlx" "Infrastructure"
                    marketCoinGeckoClient = component "CoinGecko Price Client" "Collects USD spot, volume, market capitalisation, rank and 24h change." "Rust, reqwest" "Infrastructure"
                    marketCoinPaprikaClient = component "CoinPaprika Price Client" "Collects fallback prices when the configured primary fails or is throttled." "Rust, reqwest" "Infrastructure"
                    marketLastKnownPriceCache = component "Supabase Postgres Last-Known-Price Cache" "Caches the last known price per asset and provider circuit-breaker state in Supabase Postgres." "Supabase Postgres" "Infrastructure"
                    marketSecretsReader = component "Supabase Vault Secret Reader" "Resolves provider API keys from Supabase Vault references without exposing them to logs or events." "Supabase Vault (pgsodium)" "Infrastructure"
                    marketEventPublisher = component "Supabase Queues Market Event Publisher" "Writes price, freshness and provider-health events through a transactional outbox to Supabase Queues." "Supabase Queues (pgmq)" "Infrastructure"
                    marketRunOutcomePublisher = component "Supabase Queues Run Outcome Publisher" "Publishes price-collection run outcomes and heartbeats." "Supabase Queues (pgmq)" "Infrastructure"
                    marketAssertionVerifier = component "Principal Assertion Verifier" "Resolves the gateway user-assertion and Job Orchestration System-assertion public keys from Supabase Vault, caches them with a bounded refresh, verifies signature, issuer, expiry and correlation id, and rejects any other issuer." "Rust, Ed25519" "Infrastructure"
                }
                marketRestHandlers -> marketGetLatestPricesUseCase "Serves latest prices" "in-process"
                marketRestHandlers -> marketAssertionVerifier "Verifies the gateway assertion on public REST routes" "in-process"
                marketRestHandlers -> marketEvaluateFreshnessUseCase "Serves the freshness endpoint" "in-process"
                marketAdminHandlers -> marketAdminGuard "Re-checks the admin claim on provider configuration" "in-process"
                marketAdminGuard -> marketAssertionVerifier "Verifies the gateway assertion before the admin check" "in-process"
                marketAdminHandlers -> marketUpdateProvidersUseCase "Configures primary and fallback providers" "in-process"
                marketJobConsumer -> marketCollectPricesUseCase "Runs scheduled price collection idempotently per run key and attempt" "Supabase Queues consumer" {
                    tags "Async"
                }
                marketJobConsumer -> marketApplyRetentionUseCase "Runs scheduled price retention idempotently per run key and attempt" "Supabase Queues consumer" {
                    tags "Async"
                }
                marketInternalPriceHandler -> marketInternalAuthGuard "Requires a signed service principal" "in-process"
                marketInternalAuthGuard -> marketAssertionVerifier "Verifies the assertion with the Supabase Vault-resolved public key and cache" "in-process"
                marketInternalAuthGuard -> marketGetLatestPricesUseCase "Allows asserted internal reads" "in-process"
                marketRegistryConsumer -> marketUpdateAssetCatalogUseCase "Applies registry asset changes" "Supabase Queues consumer" {
                    tags "Async"
                }
                marketPrivacyConsumer -> marketExportDataUseCase "Routes export requests" "Supabase Queues consumer" {
                    tags "Async"
                }
                marketPrivacyConsumer -> marketEraseDataUseCase "Routes erasure requests" "Supabase Queues consumer" {
                    tags "Async"
                }
                marketHealthHandler -> marketPriceRepository "Reports repository readiness" "in-process"
                marketCollectPricesUseCase -> marketCoinGeckoClient "Collects CoinGecko observations" "HTTPS client"
                marketCollectPricesUseCase -> marketCoinPaprikaClient "Collects CoinPaprika fallback observations" "HTTPS client"
                marketCollectPricesUseCase -> marketFallbackPolicy "Applies provider fallback and last-known-price rules" "in-process"
                marketCollectPricesUseCase -> marketPriceObservation "Records timestamped observations" "in-process"
                marketCollectPricesUseCase -> marketPriceRepository "Persists observations, history and outbox records" "repository port"
                marketCollectPricesUseCase -> marketLastKnownPriceCache "Updates the last-known-price cache" "Supabase Postgres client"
                marketCollectPricesUseCase -> marketSecretsReader "Resolves provider API keys" "Supabase Vault client"
                marketCollectPricesUseCase -> marketEventPublisher "Publishes price observations and batches" "Supabase Queues publisher" {
                    tags "Async"
                }
                marketCollectPricesUseCase -> marketPriceEventVersioningPolicy "Stamps the per-asset observation sequence and occurredAt" "in-process"
                marketCollectPricesUseCase -> marketRunOutcomePublisher "Publishes the run outcome and heartbeat" "Supabase Queues publisher" {
                    tags "Async"
                }
                marketGetLatestPricesUseCase -> marketPriceRepository "Reads latest stored prices" "repository port"
                marketEvaluateFreshnessUseCase -> marketPriceFreshness "Evaluates staleness thresholds and maximum stale age" "in-process"
                marketEvaluateFreshnessUseCase -> marketProviderStatus "Updates provider health and circuit state" "in-process"
                marketEvaluateFreshnessUseCase -> marketEventPublisher "Publishes freshness and provider-health events" "Supabase Queues publisher" {
                    tags "Async"
                }
                marketEvaluateFreshnessUseCase -> marketPriceRepository "Reads observations to classify staleness" "repository port"
                marketUpdateProvidersUseCase -> marketProviderStatus "Resets provider health on configuration change" "in-process"
                marketUpdateProvidersUseCase -> marketAdminPolicy "Re-checks the asserted admin claim" "in-process"
                marketUpdateProvidersUseCase -> marketProviderSecretPolicy "Requires Supabase Vault secret references and masks provider credentials" "in-process"
                marketUpdateProvidersUseCase -> marketSecretsReader "Validates provider credential references without exposing them" "Supabase Vault client"
                marketUpdateProvidersUseCase -> marketEventPublisher "Publishes the provider configuration audit event" "Supabase Queues publisher" {
                    tags "Async"
                }
                marketUpdateProvidersUseCase -> marketPriceRepository "Persists provider mapping" "repository port"
                marketUpdateAssetCatalogUseCase -> marketPriceRepository "Persists the tracked asset catalogue" "repository port"
                marketExportDataUseCase -> marketPriceRepository "Reads attributable market data" "repository port"
                marketExportDataUseCase -> marketEventPublisher "Emits the market export part" "Supabase Queues publisher" {
                    tags "Async"
                }
                marketEraseDataUseCase -> marketPriceRepository "Anonymises attributable market data" "repository port"
                marketEraseDataUseCase -> marketEventPublisher "Emits an erasure confirmation" "Supabase Queues publisher" {
                    tags "Async"
                }
                marketApplyRetentionUseCase -> marketPriceRepository "Archives or drops expired partitions" "privileged repository"
                marketApplyRetentionUseCase -> marketAdminPolicy "Re-checks the admin claim or the run-scoped System assertion" "in-process"
                marketApplyRetentionUseCase -> marketRunOutcomePublisher "Publishes the retention run outcome" "Supabase Queues publisher" {
                    tags "Async"
                }
                marketFallbackPolicy -> marketProviderStatus "Reads circuit-breaker state and request budget" "in-process"
                marketSecretsReader -> supabaseVault "Reads provider API keys" "Supabase Vault (pgsodium)"
                marketAssertionVerifier -> supabaseVault "Resolves the assertion verification public keys" "Supabase Vault (pgsodium)"
            }

            portfolioService = container "Portfolio Service" "Owns the portfolio-centred read and write loop in three Clean Architecture modules: portfolio lifecycle and configuration with unique-name, single-default and 100%-target invariants, valuation and immutable snapshots, and advisory recommendations with recorded reproducible inputs. Batch work runs from job commands in an isolated worker pool with its own connection pool and bounded per-owner concurrency. Valuation, drift, snapshot capture and recommendation generation commit in one service and one transaction against its own projections; all reads use Supabase Postgres, which guarantees read-your-writes, while optional Supabase read replicas may serve read-heavy dashboard, oversight and snapshot-history paths within a bounded lag budget without being modelled as separate elements. Scheduled reconciliation compares projection versions with Account's internal API." "Rust, Axum, sqlx, rust_decimal, tokio" "Container" {
                group "Transport" {
                    portfolioRestHandlers = component "Portfolio REST Handlers" "Serves portfolio lifecycle, account linking, targets, guardrails and oversight endpoints." "Axum" "Transport"
                    portfolioValuationHandlers = component "Valuation and Snapshot Handlers" "Serves valuation, drift, snapshot capture, history and trend endpoints." "Axum" "Transport"
                    portfolioRecommendationHandlers = component "Recommendation Handlers" "Serves recommendation generation, listing, history and accept or reject decisions." "Axum" "Transport"
                    portfolioInternalHandlers = component "Internal Brief Data Handler" "Serves the consistent single-owner brief data set to Reporting Service once per brief." "Axum, mTLS" "Transport"
                    portfolioInternalAuthGuard = component "Internal Principal Guard" "Validates the signed internal assertion (gateway user assertion or Job Orchestration run-scoped System-role assertion) and enforces owner scoping and the admin claim on internal endpoints, including the operations rebuild route. Forwarding rule: user-originated hops forward the gateway assertion unchanged; scheduled and recovery hops carry the run-scoped assertion obtained from the run-assertion exchange." "Axum middleware, mTLS" "Transport"
                    portfolioRebuildHandler = component "Operations Rebuild Handler" "Serves the admin-triggered, assertion-authorised projection rebuild endpoint over mTLS." "Axum, mTLS" "Transport"
                    portfolioJobConsumer = component "Job Command Consumer" "Consumes snapshots.eod, recommendations.expire, snapshots.retention and projections.reconcile commands from Supabase Queues with idempotency keyed on the logical run key (job + scheduled period) plus attempt, exchanges the command for a short-lived run-scoped assertion over mTLS before executing, honours a cancel command between shards, and hands them to the owning use case." "Supabase Queues (pgmq)" "Transport"
                    portfolioAccountEventConsumer = component "Account Event Consumer" "Consumes versioned holdings and account lifecycle events from Supabase Queues into the projections and link activation state." "Supabase Queues (pgmq)" "Transport"
                    portfolioMarketEventConsumer = component "Price and Registry Event Consumer" "Consumes price, freshness, registry asset and registry chain events from Supabase Queues into the price and registry projection with inbox dedup." "Supabase Queues (pgmq)" "Transport"
                    portfolioPrivacyConsumer = component "Privacy Request Consumer" "Consumes export and erasure requests for portfolio-owned data from Supabase Queues." "Supabase Queues (pgmq)" "Transport"
                    portfolioHealthHandler = component "Health and Readiness Endpoints" "Exposes liveness and readiness probes and reports repository readiness." "Axum" "Transport"
                }
                group "Application" {
                    portfolioManagePortfolioUseCase = component "Manage Portfolio Use Case" "Manages lifecycle, default portfolio, account links, targets and guardrails, validating account ownership and active status at link time." "Rust" "Application"
                    portfolioDeletePortfolioUseCase = component "Delete Portfolio Use Case" "Loads the portfolio, re-checks the asserted owner, deletes it and cascades to targets, account links, recommendations and unretained snapshots, honouring retention and legal hold, then publishes portfolio.deleted with the deletion audit event." "Rust" "Application"
                    portfolioRecomputeValuationUseCase = component "Recompute Valuation Use Case" "Computes valuation, allocation, weights and drift from its own projections in Supabase Postgres, excluding inactive linked accounts and unpriced or unmapped lines from the valued total." "Rust" "Application"
                    portfolioCaptureSnapshotUseCase = component "Capture Snapshot Use Case" "Writes a first-write-wins immutable manual or EOD snapshot with reproducible inputs. A capture is idempotent on the logical run key plus portfolio, date and type: any attempt of the same logical run whose snapshot is already present succeeds without writing again. Rejection is reserved for a genuinely different logical run key or a manual writer (a documented deviation from BRD-007 FR-VAL-09 and BRD-011 BR-JOB-03, see README section 15)." "Rust" "Application"
                    portfolioGenerateRecommendationUseCase = component "Generate Recommendation Use Case" "Detects drift beyond threshold, applies guardrails, supersedes prior pending recommendations and produces a deterministic advisory recommendation." "Rust" "Application"
                    portfolioRecordRecommendationDecisionUseCase = component "Record Recommendation Decision Use Case" "Validates that the recommendation is still pending, records the owner's accept or reject decision with actor and timestamp, and publishes recommendation.decided and the audit event." "Rust" "Application"
                    portfolioExpireRecommendationsUseCase = component "Expire Recommendations Use Case" "Expires pending recommendations whose horizon has passed and records the supersession link." "Rust" "Application"
                    portfolioScheduledFanOutUseCase = component "Scheduled Portfolio Fan-Out Use Case" "Enumerates and shards every active portfolio for snapshots.eod and recommendations.expire, applying per-owner fairness, backpressure and idempotency on (run key, attempt). Per-portfolio failure is isolated: a failed portfolio is recorded as failed and the remaining shards continue, producing a partial-failure run outcome. The fan-out is the sole publisher of the run outcome and heartbeat for the run, stops between shards when the run is cancelled, and leaves retention on its own command path." "Rust" "Application"
                    portfolioApplyAccountEventUseCase = component "Apply Account Event Use Case" "Applies only newer versioned holdings and account lifecycle events, marks links to deactivated accounts inactive and excluded from valuation, and removes links only on account.deleted or reactivates them on account.activated." "Rust" "Application"
                    portfolioApplyPriceEventUseCase = component "Apply Price and Registry Event Use Case" "Applies price, freshness, registry asset and registry chain events to the price and registry projection. Price freshness is compared by the per-asset observation sequence and registry-derived state by the asset aggregate version, so the two orderings are never mixed. A deactivated registry chain marks its holdings stale and excludes them from the valued total." "Rust" "Application"
                    portfolioRebuildProjectionsUseCase = component "Rebuild Projections Use Case" "Rebuilds the holdings and price/registry projections from Account, Registry and Market snapshot endpoints with timeout, bounded retry, circuit breaker and a degraded read-only mode." "Rust" "Application"
                    portfolioReconcileProjectionUseCase = component "Reconcile Projection Use Case" "Compares stored projection aggregate versions with Account's internal holdings API and reports divergence in the run outcome, which Job Orchestration alerts on." "Rust" "Application"
                    portfolioReconcileFanOutUseCase = component "Reconciliation Fan-Out Use Case" "Enumerates owners, reconciles each with per-owner isolation, backpressure and a partial-failure outcome, and stops between owners when the run is cancelled." "Rust" "Application"
                    portfolioValidateAccountOwnershipUseCase = component "Validate Account Ownership Use Case" "Confirms account ownership and active status through Account's internal API with timeout and failure handling." "Rust" "Application"
                    portfolioQuerySnapshotsUseCase = component "Query Snapshot History Use Case" "Returns snapshot history and trend for an owned portfolio from Supabase Postgres." "Rust" "Application"
                    portfolioQueryRecommendationsUseCase = component "Query Recommendation History Use Case" "Returns recommendation history and recorded decisions for an owned portfolio." "Rust" "Application"
                    portfolioExportDataUseCase = component "Export Portfolio Data Use Case" "Assembles the portfolio, snapshot and recommendation portion of a personal-data export." "Rust" "Application"
                    portfolioEraseDataUseCase = component "Erase Portfolio Data Use Case" "Anonymises portfolio-owned data subject to legal hold and emits an erasure confirmation." "Rust" "Application"
                    portfolioApplyRetentionUseCase = component "Apply Snapshot Retention Use Case" "Drops expired snapshot partitions under a privileged retention role." "Rust" "Application"
                }
                group "Domain" {
                    portfolioPortfolioAggregate = component "Portfolio Aggregate" "Portfolio with owner, default designation, account links with active or excluded state, target allocations and guardrails. Invariants: portfolio name unique per owner, exactly one default portfolio per owner, and target weights summing to 100% within decimal tolerance." "Rust" "Domain"
                    portfolioValuationCalculator = component "Valuation Calculator" "Merges canonical assets, excludes inactive linked accounts, values holdings in USD, classifies each line as priced, stale, unpriced or unmapped, excludes unpriced and unmapped lines from the valued total and computes weights and drift with decimal arithmetic." "Rust, rust_decimal" "Domain"
                    portfolioSnapshot = component "Snapshot" "Immutable manual or EOD snapshot with exact line values, per-line status (priced, stale, unpriced or unmapped) and input references for reproducibility." "Rust" "Domain"
                    portfolioRecommendation = component "Recommendation" "Drift-based buy or sell proposal with guardrail evaluation, risk level, rationale, disclaimer, decision state and supersession link, plus the recorded inputs needed to reproduce it: holdings projection version, price observation ids, target and guardrail versions and calculation timestamp." "Rust" "Domain"
                    portfolioSnapshotImmutabilityPolicy = component "Snapshot Immutability Policy" "One snapshot per portfolio, date and type, with first write wins. Any attempt under the same logical run key whose snapshot was already written by that run is treated as already captured; rejection is reserved for a genuinely different logical run key or a manual writer. This deviates from BRD-007 FR-VAL-09 and BRD-011 BR-JOB-03/AC-JOB-04 and is recorded in README section 15." "Rust" "Domain"
                    portfolioValuationLineStatus = component "Valuation Line Status" "Classifies each valuation and snapshot line as priced, stale, unpriced or unmapped and drives exclusion of unpriced and unmapped assets from the valued total. Holdings on a deactivated chain are classified stale and excluded." "Rust" "Domain"
                    portfolioConfigurationPolicy = component "Portfolio Configuration Policy" "Enforces a unique portfolio name per owner, exactly one default portfolio per owner and target weights summing to 100% within decimal tolerance before any configuration is persisted." "Rust" "Domain"
                    portfolioRecommendationSupersessionPolicy = component "Recommendation Supersession Policy" "A new generation supersedes prior pending recommendations while preserving history links." "Rust" "Domain"
                    portfolioAdminPolicy = component "Admin Authorization Policy" "Requires the Supabase `app_metadata.role` admin claim projected by the Custom Access Token Hook for the operations rebuild endpoint and rejects portfolio-ownership-only callers." "Rust" "Domain"
                }
                group "Infrastructure" {
                    portfolioRepository = component "Supabase Postgres Portfolio Repository" "Stores portfolios, links, targets, guardrails, snapshots and recommendations in the portfolio schema with monthly partitions, a unique (owner, name) constraint and a partial unique index on the single default portfolio per owner, outbox and inbox dedup records, and a separate privileged retention role for partition drop." "Rust, sqlx" "Infrastructure"
                    portfolioHoldingsProjection = component "Holdings Projection" "Event-maintained holdings by account and asset with a stored aggregate version, explicitly rebuildable from events." "Rust, sqlx" "Infrastructure"
                    portfolioPriceProjection = component "Price and Registry Projection" "Event-maintained latest prices and canonical asset metadata, rebuildable from events." "Rust, sqlx" "Infrastructure"
                    portfolioAdvisoryWorker = component "Advisory and Batch Worker Pool" "Dedicated batch worker pool with its own connection pool, bounded concurrency, per-owner fairness and backpressure for recommendation compute and sharded EOD snapshots." "Rust, tokio" "Infrastructure"
                    portfolioAccountOwnershipClient = component "Account Ownership Client" "Calls Account Service over mTLS to validate account ownership and active status with bounded timeout and failure behavior." "Rust, reqwest, mTLS" "Infrastructure"
                    portfolioAccountSnapshotClient = component "Account Snapshot Client" "Fetches owner holdings from Account Service over mTLS for projection rebuild and reconciliation with timeout and circuit breaker." "Rust, reqwest, mTLS" "Infrastructure"
                    portfolioRegistrySnapshotClient = component "Registry Snapshot Client" "Fetches the registry snapshot over mTLS for projection rebuild with timeout and circuit breaker." "Rust, reqwest, mTLS" "Infrastructure"
                    portfolioMarketPriceClient = component "Market Price Client" "Fetches latest prices from Market Data Service over mTLS for projection rebuild with timeout and circuit breaker." "Rust, reqwest, mTLS" "Infrastructure"
                    portfolioSecretsClient = component "Supabase Vault Secret Client" "Resolves database and projection-rebuild credentials from Supabase Vault references." "Supabase Vault (pgsodium)" "Infrastructure"
                    portfolioEventPublisher = component "Supabase Queues Portfolio Event Publisher" "Writes portfolio, valuation, snapshot, recommendation and audit events through a transactional outbox to Supabase Queues." "Supabase Queues (pgmq)" "Infrastructure"
                    portfolioRunOutcomePublisher = component "Supabase Queues Run Outcome Publisher" "Publishes EOD snapshot, recommendation-expiry, reconciliation and retention run outcomes and heartbeats; per-portfolio progress travels on a distinct item-progress queue." "Supabase Queues (pgmq)" "Infrastructure"
                    portfolioAssertionVerifier = component "Principal Assertion Verifier" "Resolves the gateway user-assertion and Job Orchestration System-assertion public keys from Supabase Vault, caches them with a bounded refresh, verifies signature, issuer, expiry and correlation id, and rejects any other issuer." "Rust, Ed25519" "Infrastructure"
                }
                portfolioRestHandlers -> portfolioManagePortfolioUseCase "Manages lifecycle, links, targets and guardrails" "in-process"
                portfolioRestHandlers -> portfolioAssertionVerifier "Verifies the gateway assertion on public REST routes" "in-process"
                portfolioValuationHandlers -> portfolioAssertionVerifier "Verifies the gateway assertion on valuation and snapshot routes" "in-process"
                portfolioRecommendationHandlers -> portfolioAssertionVerifier "Verifies the gateway assertion on recommendation routes" "in-process"
                portfolioRestHandlers -> portfolioDeletePortfolioUseCase "Deletes a portfolio with its cascade and audit event" "in-process"
                portfolioValuationHandlers -> portfolioRecomputeValuationUseCase "Serves valuation and drift reads" "in-process"
                portfolioValuationHandlers -> portfolioCaptureSnapshotUseCase "Captures manual snapshots" "in-process"
                portfolioValuationHandlers -> portfolioQuerySnapshotsUseCase "Serves snapshot history and trend" "in-process"
                portfolioRecommendationHandlers -> portfolioGenerateRecommendationUseCase "Generates recommendations on demand" "in-process"
                portfolioRecommendationHandlers -> portfolioRecordRecommendationDecisionUseCase "Records accept or reject decisions" "in-process"
                portfolioRecommendationHandlers -> portfolioQueryRecommendationsUseCase "Serves recommendation history and decisions" "in-process"
                portfolioInternalHandlers -> portfolioInternalAuthGuard "Requires a signed service principal" "in-process"
                portfolioInternalAuthGuard -> portfolioAssertionVerifier "Verifies the assertion with the Supabase Vault-resolved public key and cache" "in-process"
                portfolioInternalAuthGuard -> portfolioRecomputeValuationUseCase "Allows asserted brief-data reads" "in-process"
                portfolioInternalAuthGuard -> portfolioRebuildProjectionsUseCase "Allows asserted admin rebuild requests" "in-process"
                portfolioRebuildProjectionsUseCase -> portfolioAdminPolicy "Re-checks the asserted admin claim before rebuilding" "in-process"
                portfolioRebuildHandler -> portfolioInternalAuthGuard "Requires a signed admin principal for the rebuild endpoint" "in-process"
                portfolioJobConsumer -> portfolioScheduledFanOutUseCase "Runs sharded EOD snapshot capture and recommendation expiry idempotently per run key and attempt" "Supabase Queues consumer" {
                    tags "Async"
                }
                portfolioJobConsumer -> portfolioApplyRetentionUseCase "Runs scheduled snapshot retention idempotently per run key and attempt" "Supabase Queues consumer" {
                    tags "Async"
                }
                portfolioJobConsumer -> portfolioReconcileFanOutUseCase "Runs scheduled projection reconciliation idempotently per run key and attempt" "Supabase Queues consumer" {
                    tags "Async"
                }
                portfolioAccountEventConsumer -> portfolioApplyAccountEventUseCase "Applies versioned holdings and account lifecycle events" "Supabase Queues consumer" {
                    tags "Async"
                }
                portfolioMarketEventConsumer -> portfolioApplyPriceEventUseCase "Applies price, freshness and registry events" "Supabase Queues consumer" {
                    tags "Async"
                }
                portfolioPrivacyConsumer -> portfolioExportDataUseCase "Routes export requests" "Supabase Queues consumer" {
                    tags "Async"
                }
                portfolioPrivacyConsumer -> portfolioEraseDataUseCase "Routes erasure requests" "Supabase Queues consumer" {
                    tags "Async"
                }
                portfolioHealthHandler -> portfolioRepository "Reports repository readiness" "in-process"
                portfolioManagePortfolioUseCase -> portfolioPortfolioAggregate "Enforces ownership, default and target rules" "in-process"
                portfolioManagePortfolioUseCase -> portfolioConfigurationPolicy "Validates name uniqueness, a single default and target weights summing to 100%" "in-process"
                portfolioManagePortfolioUseCase -> portfolioRepository "Persists portfolios, links, targets and guardrails" "repository port"
                portfolioManagePortfolioUseCase -> portfolioValidateAccountOwnershipUseCase "Validates ownership when linking an account" "in-process"
                portfolioManagePortfolioUseCase -> portfolioEventPublisher "Publishes portfolio.created and portfolio.updated" "Supabase Queues publisher" {
                    tags "Async"
                }
                portfolioDeletePortfolioUseCase -> portfolioPortfolioAggregate "Loads the portfolio and re-checks the asserted owner before applying the deletion cascade" "in-process"
                portfolioDeletePortfolioUseCase -> portfolioRepository "Deletes the portfolio and cascades targets, links and recommendations" "repository port"
                portfolioDeletePortfolioUseCase -> portfolioEventPublisher "Publishes portfolio.deleted and the deletion audit event" "Supabase Queues publisher" {
                    tags "Async"
                }
                portfolioValidateAccountOwnershipUseCase -> portfolioAccountOwnershipClient "Confirms ownership and active status with Account Service" "mTLS REST client"
                portfolioRecomputeValuationUseCase -> portfolioValuationCalculator "Computes allocations, weights and drift" "in-process"
                portfolioRecomputeValuationUseCase -> portfolioPortfolioAggregate "Reads active account links and excludes inactive ones" "in-process"
                portfolioRecomputeValuationUseCase -> portfolioValuationLineStatus "Classifies lines and excludes unpriced or unmapped assets from the total" "in-process"
                portfolioRecomputeValuationUseCase -> portfolioHoldingsProjection "Reads the holdings projection" "projection port"
                portfolioRecomputeValuationUseCase -> portfolioPriceProjection "Reads the latest price and registry projection" "projection port"
                portfolioRecomputeValuationUseCase -> portfolioEventPublisher "Publishes valuation.computed" "Supabase Queues publisher" {
                    tags "Async"
                }
                portfolioCaptureSnapshotUseCase -> portfolioPortfolioAggregate "Loads and owner-scopes the portfolio before capture" "in-process"
                portfolioCaptureSnapshotUseCase -> portfolioSnapshot "Creates an immutable snapshot in one transaction" "in-process"
                portfolioCaptureSnapshotUseCase -> portfolioSnapshotImmutabilityPolicy "Treats a retry of the same run key and attempt as already captured and rejects only a different writer" "in-process"
                portfolioCaptureSnapshotUseCase -> portfolioRepository "Persists snapshot line items" "repository port"
                portfolioCaptureSnapshotUseCase -> portfolioAdvisoryWorker "Runs EOD capture in the isolated batch worker" "worker pool"
                portfolioCaptureSnapshotUseCase -> portfolioRunOutcomePublisher "Publishes per-portfolio capture progress on the item-progress queue, never the run outcome" "Supabase Queues publisher" {
                    tags "Async"
                }
                portfolioCaptureSnapshotUseCase -> portfolioEventPublisher "Publishes snapshot.captured with aggregate version and occurredAt" "Supabase Queues publisher" {
                    tags "Async"
                }
                portfolioGenerateRecommendationUseCase -> portfolioPortfolioAggregate "Loads and owner-scopes the portfolio before generating" "in-process"
                portfolioGenerateRecommendationUseCase -> portfolioRecommendation "Applies drift thresholds and guardrails" "in-process"
                portfolioGenerateRecommendationUseCase -> portfolioRecommendationSupersessionPolicy "Supersedes prior pending recommendations" "in-process"
                portfolioGenerateRecommendationUseCase -> portfolioAdvisoryWorker "Runs advisory compute in an isolated pool" "worker pool"
                portfolioGenerateRecommendationUseCase -> portfolioRepository "Persists generated recommendations" "repository port"
                portfolioGenerateRecommendationUseCase -> portfolioEventPublisher "Publishes recommendation.generated" "Supabase Queues publisher" {
                    tags "Async"
                }
                portfolioRecordRecommendationDecisionUseCase -> portfolioPortfolioAggregate "Loads and owner-scopes the portfolio before recording a decision" "in-process"
                portfolioRecordRecommendationDecisionUseCase -> portfolioRecommendation "Validates pending state and records the decision" "in-process"
                portfolioRecordRecommendationDecisionUseCase -> portfolioRepository "Persists the decision, actor and timestamp" "repository port"
                portfolioRecordRecommendationDecisionUseCase -> portfolioEventPublisher "Publishes recommendation.decided and the decision audit event" "Supabase Queues publisher" {
                    tags "Async"
                }
                portfolioExpireRecommendationsUseCase -> portfolioRecommendation "Expires pending recommendations" "in-process"
                portfolioExpireRecommendationsUseCase -> portfolioRepository "Persists expiry and supersession" "repository port"
                portfolioExpireRecommendationsUseCase -> portfolioRunOutcomePublisher "Publishes per-portfolio expiry progress on the item-progress queue, never the run outcome" "Supabase Queues publisher" {
                    tags "Async"
                }
                portfolioScheduledFanOutUseCase -> portfolioRepository "Enumerates active portfolios with per-owner fairness and sharding" "repository port"
                portfolioScheduledFanOutUseCase -> portfolioAdvisoryWorker "Shards portfolios across the isolated worker pool with backpressure" "worker pool"
                portfolioScheduledFanOutUseCase -> portfolioCaptureSnapshotUseCase "Captures each due portfolio snapshot, recording a failed portfolio and continuing with the remaining shards" "in-process"
                portfolioScheduledFanOutUseCase -> portfolioExpireRecommendationsUseCase "Expires pending recommendations per portfolio, isolating per-portfolio failures" "in-process"
                portfolioScheduledFanOutUseCase -> portfolioRunOutcomePublisher "Publishes the single aggregated run outcome, including a partial-failure outcome, and heartbeat" "Supabase Queues publisher" {
                    tags "Async"
                }
                portfolioApplyAccountEventUseCase -> portfolioHoldingsProjection "Applies only newer versions" "projection port"
                portfolioApplyAccountEventUseCase -> portfolioRepository "Marks deactivated links inactive and removes them only on account.deleted" "repository port"
                portfolioApplyPriceEventUseCase -> portfolioPriceProjection "Applies only newer price and registry state" "projection port"
                portfolioRebuildProjectionsUseCase -> portfolioHoldingsProjection "Rebuilds holdings from the Account snapshot" "projection port"
                portfolioRebuildProjectionsUseCase -> portfolioPriceProjection "Rebuilds prices and registry metadata from snapshots" "projection port"
                portfolioRebuildProjectionsUseCase -> portfolioAccountSnapshotClient "Fetches the holdings snapshot" "mTLS REST client"
                portfolioRebuildProjectionsUseCase -> portfolioRegistrySnapshotClient "Fetches the registry snapshot" "mTLS REST client"
                portfolioRebuildProjectionsUseCase -> portfolioMarketPriceClient "Fetches latest prices" "mTLS REST client"
                portfolioReconcileProjectionUseCase -> portfolioHoldingsProjection "Reads stored projection versions" "projection port"
                portfolioReconcileFanOutUseCase -> portfolioReconcileProjectionUseCase "Reconciles each owner with isolation, backpressure and a partial-failure outcome" "in-process"
                portfolioReconcileProjectionUseCase -> portfolioAccountSnapshotClient "Compares with Account's internal holdings" "mTLS REST client"
                portfolioReconcileProjectionUseCase -> portfolioRunOutcomePublisher "Reports divergence in the run outcome for alerting" "Supabase Queues publisher" {
                    tags "Async"
                }
                portfolioQuerySnapshotsUseCase -> portfolioRepository "Reads snapshot history and trend from Supabase Postgres" "repository port"
                portfolioQueryRecommendationsUseCase -> portfolioRepository "Reads recommendation history and decisions" "repository port"
                portfolioRepository -> portfolioSecretsClient "Resolves database credentials from Supabase Vault" "Supabase Vault client"
                portfolioSecretsClient -> supabaseVault "Reads database and rebuild secret references" "Supabase Vault (pgsodium)"
                portfolioExportDataUseCase -> portfolioRepository "Reads attributable portfolio data" "repository port"
                portfolioExportDataUseCase -> portfolioEventPublisher "Emits the portfolio export part" "Supabase Queues publisher" {
                    tags "Async"
                }
                portfolioEraseDataUseCase -> portfolioRepository "Anonymises portfolio-owned data" "repository port"
                portfolioEraseDataUseCase -> portfolioEventPublisher "Emits an erasure confirmation" "Supabase Queues publisher" {
                    tags "Async"
                }
                portfolioApplyRetentionUseCase -> portfolioRepository "Drops expired snapshot partitions" "privileged repository"
                portfolioApplyRetentionUseCase -> portfolioAdminPolicy "Re-checks the admin claim for interactive runs or the run-scoped System assertion for scheduled runs" "in-process"
                portfolioApplyRetentionUseCase -> portfolioRunOutcomePublisher "Publishes the retention run outcome" "Supabase Queues publisher" {
                    tags "Async"
                }
                portfolioSnapshot -> portfolioValuationCalculator "Stores reproducible calculated values" "in-process"
                portfolioRecommendation -> portfolioPortfolioAggregate "Reads targets, guardrails and allocation" "in-process"
                portfolioPortfolioAggregate -> portfolioConfigurationPolicy "Applies name, single-default and target-weight invariants" "in-process"
                portfolioAssertionVerifier -> supabaseVault "Resolves the assertion verification public keys" "Supabase Vault (pgsodium)"
            }

            reportingService = container "Reporting Service" "Owns reporting preferences and the scheduled brief pipeline. Composes daily and weekly briefs, owns structure, disclaimer and final validation, delegates narrative composition and Notion publication to the OpenClaw Agent under a least-privilege scoped token and a versioned redacted payload contract, enforces idempotency per owner, portfolio and period, retries within bounds with pgmq archive and dead-lettering, applies a retention schedule to briefs, narratives and delivery logs, and records every delivery attempt and run outcome. There is no direct Notion write path." "Rust, Axum, sqlx, reqwest" "Container" {
                group "Transport" {
                    reportingRestHandlers = component "Reporting REST Handlers" "Serves preferences, generate-now, brief history and delivery log endpoints." "Axum" "Transport"
                    reportingJobConsumer = component "Brief Dispatch Command Consumer" "Consumes scheduled brief-dispatch and reporting.retention commands from Supabase Queues with idempotency keyed on the logical run key (job + scheduled period) plus attempt number, exchanges the command for a short-lived run-scoped assertion over mTLS before executing, and honours a cancel command between briefs." "Supabase Queues (pgmq)" "Transport"
                    reportingDomainEventConsumer = component "Domain Event Consumer" "Consumes snapshot, valuation, recommendation and portfolio events from Supabase Queues and triggers due-brief dispatch for the affected owner and portfolio." "Supabase Queues (pgmq)" "Transport"
                    reportingInternalHandler = component "Internal Brief Handler" "Serves internal brief lookup by period." "Axum, mTLS" "Transport"
                    reportingInternalAuthGuard = component "Internal Principal Guard" "Validates the signed internal assertion (gateway user assertion or Job Orchestration System-role assertion) and enforces owner scoping on internal endpoints. Forwarding rule: user-originated hops forward the gateway assertion unchanged; scheduled and recovery hops carry the System-role assertion from the run command." "Axum middleware, mTLS" "Transport"
                    reportingRetryHandler = component "Delivery Retry Handler" "Accepts delivery retry requests and delegates authorisation, bounds and rate limits to the Retry Delivery Use Case." "Axum" "Transport"
                    reportingPrivacyConsumer = component "Privacy Request Consumer" "Consumes export and erasure requests for reporting-owned data from Supabase Queues." "Supabase Queues (pgmq)" "Transport"
                    reportingHealthHandler = component "Health and Readiness Endpoints" "Exposes liveness and readiness probes and reports repository readiness." "Axum" "Transport"
                }
                group "Application" {
                    reportingManagePreferencesUseCase = component "Manage Preferences Use Case" "Manages destination, daily and weekly schedules, disablement and generate-now." "Rust" "Application"
                    reportingGetBriefHistoryUseCase = component "Get Brief History Use Case" "Returns brief history and the delivery log for the owning user or an admin." "Rust" "Application"
                    reportingGetBriefByPeriodUseCase = component "Get Brief By Period Use Case" "Returns the brief and delivery record for an owner and period without side effects, for the internal brief-lookup endpoint." "Rust" "Application"
                    reportingComposeBriefUseCase = component "Compose Brief Use Case" "Fetches brief data once and composes structure, value, drift, holdings, recommendations, freshness and disclaimer." "Rust" "Application"
                    reportingPublishBriefUseCase = component "Publish Brief Use Case" "Sends the redacted payload, validates the agent response, verifies the delivery record and external reference before publication is final." "Rust" "Application"
                    reportingDispatchDueBriefsUseCase = component "Dispatch Due Briefs Use Case" "Finds due briefs, composes and publishes each one idempotently, and stops between briefs when the run is cancelled." "Rust" "Application"
                    reportingRetryDeliveryUseCase = component "Retry Delivery Use Case" "Authorises a retry against the owning user or an admin, applies the retry policy bounds and rate limits, and delegates to the publish use case." "Rust" "Application"
                    reportingExportDataUseCase = component "Export Reporting Data Use Case" "Assembles the reporting-preferences and delivery-history portion of a personal-data export." "Rust" "Application"
                    reportingEraseDataUseCase = component "Erase Reporting Data Use Case" "Anonymises reporting-owned data subject to legal hold and emits an erasure confirmation." "Rust" "Application"
                    reportingApplyRetentionUseCase = component "Apply Reporting Retention Use Case" "Deletes or archives expired briefs, narratives and delivery logs under the reporting retention schedule and legal hold, and emits retention-applied and run-outcome events." "Rust" "Application"
                }
                group "Domain" {
                    reportingBrief = component "Brief" "Daily or weekly brief with period, structured content and data freshness markers." "Rust" "Domain"
                    reportingBriefComposer = component "Brief Composer" "Builds the structured brief from valuation, drift, holdings and pending recommendations." "Rust" "Domain"
                    reportingDeliveryRecord = component "Delivery Record" "Status, timestamp, external Notion reference and failure reason for each attempt." "Rust" "Domain"
                    reportingIdempotencyPolicy = component "Idempotent Delivery Policy" "One brief per owner, portfolio and period with safe retry semantics." "Rust" "Domain"
                    reportingDeliveryRetryPolicy = component "Delivery Retry Policy" "Bounded retries with exponential backoff, pgmq archive and dead-letter after the limit, and duplicate detection through the delivery record and external reference." "Rust" "Domain"
                    reportingBriefPayloadPolicy = component "Brief Payload Contract" "Versioned, minimal redaction allowlist for the OpenClaw payload, plus validation of the returned narrative before publication." "Rust" "Domain"
                    reportingAdminPolicy = component "Admin Authorization Policy" "Requires the Supabase `app_metadata.role` admin claim projected by the Custom Access Token Hook for administrative reporting routes, and accepts a run-scoped System assertion bound to the run key for scheduled retention." "Rust" "Domain"
                }
                group "Infrastructure" {
                    reportingBriefRepository = component "Supabase Postgres Brief Repository" "Stores preferences, schedules, briefs, delivery log, idempotency keys, outbox and inbox dedup records in the reporting schema." "Rust, sqlx" "Infrastructure"
                    reportingPortfolioClient = component "Portfolio Brief Data Client" "Fetches a consistent single-owner brief data set over mTLS." "Rust, reqwest, mTLS" "Infrastructure"
                    reportingOpenClawClient = component "OpenClaw Agent Client" "Submits the redacted brief payload with an idempotency key and a per-call timeout and records the publication outcome." "Rust, reqwest" "Infrastructure"
                    reportingSecretsReader = component "Supabase Vault Secret Reader" "Resolves the scoped OpenClaw token reference from Supabase Vault without exposing secrets to briefs, logs or events. Notion credentials stay outside the platform and are never resolved here." "Supabase Vault (pgsodium)" "Infrastructure"
                    reportingEventPublisher = component "Supabase Queues Reporting Event Publisher" "Writes brief, delivery and audit events through a transactional outbox to Supabase Queues." "Supabase Queues (pgmq)" "Infrastructure"
                    reportingRunOutcomePublisher = component "Supabase Queues Run Outcome Publisher" "Publishes brief-dispatch and retention run outcomes and heartbeats." "Supabase Queues (pgmq)" "Infrastructure"
                    reportingAssertionVerifier = component "Principal Assertion Verifier" "Resolves the gateway user-assertion and Job Orchestration System-assertion public keys from Supabase Vault, caches them with a bounded refresh, verifies signature, issuer, expiry and correlation id, and rejects any other issuer." "Rust, Ed25519" "Infrastructure"
                }
                reportingRestHandlers -> reportingManagePreferencesUseCase "Manages destination, schedule and generate-now" "in-process"
                reportingRestHandlers -> reportingAssertionVerifier "Verifies the gateway assertion on public REST routes" "in-process"
                reportingRestHandlers -> reportingGetBriefHistoryUseCase "Serves brief history and the delivery log" "in-process"
                reportingJobConsumer -> reportingDispatchDueBriefsUseCase "Dispatches due daily and weekly briefs idempotently per run key and attempt" "Supabase Queues consumer" {
                    tags "Async"
                }
                reportingJobConsumer -> reportingApplyRetentionUseCase "Runs scheduled reporting retention idempotently per run key and attempt" "Supabase Queues consumer" {
                    tags "Async"
                }
                reportingDomainEventConsumer -> reportingDispatchDueBriefsUseCase "Triggers due-brief dispatch when a portfolio event makes a brief due" "Supabase Queues consumer" {
                    tags "Async"
                }
                reportingInternalHandler -> reportingInternalAuthGuard "Requires a signed service principal" "in-process"
                reportingInternalAuthGuard -> reportingAssertionVerifier "Verifies the assertion with the Supabase Vault-resolved public key and cache" "in-process"
                reportingInternalAuthGuard -> reportingGetBriefByPeriodUseCase "Allows asserted internal brief lookup" "in-process"
                reportingRetryHandler -> reportingAssertionVerifier "Verifies the gateway assertion before the retry" "in-process"
                reportingRetryHandler -> reportingRetryDeliveryUseCase "Delegates retry authorisation and bounds to the application use case" "in-process"
                reportingRetryDeliveryUseCase -> reportingPublishBriefUseCase "Retries the failed delivery through the publish use case" "in-process"
                reportingRetryDeliveryUseCase -> reportingDeliveryRetryPolicy "Applies retry authorization, bounds and rate limits" "in-process"
                reportingRetryDeliveryUseCase -> reportingBriefRepository "Confirms the delivery record belongs to the owner or to an admin" "repository port"
                reportingPrivacyConsumer -> reportingExportDataUseCase "Routes export requests" "Supabase Queues consumer" {
                    tags "Async"
                }
                reportingPrivacyConsumer -> reportingEraseDataUseCase "Routes erasure requests" "Supabase Queues consumer" {
                    tags "Async"
                }
                reportingHealthHandler -> reportingBriefRepository "Reports repository readiness" "in-process"
                reportingManagePreferencesUseCase -> reportingBriefRepository "Persists preferences and schedules" "repository port"
                reportingGetBriefHistoryUseCase -> reportingBriefRepository "Reads brief history and delivery records" "repository port"
                reportingGetBriefByPeriodUseCase -> reportingBriefRepository "Reads the brief and delivery record for the requested period" "repository port"
                reportingComposeBriefUseCase -> reportingPortfolioClient "Fetches single-owner brief data once" "mTLS REST client"
                reportingComposeBriefUseCase -> reportingBriefComposer "Composes the structured brief" "in-process"
                reportingComposeBriefUseCase -> reportingBrief "Produces a brief for the period" "in-process"
                reportingComposeBriefUseCase -> reportingBriefPayloadPolicy "Selects only allowlisted redacted fields" "in-process"
                reportingComposeBriefUseCase -> reportingBriefRepository "Stores the generated brief" "repository port"
                reportingPublishBriefUseCase -> reportingOpenClawClient "Requests narrative and Notion publication with a bounded timeout" "HTTPS client"
                reportingPublishBriefUseCase -> reportingSecretsReader "Resolves the scoped OpenClaw token" "Supabase Vault client"
                reportingPublishBriefUseCase -> reportingIdempotencyPolicy "Enforces one delivery per period" "in-process"
                reportingPublishBriefUseCase -> reportingBriefPayloadPolicy "Validates the returned narrative before publication" "in-process"
                reportingPublishBriefUseCase -> reportingDeliveryRetryPolicy "Applies bounded retries and dead-lettering" "in-process"
                reportingPublishBriefUseCase -> reportingDeliveryRecord "Records status, reference and error" "in-process"
                reportingPublishBriefUseCase -> reportingBriefRepository "Persists delivery outcomes" "repository port"
                reportingPublishBriefUseCase -> reportingEventPublisher "Publishes brief published and delivery-failure events" "Supabase Queues publisher" {
                    tags "Async"
                }
                reportingDispatchDueBriefsUseCase -> reportingComposeBriefUseCase "Composes each due brief" "in-process"
                reportingDispatchDueBriefsUseCase -> reportingPublishBriefUseCase "Publishes each due brief" "in-process"
                reportingDispatchDueBriefsUseCase -> reportingRunOutcomePublisher "Publishes the dispatch run outcome and heartbeat" "Supabase Queues publisher" {
                    tags "Async"
                }
                reportingExportDataUseCase -> reportingBriefRepository "Reads attributable reporting data" "repository port"
                reportingExportDataUseCase -> reportingEventPublisher "Emits the reporting export part" "Supabase Queues publisher" {
                    tags "Async"
                }
                reportingEraseDataUseCase -> reportingBriefRepository "Anonymises reporting-owned data" "repository port"
                reportingEraseDataUseCase -> reportingEventPublisher "Emits an erasure confirmation" "Supabase Queues publisher" {
                    tags "Async"
                }
                reportingApplyRetentionUseCase -> reportingBriefRepository "Deletes or archives expired briefs, narratives and delivery logs" "repository port"
                reportingApplyRetentionUseCase -> reportingAdminPolicy "Re-checks the admin claim for interactive runs or the run-scoped System assertion for scheduled runs" "in-process"
                reportingApplyRetentionUseCase -> reportingEventPublisher "Publishes retention-applied events" "Supabase Queues publisher" {
                    tags "Async"
                }
                reportingApplyRetentionUseCase -> reportingRunOutcomePublisher "Publishes the retention run outcome and heartbeat" "Supabase Queues publisher" {
                    tags "Async"
                }
                reportingSecretsReader -> supabaseVault "Reads the scoped OpenClaw token reference" "Supabase Vault (pgsodium)"
                reportingAssertionVerifier -> supabaseVault "Resolves the assertion verification public keys" "Supabase Vault (pgsodium)"
            }

            jobOrchestrationService = container "Job Orchestration Service (Scheduler / Job Runner)" "Control plane and scheduler for non-interactive work. Stores durable UTC schedules, consumes job.tick signals that Supabase Cron enqueues into Supabase Queues, issues asynchronous run commands keyed by a logical run key (job + scheduled period) with a per-attempt run id and a heartbeat-renewed, TTL-bounded lease, enforces single-flight with Supabase Postgres advisory locks, records run outcomes and heartbeats, expires stale runs with a fencing lease epoch, supports admin cancel and bounded backfill, detects missed runs, applies bounded retries, raises alerts within 15 minutes, audits admin manual triggers, and exchanges a run command for a short-lived run-scoped System assertion so no credential ever travels on the queue. Supabase Cron runs the independent dead-man watchdog that raises an alert through pg_net when scheduler heartbeats stop. Owns no business data." "Rust, Axum, sqlx, tokio" "Container" {
                group "Transport" {
                    jobAdminHandlers = component "Admin Job Handlers" "Serves job listing, schedule updates, manual triggers, cancel, backfill and run history behind a first-class admin guard, with stricter gateway rate limits on admin routes." "Axum" "Transport"
                    jobAdminGuard = component "Admin Role Guard Middleware" "Re-checks the Supabase `app_metadata.role` admin claim on every job admin route, including listing and run history, and rejects portfolio-ownership-only callers before the route executes." "Axum middleware" "Transport"
                    jobInternalAuthGuard = component "Internal Principal Guard" "Validates the signed internal assertion (gateway user assertion or Job Orchestration run-scoped System-role assertion) and enforces the admin claim on internal endpoints. Forwarding rule: user-originated hops forward the gateway assertion unchanged; scheduled and recovery hops carry the run-scoped assertion obtained from the run-assertion exchange." "Axum middleware, mTLS" "Transport"
                    jobOutcomeConsumer = component "Run Outcome Consumer" "Consumes run outcomes and heartbeats from Supabase Queues with inbox dedup." "Supabase Queues (pgmq)" "Transport"
                    jobSchedulerWorker = component "Scheduler Tick Loop" "Consumes job.tick signals from Supabase Queues and dispatches due jobs." "Rust, tokio" "Transport"
                    jobWatchdogWorker = component "Missed-Run Watchdog Loop" "Runs missed-run detection and stale-run expiry on a fixed interval independent of health probes." "Rust, tokio" "Transport"
                    jobInternalHealthHandler = component "Internal Job Health Handler" "Serves internal scheduler health and last missed-run state as a side-effect-free probe for assertion-authorised internal callers." "Axum, mTLS" "Transport"
                    jobRunAssertionHandler = component "Run Assertion Exchange Handler" "Serves the run-assertion exchange over mTLS: a service holding a run command for a run key and attempt obtains a short-lived, run-scoped System assertion bound to that run key, lease epoch, audience and permitted owner set. The handler checks the mTLS caller identity is the run's owning service, so another service cannot obtain an assertion for a run it does not own. No credential is carried in the command or placed on the queue." "Axum, mTLS" "Transport"
                    jobHealthHandler = component "Health and Readiness Endpoints" "Exposes liveness and readiness probes and reports repository readiness." "Axum" "Transport"
                }
                group "Application" {
                    jobTickSchedulerUseCase = component "Tick Scheduler Use Case" "Handles a job.tick signal by loading due definitions, acquiring single-flight and a heartbeat-renewed lease, persisting the last dispatched period and attempt, and issuing run commands carrying the logical run key and per-attempt id; repeated ticks in the same period are no-ops." "Rust" "Application"
                    jobTriggerManualRunUseCase = component "Trigger Manual Run Use Case" "Authorises, rate-limits and audits an administrator-triggered run under a logical run key and first attempt id." "Rust" "Application"
                    jobRecordRunOutcomeUseCase = component "Record Run Outcome Use Case" "Records start, end, outcome, attempt number, affected-record counts and heartbeats." "Rust" "Application"
                    jobRetryFailedRunUseCase = component "Retry Failed Run Use Case" "Evaluates retry eligibility on failure and schedules a bounded retry with a new attempt while keeping side effects idempotent." "Rust" "Application"
                    jobDetectMissedRunsUseCase = component "Detect Missed Runs Use Case" "Detects missed or failed runs and publishes alerts within 15 minutes." "Rust" "Application"
                    jobListJobsUseCase = component "List Jobs Use Case" "Returns job definitions, schedules and run history for administrators." "Rust" "Application"
                    jobUpdateScheduleUseCase = component "Update Job Schedule Use Case" "Validates and persists UTC schedule changes with an audit event." "Rust" "Application"
                    jobExpireStaleRunsUseCase = component "Expire Stale Runs Use Case" "Detects runs whose lease TTL elapsed without a heartbeat, marks the attempt failed, increments the lease epoch so a live but partitioned executor can no longer write, releases single-flight and schedules a bounded retry." "Rust" "Application"
                    jobCancelRunUseCase = component "Cancel Run Use Case" "Authorises an admin cancel, marks the run cancelled, publishes a cancel command and releases single-flight; consumers check for cancellation between shards." "Rust" "Application"
                    jobBackfillRunsUseCase = component "Request Backfill Use Case" "Issues a bounded backfill command for a closed period window under a distinct backfill run key and a hard maximum window, refusing overlapping single-flight runs." "Rust" "Application"
                    jobIssueRunAssertionUseCase = component "Issue Run Assertion Use Case" "Validates a run key and attempt against the recorded run and current lease epoch, and issues a short-lived run-scoped System assertion for the owning service." "Rust" "Application"
                }
                group "Domain" {
                    jobDefinition = component "Job Definition" "Seeded job keys including catalogue.refresh, prices.collect, accounts.sync, snapshots.eod, recommendations.expire, briefs.dispatch, projections.reconcile, audit.retention, prices.retention, snapshots.retention, reporting.retention and identity.retention." "Rust" "Domain"
                    jobSchedule = component "Job Schedule" "Durable UTC schedule that survives restarts and applies ordering constraints." "Rust" "Domain"
                    jobRun = component "Job Run" "Logical run key (job + scheduled period, or job + manual/backfill occurrence), per-attempt run id, attempt number, monotonically increasing lease epoch used as a fencing token, start, end, outcome, affected-record counts and error summary." "Rust" "Domain"
                    jobRetryPolicy = component "Retry Policy" "Bounded automatic retry with attempt counts, backoff and a new side-effect-safe attempt id under the same logical run key; exhausted retries raise an alert." "Rust" "Domain"
                    jobSingleFlightPolicy = component "Single-Flight Policy" "Rejects or queues overlapping runs of the same job." "Rust" "Domain"
                    jobRunLeasePolicy = component "Run Lease and Idempotency Policy" "Assigns the logical run key (job + scheduled period, or job + manual/backfill occurrence), a per-attempt run id and a monotonically increasing lease epoch as a fencing token, holds a lease with a bounded TTL renewed by heartbeats, and defines idempotency on (run key, attempt): a redelivery of the same attempt is rejected while a retry with a new attempt is accepted. Consumers compare the fencing token immediately before each write and reject a command or outcome bearing an older epoch. A lease that expires without a heartbeat is treated as a failed attempt and its epoch is superseded. Each owning service performs the epoch comparison inside the same database transaction as its write, so a partitioned executor with an unexpired assertion cannot write after its epoch is superseded." "Rust" "Domain"
                    jobAdminPolicy = component "Admin Authorization Policy" "Requires the Supabase `app_metadata.role` admin claim projected by the Custom Access Token Hook for every job admin route, including listing, run history, schedule updates, manual triggers, cancellation and backfill." "Rust" "Domain"
                }
                group "Infrastructure" {
                    jobDefinitionRepository = component "Supabase Postgres Job Definition Repository" "Stores job definitions and UTC schedules with outbox and inbox dedup records in the job schema." "Rust, sqlx" "Infrastructure"
                    jobRunRepository = component "Supabase Postgres Run Repository" "Stores runs, outcomes, heartbeats, alert state and trigger audit in the job schema." "Rust, sqlx" "Infrastructure"
                    jobAdvisoryLock = component "Supabase Postgres Advisory Lock" "Provides single-flight ownership across scheduler replicas." "Supabase Postgres advisory locks" "Infrastructure"
                    jobRunCommandPublisher = component "Supabase Queues Run Command Publisher" "Publishes job.run.requested commands carrying the logical run key, attempt id, lease epoch and correlation id, never a credential or assertion, plus cancel, backfill, missed-run events, alerts and trigger audit events to Supabase Queues." "Supabase Queues (pgmq)" "Infrastructure"
                    jobSystemAssertionSigner = component "System Assertion Signer" "Mints the short-lived run-scoped System-role assertion for a verified run command. The assertion is bound to the run key, lease epoch, target audience and permitted owner set, is handed to the owning service by the run-assertion exchange over mTLS, and is never transported in an event payload." "Rust, Ed25519" "Infrastructure"
                    jobAlertWebhookClient = component "Alerting Webhook Client" "Sends missed-run, failed-schedule and sustained-error alerts to the operational alerting channel with bounded retry." "Rust, reqwest" "Infrastructure"
                    jobSecretsClient = component "Supabase Vault Secret Client" "Resolves database credentials and the System-assertion signing key from Supabase Vault." "Supabase Vault (pgsodium)" "Infrastructure"
                    jobAssertionVerifier = component "Principal Assertion Verifier" "Resolves the gateway user-assertion and Job Orchestration System-assertion public keys from Supabase Vault, caches them with a bounded refresh, verifies signature, issuer, expiry, audience, run key, lease epoch and correlation id, and rejects any other issuer." "Rust, Ed25519" "Infrastructure"
                }
                jobAdminHandlers -> jobTriggerManualRunUseCase "Triggers a job on demand with rate limiting" "in-process"
                jobAdminHandlers -> jobListJobsUseCase "Lists job definitions, schedules and run history" "in-process"
                jobAdminHandlers -> jobUpdateScheduleUseCase "Updates UTC job schedules" "in-process"
                jobAdminHandlers -> jobCancelRunUseCase "Cancels a running job on demand" "in-process"
                jobAdminHandlers -> jobBackfillRunsUseCase "Requests a bounded backfill for a closed period window" "in-process"
                jobAdminHandlers -> jobAdminGuard "Requires the admin claim on every admin route" "in-process"
                jobAdminGuard -> jobInternalAuthGuard "Delegates assertion verification and the admin-claim re-check" "in-process"
                jobInternalAuthGuard -> jobTriggerManualRunUseCase "Allows only the asserted admin claim" "in-process"
                jobInternalAuthGuard -> jobAssertionVerifier "Verifies the assertion with the Supabase Vault-resolved public key and cache" "in-process"
                jobOutcomeConsumer -> jobRecordRunOutcomeUseCase "Records run outcomes and heartbeats" "Supabase Queues consumer" {
                    tags "Async"
                }
                jobSchedulerWorker -> jobTickSchedulerUseCase "Dispatches job.tick signals to the tick scheduler" "in-process"
                jobWatchdogWorker -> jobDetectMissedRunsUseCase "Runs missed-run detection on a fixed watchdog interval" "in-process"
                jobWatchdogWorker -> jobExpireStaleRunsUseCase "Expires stale runs on each watchdog interval" "in-process"
                jobHealthHandler -> jobRunRepository "Reports repository readiness" "in-process"
                jobInternalHealthHandler -> jobInternalAuthGuard "Requires the signed internal assertion" "in-process"
                jobInternalHealthHandler -> jobRunRepository "Reports scheduler and last-run readiness without side effects" "in-process"
                jobRunAssertionHandler -> jobIssueRunAssertionUseCase "Issues a run-scoped assertion for a verified command" "in-process"
                jobIssueRunAssertionUseCase -> jobRunRepository "Confirms the run key, attempt and current lease epoch" "repository port"
                jobIssueRunAssertionUseCase -> jobRunLeasePolicy "Confirms the attempt is current for the lease epoch" "in-process"
                jobIssueRunAssertionUseCase -> jobSystemAssertionSigner "Mints the run-scoped System assertion" "in-process"
                jobTickSchedulerUseCase -> jobDefinitionRepository "Loads due job definitions and schedules" "repository port"
                jobTickSchedulerUseCase -> jobSchedule "Evaluates due schedules" "in-process"
                jobTickSchedulerUseCase -> jobSingleFlightPolicy "Enforces single-flight per job" "in-process"
                jobTickSchedulerUseCase -> jobRunLeasePolicy "Assigns the logical run key, attempt id and heartbeat-renewed lease" "in-process"
                jobTickSchedulerUseCase -> jobAdvisoryLock "Acquires Supabase Postgres advisory locks" "advisory lock"
                jobTickSchedulerUseCase -> jobRunCommandPublisher "Issues asynchronous run commands with logical run keys and attempt ids" "Supabase Queues publisher" {
                    tags "Async"
                }
                jobTriggerManualRunUseCase -> jobDefinitionRepository "Reads and validates the targeted definition" "repository port"
                jobTriggerManualRunUseCase -> jobSingleFlightPolicy "Rejects overlapping manual runs" "in-process"
                jobTriggerManualRunUseCase -> jobRunLeasePolicy "Assigns the logical run key and first attempt" "in-process"
                jobTriggerManualRunUseCase -> jobRunRepository "Records the trigger audit" "repository port"
                jobTriggerManualRunUseCase -> jobRunCommandPublisher "Issues the manual run command and trigger audit event" "Supabase Queues publisher" {
                    tags "Async"
                }
                jobTriggerManualRunUseCase -> jobAdminPolicy "Re-checks the asserted admin claim" "in-process"
                jobCancelRunUseCase -> jobRun "Marks the run cancelled" "in-process"
                jobCancelRunUseCase -> jobRunRepository "Releases single-flight and records the cancellation" "repository port"
                jobCancelRunUseCase -> jobRunCommandPublisher "Publishes the cancel command so consumers stop between shards" "Supabase Queues publisher" {
                    tags "Async"
                }
                jobCancelRunUseCase -> jobAdminPolicy "Re-checks the asserted admin claim" "in-process"
                jobBackfillRunsUseCase -> jobSingleFlightPolicy "Refuses a backfill that overlaps an active run" "in-process"
                jobBackfillRunsUseCase -> jobRunLeasePolicy "Assigns a distinct backfill run key and bounded attempts" "in-process"
                jobBackfillRunsUseCase -> jobRunRepository "Records the backfill request and audit" "repository port"
                jobBackfillRunsUseCase -> jobRunCommandPublisher "Issues the bounded backfill commands" "Supabase Queues publisher" {
                    tags "Async"
                }
                jobBackfillRunsUseCase -> jobAdminPolicy "Re-checks the asserted admin claim" "in-process"
                jobRecordRunOutcomeUseCase -> jobRun "Records start, end and affected counts" "in-process"
                jobRecordRunOutcomeUseCase -> jobRunRepository "Persists run outcomes" "repository port"
                jobRecordRunOutcomeUseCase -> jobRunLeasePolicy "Renews the lease on each heartbeat" "in-process"
                jobRecordRunOutcomeUseCase -> jobDetectMissedRunsUseCase "Feeds missed-run evaluation" "in-process"
                jobRecordRunOutcomeUseCase -> jobRetryFailedRunUseCase "Evaluates retry eligibility on a failed outcome" "in-process"
                jobRetryFailedRunUseCase -> jobRetryPolicy "Applies bounded retry and attempt rules" "in-process"
                jobRetryFailedRunUseCase -> jobRun "Creates the next attempt for the run" "in-process"
                jobRetryFailedRunUseCase -> jobRunRepository "Records the retry attempt" "repository port"
                jobRetryFailedRunUseCase -> jobRunCommandPublisher "Issues the retry command with the same logical run key and the next attempt id" "Supabase Queues publisher" {
                    tags "Async"
                }
                jobExpireStaleRunsUseCase -> jobRunLeasePolicy "Applies lease TTL and heartbeat expiry rules" "in-process"
                jobExpireStaleRunsUseCase -> jobRun "Marks the stale attempt failed" "in-process"
                jobExpireStaleRunsUseCase -> jobRunRepository "Releases single-flight and persists the failed outcome" "repository port"
                jobExpireStaleRunsUseCase -> jobRetryFailedRunUseCase "Schedules a bounded retry for the stale run" "in-process"
                jobTickSchedulerUseCase -> jobRetryFailedRunUseCase "Dispatches due retries on each tick" "in-process"
                jobListJobsUseCase -> jobDefinitionRepository "Reads definitions and schedules" "repository port"
                jobListJobsUseCase -> jobRunRepository "Reads run history" "repository port"
                jobListJobsUseCase -> jobAdminPolicy "Re-checks the asserted admin claim for listing reads" "in-process"
                jobUpdateScheduleUseCase -> jobSchedule "Validates the UTC schedule change" "in-process"
                jobUpdateScheduleUseCase -> jobDefinitionRepository "Persists the updated schedule" "repository port"
                jobUpdateScheduleUseCase -> jobAdminPolicy "Re-checks the asserted admin claim" "in-process"
                jobUpdateScheduleUseCase -> jobRunCommandPublisher "Publishes the schedule-change audit event" "Supabase Queues publisher" {
                    tags "Async"
                }
                jobDetectMissedRunsUseCase -> jobRun "Detects missed or failed runs" "in-process"
                jobDetectMissedRunsUseCase -> jobRunRepository "Reads last successful runs" "repository port"
                jobDetectMissedRunsUseCase -> jobRunCommandPublisher "Publishes missed-run events" "Supabase Queues publisher" {
                    tags "Async"
                }
                jobDetectMissedRunsUseCase -> jobAlertWebhookClient "Sends missed-run and failed-schedule alerts" "HTTPS webhook"
                jobAlertWebhookClient -> alertingChannel "Delivers operational alerts" "HTTPS webhook"
                jobDefinitionRepository -> jobSecretsClient "Resolves database credentials from Supabase Vault" "Supabase Vault client"
                jobSecretsClient -> supabaseVault "Reads database and assertion-signing key references" "Supabase Vault (pgsodium)"
                jobSystemAssertionSigner -> jobSecretsClient "Resolves the System-assertion signing key" "Supabase Vault client"
                jobAssertionVerifier -> supabaseVault "Resolves the assertion verification public keys" "Supabase Vault (pgsodium)"
                jobSchedule -> jobDefinition "Applies cron definitions" "in-process"
                jobRunLeasePolicy -> jobRun "Guards the run lifecycle" "in-process"
            }

            auditService = container "Audit Service" "Append-only compliance store and privacy operations. Consumes security- and financially-material events into per-partition hash chains serialised by a Supabase Postgres advisory-lock writer lease, so only one replica appends to a given partition at a time, runs retention as archival/partition drop under a separate audited privileged role, assembles personal-data exports in Supabase Storage and coordinates erasure by crypto-shredding, with pseudonymisation only on the ingestion path and never by mutating hash-chained rows. Application users can query but never modify audit data." "Rust, Axum, sqlx" "Container" {
                group "Transport" {
                    auditAdminHandlers = component "Admin Audit Handlers" "Serves audit event queries, export requests and retention administration." "Axum" "Transport"
                    auditAdminGuard = component "Admin Role Guard Middleware" "Re-checks the Supabase `app_metadata.role` admin claim on audit query, export and retention routes." "Axum middleware" "Transport"
                    auditPrivacyHandlers = component "Privacy Handlers" "Accepts personal-data export and erasure requests and exposes processor disclosures." "Axum" "Transport"
                    auditPrivacyGuard = component "Privacy Request Guard" "Requires an authenticated owner principal and enforces owner scoping on export and erasure requests." "Axum middleware" "Transport"
                    auditInternalAuthGuard = component "Internal Principal Guard" "Validates the signed internal assertion (gateway user assertion or Job Orchestration System-role assertion) and enforces the admin claim on internal endpoints. Forwarding rule: user-originated hops forward the gateway assertion unchanged; scheduled and recovery hops carry the System-role assertion from the run command." "Axum middleware, mTLS" "Transport"
                    auditEventConsumer = component "Audit Event Consumer" "Consumes security- and financially-material audit events from every context via Supabase Queues with inbox dedup keyed by event id and chain partition, and submits each append to the partition writer lock." "Supabase Queues (pgmq)" "Transport"
                    auditExportConsumer = component "Export Part Consumer" "Consumes per-service export parts and erasure confirmations from Supabase Queues." "Supabase Queues (pgmq)" "Transport"
                    auditJobConsumer = component "Job Command Consumer" "Consumes audit.retention commands from Supabase Queues with idempotency keyed on the logical run key (job + scheduled period) plus attempt number, exchanges the command for a short-lived run-scoped assertion over mTLS before executing, and honours a cancel command." "Supabase Queues (pgmq)" "Transport"
                    auditHealthHandler = component "Health and Readiness Endpoints" "Exposes liveness and readiness probes and reports repository readiness." "Axum" "Transport"
                }
                group "Application" {
                    auditAppendEventUseCase = component "Append Audit Event Use Case" "Pseudonymises PII fields through the per-subject key, then appends actor, action, target, timestamp, source context and outcome with hash chaining and dedup." "Rust" "Application"
                    auditQueryEventsUseCase = component "Query Audit Events Use Case" "Queries append-only audit events for authorised administrators." "Rust" "Application"
                    auditRequestExportUseCase = component "Request Data Export Use Case" "Creates an export request and coordinates per-service export parts." "Rust" "Application"
                    auditAssembleExportUseCase = component "Assemble Export Use Case" "Assembles export parts into an archive in Supabase Storage." "Rust" "Application"
                    auditApplyRetentionUseCase = component "Apply Retention Use Case" "Applies retention and legal-hold policy under a privileged job using archival or partition drop." "Rust" "Application"
                    auditEraseDataUseCase = component "Erase or Anonymise Data Use Case" "Applies erasure by crypto-shredding per-subject keys while leaving hash-chained rows intact. Pseudonymisation happens at ingestion, so there is no post-hoc row mutation and no second concurrent writer to the chain." "Rust" "Application"
                }
                group "Domain" {
                    auditEvent = component "Audit Event" "Append-only event with actor, action, target, timestamp, source context, outcome and event id." "Rust" "Domain"
                    auditHashChainPolicy = component "Hash Chain Policy" "Maintains and verifies the tamper-evident audit hash chain." "Rust" "Domain"
                    auditChainWriterPolicy = component "Chain Writer Policy" "Single ordered writer per partition, enforced by the per-partition Supabase Postgres advisory-lock writer lease so concurrent replicas and consumers cannot interleave or fork a chain." "Rust" "Domain"
                    auditExportRequest = component "Data Export Request" "Export request lifecycle and per-service export parts." "Rust" "Domain"
                    auditRetentionPolicy = component "Retention Policy" "Retention periods and legal-hold state for audit and personal data." "Rust" "Domain"
                    auditPiiPseudonymisationPolicy = component "PII Pseudonymisation Policy" "Applies pseudonymisation to PII fields on the ingestion path before chaining, and erasure by crypto-shredding; immutable rows therefore contain no erasable plaintext PII." "Rust" "Domain"
                    auditAdminPolicy = component "Admin Authorization Policy" "Requires the Supabase `app_metadata.role` admin claim projected by the Custom Access Token Hook for audit queries, exports and interactive retention administration, and accepts a Job Orchestration run-scoped System assertion bound to the matching run key for scheduled retention." "Rust" "Domain"
                }
                group "Infrastructure" {
                    auditAppendOnlyRepository = component "Append-Only Audit Repository" "Inserts through an insert-only role in Supabase Postgres with UPDATE and DELETE revoked, and reads for authorised queries in the audit schema." "Rust, sqlx" "Infrastructure"
                    auditPrivilegedAdminRepository = component "Privileged Retention Repository" "Uses a separate audited role in Supabase Postgres for archival and partition drop, never row UPDATE or DELETE on the append-only role." "Rust, sqlx" "Infrastructure"
                    auditPrivacyCoordinator = component "Privacy Event Coordinator" "Publishes export and erasure requests and collects per-service parts through Supabase Queues." "Supabase Queues (pgmq)" "Infrastructure"
                    auditEventPublisher = component "Supabase Queues Privacy Event Publisher" "Publishes privacy requests, export-ready and retention-applied events to Supabase Queues." "Supabase Queues (pgmq)" "Infrastructure"
                    auditSecretsReader = component "Supabase Vault Secret Reader" "Resolves per-subject encryption keys used for crypto-shredding erasure from Supabase Vault." "Supabase Vault (pgsodium)" "Infrastructure"
                    auditRunOutcomePublisher = component "Supabase Queues Run Outcome Publisher" "Publishes audit.retention run outcomes and heartbeats." "Supabase Queues (pgmq)" "Infrastructure"
                    auditIdentityClient = component "Identity Lookup Client" "Calls Identity's internal user lookup over mTLS with timeout, bounded retry and circuit breaker." "Rust, reqwest, mTLS" "Infrastructure"
                    auditStorageClient = component "Supabase Storage Client" "Writes and reads encrypted export and retention archives in private Supabase Storage buckets through signed, access-controlled URLs." "Supabase Storage" "Infrastructure"
                    auditChainWriterLock = component "Chain Partition Writer Lock" "Serialises appends per chain partition with a Supabase Postgres advisory lock held for the append and a single-active writer lease, so only one replica extends a chain at a time." "Supabase Postgres advisory locks" "Infrastructure"
                    auditAssertionVerifier = component "Principal Assertion Verifier" "Resolves the gateway user-assertion and Job Orchestration System-assertion public keys from Supabase Vault, caches them with a bounded refresh, verifies signature, issuer, expiry and correlation id, and rejects any other issuer." "Rust, Ed25519" "Infrastructure"
                }
                auditAdminHandlers -> auditAdminGuard "Re-checks the admin claim on audit routes" "in-process"
                auditAdminGuard -> auditAssertionVerifier "Verifies the gateway assertion before the admin check" "in-process"
                auditAdminHandlers -> auditQueryEventsUseCase "Queries audit events for administrators" "in-process"
                auditAdminHandlers -> auditRequestExportUseCase "Serves admin-scoped export requests" "in-process"
                auditAdminHandlers -> auditApplyRetentionUseCase "Applies retention policy under a privileged job" "in-process"
                auditAdminHandlers -> auditInternalAuthGuard "Requires a signed admin principal" "in-process"
                auditInternalAuthGuard -> auditAssertionVerifier "Verifies the assertion with the Supabase Vault-resolved public key and cache" "in-process"
                auditInternalAuthGuard -> auditApplyRetentionUseCase "Allows only the asserted admin claim" "in-process"
                auditPrivacyHandlers -> auditPrivacyGuard "Requires an authenticated owner principal" "in-process"
                auditPrivacyGuard -> auditAssertionVerifier "Verifies the gateway assertion and owner scoping" "in-process"
                auditPrivacyHandlers -> auditRequestExportUseCase "Accepts export and erasure requests" "in-process"
                auditPrivacyHandlers -> auditEraseDataUseCase "Accepts erasure and anonymisation requests" "in-process"
                auditEventConsumer -> auditAppendEventUseCase "Appends security and financial events" "Supabase Queues consumer" {
                    tags "Async"
                }
                auditExportConsumer -> auditAssembleExportUseCase "Assembles per-service export parts" "Supabase Queues consumer" {
                    tags "Async"
                }
                auditJobConsumer -> auditApplyRetentionUseCase "Runs scheduled retention idempotently per run key and attempt" "Supabase Queues consumer" {
                    tags "Async"
                }
                auditHealthHandler -> auditAppendOnlyRepository "Reports repository readiness" "in-process"
                auditAppendEventUseCase -> auditPiiPseudonymisationPolicy "Pseudonymises PII fields through the per-subject key before the record enters the hash chain" "in-process"
                auditAppendEventUseCase -> auditSecretsReader "Resolves the per-subject pseudonymisation key before chaining" "Supabase Vault client"
                auditAppendEventUseCase -> auditEvent "Creates append-only audit events" "in-process"
                auditAppendEventUseCase -> auditHashChainPolicy "Maintains the hash chain" "in-process"
                auditAppendEventUseCase -> auditChainWriterPolicy "Enforces single ordered writer per partition" "in-process"
                auditAppendEventUseCase -> auditChainWriterLock "Acquires the per-partition writer lease before appending" "advisory lock"
                auditAppendEventUseCase -> auditAppendOnlyRepository "Inserts through the insert-only role" "repository port"
                auditQueryEventsUseCase -> auditAppendOnlyRepository "Reads audit events" "repository port"
                auditQueryEventsUseCase -> auditAdminPolicy "Re-checks the asserted admin claim" "in-process"
                auditRequestExportUseCase -> auditExportRequest "Records the export request" "in-process"
                auditRequestExportUseCase -> auditIdentityClient "Resolves the subject and owner mapping for the export" "mTLS REST client"
                auditRequestExportUseCase -> auditPrivacyCoordinator "Requests per-service export parts" "Supabase Queues coordinator" {
                    tags "Async"
                }
                auditAssembleExportUseCase -> auditExportRequest "Tracks export assembly" "in-process"
                auditAssembleExportUseCase -> auditStorageClient "Stores the assembled export archive in Supabase Storage" "Supabase Storage client"
                auditIdentityClient -> identityService "Looks up the user over mTLS with timeout, retry and circuit breaker" "mTLS REST client"
                auditApplyRetentionUseCase -> auditRetentionPolicy "Evaluates retention and legal hold" "in-process"
                auditApplyRetentionUseCase -> auditAdminPolicy "Re-checks the admin claim for interactive runs or the run-scoped System assertion for scheduled runs" "in-process"
                auditApplyRetentionUseCase -> auditPrivilegedAdminRepository "Archives or drops partitions under the privileged role" "privileged repository"
                auditApplyRetentionUseCase -> auditChainWriterLock "Acquires the same per-partition writer lock or verifies the partition is sealed before archival or drop" "advisory lock"
                auditApplyRetentionUseCase -> auditEventPublisher "Publishes retention-applied events" "Supabase Queues publisher" {
                    tags "Async"
                }
                auditApplyRetentionUseCase -> auditRunOutcomePublisher "Publishes the retention run outcome and heartbeat" "Supabase Queues publisher" {
                    tags "Async"
                }
                auditEraseDataUseCase -> auditPiiPseudonymisationPolicy "Applies the crypto-shredding rules; pseudonymisation is applied only at ingestion" "in-process"
                auditEraseDataUseCase -> auditPrivacyCoordinator "Publishes erasure requests to owning services and collects per-service confirmations" "Supabase Queues coordinator" {
                    tags "Async"
                }
                auditEraseDataUseCase -> auditIdentityClient "Resolves the owner mapping for anonymisation" "mTLS REST client"
                auditEraseDataUseCase -> auditSecretsReader "Destroys or rotates per-subject keys in Supabase Vault" "Supabase Vault client"
                auditEraseDataUseCase -> auditEventPublisher "Publishes erasure confirmation" "Supabase Queues publisher" {
                    tags "Async"
                }
                auditPrivacyCoordinator -> auditEventPublisher "Publishes privacy requests and export-ready events" "Supabase Queues publisher" {
                    tags "Async"
                }
                auditHashChainPolicy -> auditEvent "Verifies event chaining" "in-process"
                auditSecretsReader -> supabaseVault "Reads and destroys per-subject crypto-shredding keys" "Supabase Vault (pgsodium)"
                auditStorageClient -> supabaseStorage "Reads and writes export and retention archives in private buckets" "Supabase Storage API"
                auditAssertionVerifier -> supabaseVault "Resolves the assertion verification public keys" "Supabase Vault (pgsodium)"
            }

            portfolioOwner -> reactSpa "Uses the dashboard, accounts, portfolios, valuation, recommendations and reporting screens" "HTTPS"
            administrator -> reactSpa "Administers chains, tokens, jobs, audit and privacy requests" "HTTPS"
            reactSpa -> apiGateway "Calls the REST API and SSE sync-progress stream" "HTTPS/REST/JSON, Server-Sent Events"
            reactSpa -> supabaseAuth "Signs up, signs in with password, OAuth, magic link and MFA, refreshes sessions with PKCE and signs out; the only direct browser-to-Supabase exception" "Supabase Auth over HTTPS"
            apiGateway -> identityService "Ensures the internal user on first sighting and resolves owner id, status and roles on a cache miss" "REST over mTLS with Supabase access token or signed principal assertion"
            apiGateway -> accountService "Routes account, sync and sync-status requests including the SSE progress stream" "REST over mTLS with signed principal assertion"
            apiGateway -> assetRegistryService "Routes asset catalogue and admin chain and token requests" "REST over mTLS with signed principal assertion"
            apiGateway -> marketDataService "Routes price and freshness requests" "REST over mTLS with signed principal assertion"
            apiGateway -> portfolioService "Routes portfolio, valuation, snapshot and recommendation requests" "REST over mTLS with signed principal assertion"
            apiGateway -> reportingService "Routes reporting preferences, brief history, delivery log and generate-now" "REST over mTLS with signed principal assertion"
            apiGateway -> jobOrchestrationService "Routes admin job listing, scheduling, triggering and run history" "REST over mTLS with signed principal assertion"
            apiGateway -> auditService "Routes admin audit queries and user privacy export and erasure requests" "REST over mTLS with signed principal assertion"
            apiGateway -> supabaseAuth "Validates Supabase JWTs against the JWKS endpoint with a cached JWKS and a bounded stale-key grace period" "HTTPS JWKS"
            apiGateway -> supabasePostgres "Caches JWKS, principal status and roles and rate-limit counters" "Supabase Postgres (schema gateway, role gateway)"
            apiGateway -> supabaseQueues "Publishes security access-denied and rate-limited audit events and consumes identity events" "Supabase Queues (pgmq)" {
                tags "Async"
            }
            apiGateway -> supabaseVault "Reads internal-call credentials and assertion- and ticket-signing keys" "Supabase Vault (pgsodium)"
            apiGateway -> supabaseLogs "Logs, metrics and traces via log drain" "Log drain"
            identityService -> supabaseAuth "Handles auth hooks, validates Supabase JWTs against the JWKS endpoint and calls the Admin API for sessions, app_metadata and deletion" "Supabase Auth hooks, JWKS and Admin API over HTTPS"
            identityService -> supabasePostgres "Reads and writes internal users, Supabase user UUID mapping, roles, status, outbox and inbox dedup records" "Supabase Postgres (schema identity, role identity)"
            identityService -> supabaseQueues "Publishes user, role, session, deletion, retention-applied events and run outcomes and consumes privacy requests and identity.retention commands" "Supabase Queues (pgmq)" {
                tags "Async"
            }
            identityService -> supabaseVault "Reads auth, trust and database secret references" "Supabase Vault (pgsodium)"
            identityService -> supabaseLogs "Logs, metrics and traces via log drain" "Log drain"
            accountService -> supabasePostgres "Reads and writes accounts, credential ciphertext, holdings, sync runs and registry projection" "Supabase Postgres (schema account, role account)"
            accountService -> supabaseQueues "Publishes account lifecycle, versioned holdings, sync outcomes, unmapped observations, export/erasure confirmations and run outcomes and consumes registry events, job commands and privacy requests" "Supabase Queues (pgmq)" {
                tags "Async"
            }
            accountService -> supabaseVault "Encrypts and decrypts exchange credentials and resolves database credentials" "Supabase Vault (pgsodium)"
            accountService -> supabaseLogs "Logs, metrics and traces via log drain" "Log drain"
            accountService -> okxExchange "Reads balances only with read-only API credentials" "HTTPS REST with HMAC request signing"
            accountService -> evmRpcProviders "Reads native and ERC-20 balances and token metadata with provider rotation" "JSON-RPC 2.0 over HTTPS"
            accountService -> solanaRpc "Reads SOL and SPL balances and mint metadata" "JSON-RPC over HTTPS"
            accountService -> assetRegistryService "Fetches the registry snapshot only on cold start or reconciliation" "REST over mTLS"
            assetRegistryService -> supabasePostgres "Reads and writes assets, contracts, decimals, chains, token registries, quarantine and affected-account projection" "Supabase Postgres (schema registry, role registry)"
            assetRegistryService -> supabaseQueues "Publishes registry asset, contract, chain, token, registry-change audit, export/erasure confirmations and run-outcome events and consumes unmapped observations, account lifecycle events, job commands and privacy requests" "Supabase Queues (pgmq)" {
                tags "Async"
            }
            assetRegistryService -> supabaseVault "Resolves RPC provider credentials and lookup API keys" "Supabase Vault (pgsodium)"
            assetRegistryService -> supabaseLogs "Logs, metrics and traces via log drain" "Log drain"
            assetRegistryService -> priceDataProviders "Looks up contract addresses by symbol for admin review" "HTTPS REST/JSON"
            marketDataService -> supabasePostgres "Reads and writes price observations, partitioned history, provider mapping, freshness state, outbox and inbox dedup records" "Supabase Postgres (schema market, role market)"
            marketDataService -> supabaseQueues "Publishes price observations, batches, freshness, provider-health, export/erasure confirmations and run-outcome events and consumes registry asset events, price-collection commands and privacy requests" "Supabase Queues (pgmq)" {
                tags "Async"
            }
            marketDataService -> supabaseVault "Resolves provider API keys and database credentials" "Supabase Vault (pgsodium)"
            marketDataService -> supabaseLogs "Logs, metrics and traces via log drain" "Log drain"
            marketDataService -> priceDataProviders "Collects primary and fallback USD market observations within a shared provider budget" "HTTPS REST/JSON with API key"
            portfolioService -> supabasePostgres "Reads and writes portfolios, targets, guardrails, valuations, snapshots, recommendations, projections, outbox and inbox dedup records" "Supabase Postgres (schema portfolio, role portfolio)"
            portfolioService -> supabaseQueues "Publishes portfolio, valuation, snapshot, recommendation, audit, export/erasure confirmation and run-outcome events and consumes versioned holdings, account lifecycle, price, registry, job and privacy events" "Supabase Queues (pgmq)" {
                tags "Async"
            }
            portfolioService -> supabaseVault "Reads database credentials and projection rebuild credentials" "Supabase Vault (pgsodium)"
            portfolioService -> supabaseLogs "Logs, metrics and traces via log drain" "Log drain"
            portfolioService -> accountService "Rebuilds the holdings projection and validates account ownership and active status at link time" "REST over mTLS"
            portfolioService -> assetRegistryService "Rebuilds the asset projection after cold start or event-retention gap" "REST over mTLS"
            portfolioService -> marketDataService "Rebuilds the latest-price projection after cold start or schema change" "REST over mTLS"
            reportingService -> supabasePostgres "Reads and writes preferences, schedules, briefs, delivery log and idempotency keys" "Supabase Postgres (schema reporting, role reporting)"
            reportingService -> supabaseQueues "Publishes brief generated, published, delivery failure, audit, export/erasure confirmation, retention-applied and run-outcome events and consumes snapshot, valuation, recommendation, portfolio, job and privacy events" "Supabase Queues (pgmq)" {
                tags "Async"
            }
            reportingService -> supabaseVault "Resolves the scoped OpenClaw token reference and database credentials" "Supabase Vault (pgsodium)"
            reportingService -> supabaseLogs "Logs, metrics and traces via log drain" "Log drain"
            reportingService -> portfolioService "Fetches a consistent single-owner brief data set once per brief" "REST over mTLS"
            reportingService -> openClawAgent "Requests narrative composition and Notion publication with an idempotency key and bounded timeout" "HTTPS JSON API"
            openClawAgent -> notion "Writes the composed brief one-way to the configured destination" "HTTPS REST/JSON with scoped token"
            jobOrchestrationService -> supabasePostgres "Reads and writes job definitions, schedules, run records, heartbeats, lock state and trigger audit" "Supabase Postgres (schema job, role job)"
            jobOrchestrationService -> supabaseQueues "Consumes job.tick signals and publishes job run commands with logical run keys and attempt ids, missed-run events, alerts and manual-trigger audit events" "Supabase Queues (pgmq)" {
                tags "Async"
            }
            jobOrchestrationService -> supabaseVault "Reads database credentials and the System-assertion signing key" "Supabase Vault (pgsodium)"
            jobOrchestrationService -> supabaseLogs "Logs, metrics and traces via log drain" "Log drain"
            jobOrchestrationService -> alertingChannel "Sends missed-run, failed-schedule and sustained-error alerts" "Alert webhook"
            identityService -> jobOrchestrationService "Exchanges a run command for a short-lived run-scoped System assertion" "REST over mTLS"
            accountService -> jobOrchestrationService "Exchanges a run command for a short-lived run-scoped System assertion" "REST over mTLS"
            assetRegistryService -> jobOrchestrationService "Exchanges a run command for a short-lived run-scoped System assertion" "REST over mTLS"
            marketDataService -> jobOrchestrationService "Exchanges a run command for a short-lived run-scoped System assertion" "REST over mTLS"
            portfolioService -> jobOrchestrationService "Exchanges a run command for a short-lived run-scoped System assertion" "REST over mTLS"
            reportingService -> jobOrchestrationService "Exchanges a run command for a short-lived run-scoped System assertion" "REST over mTLS"
            auditService -> jobOrchestrationService "Exchanges a run command for a short-lived run-scoped System assertion" "REST over mTLS"
            auditService -> supabasePostgres "Appends hash-chained audit events through an insert-only role, applies archival retention through a separate privileged role, and queries for authorised admins" "Supabase Postgres (schema audit, role audit)"
            auditService -> supabaseQueues "Publishes privacy requests, export-ready, retention-applied and retention run-outcome events and consumes audit events, per-service export and erasure parts, and job commands" "Supabase Queues (pgmq)" {
                tags "Async"
            }
            auditService -> supabaseStorage "Stores and retrieves export and retention archives" "Supabase Storage API"
            auditService -> supabaseVault "Reads per-subject crypto-shredding keys and database credentials" "Supabase Vault (pgsodium)"
            auditService -> supabaseLogs "Logs, metrics and traces via log drain" "Log drain"
            auditService -> identityService "Resolves user profile and owner mapping for exports and anonymisation" "REST over mTLS"
            accountRegistrySnapshotClient -> assetRegistryService "Fetches the registry snapshot over mTLS for a cold-start rebuild" "mTLS REST client"
            portfolioAccountSnapshotClient -> accountService "Fetches owner holdings over mTLS for projection rebuild" "mTLS REST client"
            portfolioRegistrySnapshotClient -> assetRegistryService "Fetches the registry snapshot over mTLS for projection rebuild" "mTLS REST client"
            portfolioMarketPriceClient -> marketDataService "Fetches latest prices over mTLS for projection rebuild" "mTLS REST client"

        }

        portfolioOwner -> cpb "Uses read-only portfolio aggregation, valuation, recommendations and briefs" "HTTPS"
        administrator -> cpb "Administers chains, tokens, jobs, audit and privacy operations" "HTTPS"
        cpb -> supabaseAuth "Delegates authentication, MFA and session management" "Supabase Auth over HTTPS"
        cpb -> okxExchange "Reads balances from connected read-only exchange accounts" "HTTPS REST with HMAC request signing"
        cpb -> evmRpcProviders "Reads native and ERC-20 balances on configured EVM chains" "JSON-RPC 2.0 over HTTPS"
        cpb -> solanaRpc "Reads SOL and SPL balances for registered wallets" "JSON-RPC over HTTPS"
        cpb -> priceDataProviders "Collects USD market observations and admin token lookup candidates" "HTTPS REST/JSON"
        cpb -> openClawAgent "Requests narrative brief composition and publication" "HTTPS JSON API"
        cpb -> supabasePostgres "Stores every service's data in its own schema and role" "SQL over TLS via Supavisor"
        cpb -> supabaseQueues "Publishes and consumes domain events, job commands, run outcomes and privacy requests" "Supabase Queues (pgmq)"
        cpb -> supabaseStorage "Stores audit export and retention archives" "Supabase Storage API"
        cpb -> supabaseVault "Encrypts credentials and resolves scoped secret references" "Supabase Vault (pgsodium)"
        cpb -> supabaseLogs "Emits structured logs, metrics and traces with correlation and trace context" "Log drain"
        cpb -> alertingChannel "Routes missed-run, failed-schedule, error-rate and stale-price alerts" "Alert webhook"
        supabaseCron -> supabaseQueues "Enqueues job.tick for each due job on its schedule" "SQL / pgmq" {
            tags "Async"
        }
        supabaseCron -> supabasePostgres "Runs scheduled SQL for retention triggers and dead-man watchdog checks" "SQL"
        supabaseCron -> alertingChannel "Raises dead-man alerts when scheduler heartbeats stop, via pg_net" "HTTPS via pg_net"
        supabaseQueues -> portfolioService "Delivers versioned account, price, registry and job events to the portfolio consumer" "Supabase Queues (pgmq)" {
            tags "Async"
        }
        supabaseQueues -> reportingService "Delivers snapshot, valuation, recommendation and portfolio events to the reporting consumer" "Supabase Queues (pgmq)" {
            tags "Async"
        }
        supabaseQueues -> auditService "Delivers audit events, export parts and erasure confirmations to the audit consumer" "Supabase Queues (pgmq)" {
            tags "Async"
        }

        deploymentEnvironment "Production" {

            containerHost = deploymentNode "Container Host" "Autoscaling container platform hosting the SPA static assets and every Rust service as separate process groups, with managed TLS, health checks and rolling deploys" "Fly.io / Render / Cloud Run" {

                containerInstance reactSpa
                containerInstance apiGateway
                containerInstance identityService
                containerInstance accountService
                containerInstance assetRegistryService
                containerInstance marketDataService
                containerInstance portfolioService
                containerInstance reportingService
                containerInstance jobOrchestrationService
                containerInstance auditService

            }

        }

    }

    views {

        systemContext cpb "SystemContext" "System Context" {
            include *
            include notion
            autolayout
        }

        container cpb "Containers" "Containers" {
            include *
            include notion
            autolayout
        }

        component apiGateway "ApiGatewayComponents" "API Gateway Components" {
            include *
            autolayout
        }

        component identityService "IdentityComponents" "Identity Service Components" {
            include *
            autolayout
        }

        component accountService "AccountComponents" "Account Service Components" {
            include *
            autolayout
        }

        component assetRegistryService "AssetRegistryComponents" "Asset Registry Service Components" {
            include *
            autolayout
        }

        component marketDataService "MarketDataComponents" "Market Data Service Components" {
            include *
            autolayout
        }

        component portfolioService "PortfolioComponents" "Portfolio Service Components" {
            include *
            autolayout
        }

        component reportingService "ReportingComponents" "Reporting Service Components" {
            include *
            autolayout
        }

        component jobOrchestrationService "JobOrchestrationComponents" "Job Orchestration Service Components" {
            include *
            autolayout
        }

        component auditService "AuditComponents" "Audit Service Components" {
            include *
            autolayout
        }

        deployment cpb "Production" "ProductionDeployment" "Production Deployment" {
            include *
            autolayout
        }

        dynamic cpb "AccountSyncToValuation" "Account Sync to Valuation" {
            reactSpa -> apiGateway "Requests a bulk account sync and a short-lived stream ticket"
            apiGateway -> accountService "Routes the sync command with a signed principal assertion"
            accountService -> supabasePostgres "Replaces holdings atomically in the account schema"
            accountService -> supabaseQueues "Publishes versioned account.holdings.replaced"
            supabaseQueues -> portfolioService "Delivers the holdings replacement event"
            portfolioService -> supabasePostgres "Updates the holdings projection and computes the post-sync valuation"
            autolayout
        }

        dynamic apiGateway "AccountSyncProgress" "Account Sync Progress" {
            gatewayHttpRouter -> gatewayIssueStreamTicketUseCase "Issues the short-lived stream ticket for the progress stream"
            gatewayIssueStreamTicketUseCase -> gatewayStreamTicketSigner "Signs the ticket with the Supabase Vault-resolved key"
            gatewayAssertionSigner -> gatewayInternalPrincipalAssertion "Mints the signed principal assertion for the routed request"
            autolayout
        }

        dynamic portfolioService "AccountSyncProjection" "Account Sync Projection" {
            portfolioAccountEventConsumer -> portfolioApplyAccountEventUseCase "Applies versioned holdings and account lifecycle events"
            portfolioApplyAccountEventUseCase -> portfolioHoldingsProjection "Applies only a newer aggregate version"
            portfolioValuationHandlers -> portfolioRecomputeValuationUseCase "Requests the post-sync valuation"
            portfolioRecomputeValuationUseCase -> portfolioHoldingsProjection "Reads the projection after the write from Supabase Postgres"
            autolayout
        }

        dynamic cpb "RecommendationToReporting" "Recommendation to Reporting" {
            portfolioService -> supabasePostgres "Persists the generated recommendation"
            portfolioService -> supabaseQueues "Publishes recommendation.generated"
            supabaseQueues -> reportingService "Delivers the recommendation event"
            reportingService -> portfolioService "Fetches consistent brief data"
            reportingService -> openClawAgent "Requests narrative composition and Notion publication"
            openClawAgent -> notion "Publishes the brief to Notion"
            reportingService -> supabaseQueues "Publishes the delivery outcome"
            supabaseQueues -> auditService "Delivers the delivery audit event"
            auditService -> supabasePostgres "Appends the audit record"
            autolayout
        }

        styles {
            element "Person" {
                shape Person
                background "#08427b"
                color "#ffffff"
            }
            element "Software System" {
                background "#1168bd"
                color "#ffffff"
            }
            element "External" {
                background "#8c8c8c"
                color "#ffffff"
            }
            element "Supabase" {
                background "#3ecf8e"
                color "#000000"
            }
            element "Container" {
                background "#438dd5"
                color "#ffffff"
            }
            element "Database" {
                shape Cylinder
                background "#438dd5"
                color "#ffffff"
            }
            element "Queue" {
                shape Pipe
                background "#438dd5"
                color "#ffffff"
            }
            element "Cache" {
                shape Cylinder
                background "#6d4c41"
                color "#ffffff"
            }
            element "Bucket" {
                shape Cylinder
                background "#00695c"
                color "#ffffff"
            }
            element "Domain" {
                background "#2e7d32"
                color "#ffffff"
            }
            element "Application" {
                background "#1565c0"
                color "#ffffff"
            }
            element "Infrastructure" {
                background "#6a1b9a"
                color "#ffffff"
            }
            element "Transport" {
                background "#ef6c00"
                color "#ffffff"
            }
            relationship "Async" {
                dashed true
            }
        }

        theme default

    }

}
