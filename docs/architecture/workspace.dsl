workspace "Crypto Pocket Butler" "Read-only crypto portfolio aggregation and decision support" {

    model {

        portfolioOwner = person "Portfolio Owner" "Authenticated retail investor who connects read-only exchange and wallet accounts, builds portfolios, reviews valuation, drift, snapshots and recommendations." "Person"
        administrator = person "Admin" "Holder of the Supabase `app_metadata.role` admin claim who manages the chain and token registry, job schedules, audit queries and privacy requests." "Person"

        okxExchange = softwareSystem "OKX Exchange API" "Read-only exchange API supplying balances and holdings, ticker prices, candle histories and instrument/pair metadata with API-key, secret and passphrase HMAC signing." "External"
        evmRpcProviders = softwareSystem "EVM JSON-RPC Providers" "Primary and fallback JSON-RPC endpoints per active EVM chain, with provider rotation and per-provider rate-limit budgets, supplying native and ERC-20 balances and token metadata." "External"
        solanaRpc = softwareSystem "Solana RPC Provider" "JSON-RPC provider supplying SOL and SPL token balances and mint metadata." "External"
        alertingChannel = softwareSystem "Operational Alerting Channel" "Pager/on-call alert consumer for missed runs, failed schedules, error-rate and stale-price alerts." "External"

        group "Supabase" {
            supabaseAuth = softwareSystem "Supabase Auth" "GoTrue identity provider: email/password, magic link, OAuth providers, MFA, JWT issuance and refresh, the JWKS endpoint, a custom access-token hook that projects roles into `app_metadata.role`, signed webhooks for user lifecycle events, and the Admin API." "External,Supabase"
            supabasePostgres = softwareSystem "Supabase Postgres" "Managed Postgres with logical database-per-service isolation: one schema and one role per service (identity, account, registry, market, portfolio, reporting, job, audit), no cross-schema grants, and the Supavisor connection pooler." "External,Supabase"
            supabaseStorage = softwareSystem "Supabase Storage" "Private buckets for audit export and retention archives, accessed only through signed, access-controlled URLs." "External,Supabase"
            supabaseVault = softwareSystem "Supabase Vault" "pgsodium-backed secrets store for exchange credentials, provider keys, the internal service token and per-subject encryption keys, with versioned encryption and no plaintext exposure to logs." "External,Supabase"
            supabaseCron = softwareSystem "Supabase Cron (pg_cron)" "Scheduled SQL that ticks the Job Orchestration Service through pg_net, runs retention SQL, and hosts the dead-man check that alerts when scheduler heartbeats stop." "External,Supabase"
            supabaseLogs = softwareSystem "Supabase Logs & Metrics" "Platform logs, metrics, log drains and alerts for every service, with correlation context." "External,Supabase"
        }

        cpb = softwareSystem "Crypto Pocket Butler" "Single-tenant MVP platform that aggregates read-only holdings, values portfolios in USD, computes drift, proposes human-reviewed recommendations and publishes scheduled briefs." {

            reactSpa = container "React Web Client (SPA)" "Authenticated UI for dashboard, accounts, portfolios, valuation, snapshots, recommendations, briefs and admin screens. Signs in through Supabase Auth and calls only the API Gateway." "React 18 + TypeScript 5, Vite, TanStack Query, @supabase/supabase-js, Recharts" "Container"

            apiGateway = container "API Gateway (BFF)" "Single public entry point: validates Supabase JWTs, resolves the principal via Identity, applies deny-by-default routing, admin-claim gating, rate limits and correlation IDs, and forwards the user's JWT unchanged to the owning service. Stateless." "Rust, Axum, tower, jsonwebtoken, reqwest" "Container"

            identityService = container "Identity Service" "Internal users keyed by Supabase auth.users.id, created from signed Auth webhooks or lazily on first lookup. Owns roles from `app_metadata.role`, user status, session events, deletion/anonymisation and retention. Holds no passwords or MFA state." "Rust, Axum, jsonwebtoken, sqlx" "Container"

            accountService = container "Account Service" "Read-only exchange and wallet accounts and the holdings sync pipeline. Credentials encrypted in Supabase Vault; never accepts private keys or seed phrases. Notifies Portfolio of versioned holdings after each sync." "Rust, Axum, sqlx, rust_decimal, alloy, solana-client" "Container"

            assetRegistryService = container "Asset Registry Service" "Single authority for canonical asset identity: unique symbols, one contract or mint per asset per chain, chains with primary/fallback RPCs, token registries and the quarantine for unmapped observations." "Rust, Axum, sqlx, rust_decimal" "Container"

            marketDataService = container "Market Data Service" "Point-in-time USD prices, candle histories and pair metadata from OKX tickers under a shared request budget. Tracks freshness and marks assets unpriced past a per-asset-class maximum stale age." "Rust, Axum, sqlx, rust_decimal" "Container"

            portfolioService = container "Portfolio Service" "Portfolios and targets, decimal valuation and drift, immutable snapshots, and advisory recommendations with human accept/reject decisions. Unpriced and unmapped assets are labelled and excluded from valued totals." "Rust, Axum, sqlx, rust_decimal, tokio" "Container"

            reportingService = container "Reporting Service" "Reporting preferences and the scheduled daily/weekly brief pipeline, composed in-app and served to the SPA. Idempotent per owner, portfolio and period." "Rust, Axum, sqlx" "Container"

            jobOrchestrationService = container "Job Orchestration Service (Scheduler / Job Runner)" "Scheduler for non-interactive work: pg_cron ticks it every minute, it triggers each service's run endpoint single-flight under advisory locks, records outcomes, detects missed runs and raises alerts within 15 minutes." "Rust, Axum, sqlx, tokio" "Container"

            auditService = container "Audit Service" "Append-only audit store written through an insert-only Postgres role. Admin queries, privacy exports to Supabase Storage, erasure by crypto-shredding and partition retention." "Rust, Axum, sqlx" "Container"

            portfolioOwner -> reactSpa "Uses the dashboard, accounts, portfolios, valuation, recommendations and reporting screens" "HTTPS"
            administrator -> reactSpa "Administers chains, tokens, jobs, audit and privacy requests" "HTTPS"
            reactSpa -> apiGateway "Calls the REST API and the SSE sync-progress stream" "HTTPS/REST/JSON, Server-Sent Events"
            reactSpa -> supabaseAuth "Signs up, signs in with password, OAuth, magic link and MFA, refreshes sessions with PKCE and signs out; the only direct browser-to-Supabase exception" "Supabase Auth over HTTPS"

            apiGateway -> identityService "Resolves the internal principal (owner UUID, status, roles), which upserts from the validated JWT on first lookup" "Internal HTTP, forwarded Supabase JWT"
            apiGateway -> accountService "Routes account, sync and sync-status requests including the SSE progress stream" "Internal HTTP, forwarded Supabase JWT"
            apiGateway -> assetRegistryService "Routes asset catalogue and admin chain and token requests" "Internal HTTP, forwarded Supabase JWT"
            apiGateway -> marketDataService "Routes price and freshness requests" "Internal HTTP, forwarded Supabase JWT"
            apiGateway -> portfolioService "Routes portfolio, valuation, snapshot and recommendation requests" "Internal HTTP, forwarded Supabase JWT"
            apiGateway -> reportingService "Routes reporting preferences, brief history, generation log and generate-now" "Internal HTTP, forwarded Supabase JWT"
            apiGateway -> jobOrchestrationService "Routes admin job listing, scheduling, triggering and run history" "Internal HTTP, forwarded Supabase JWT"
            apiGateway -> auditService "Routes admin audit queries and user privacy export and erasure requests" "Internal HTTP, forwarded Supabase JWT"
            apiGateway -> supabaseAuth "Validates Supabase JWTs against the JWKS endpoint with an in-memory cache" "HTTPS JWKS"
            apiGateway -> auditService "Records access-denied and rate-limited security events synchronously" "Internal HTTP"
            apiGateway -> supabaseLogs "Logs, metrics and traces via log drain" "Log drain"

            identityService -> supabaseAuth "Verifies webhook signatures, validates JWTs against JWKS and calls the Admin API for sessions, app_metadata and deletion" "HTTPS webhooks, JWKS and Admin API"
            identityService -> supabasePostgres "Reads and writes internal users, roles, status and session events" "Supabase Postgres (schema identity, role identity)"
            identityService -> auditService "Records user lifecycle, role and status audit events" "Internal HTTP"
            identityService -> supabaseLogs "Logs, metrics and traces via log drain" "Log drain"

            accountService -> supabasePostgres "Reads and writes accounts, credential ciphertext, holdings, sync runs and the registry projection" "Supabase Postgres (schema account, role account)"
            accountService -> supabaseVault "Encrypts and decrypts read-only exchange credentials" "Supabase Vault (pgsodium)"
            accountService -> okxExchange "Reads balances only with read-only API credentials" "HTTPS REST with HMAC request signing"
            accountService -> evmRpcProviders "Reads native and ERC-20 balances and token metadata with provider rotation" "JSON-RPC 2.0 over HTTPS"
            accountService -> solanaRpc "Reads SOL and SPL balances and mint metadata" "JSON-RPC over HTTPS"
            accountService -> assetRegistryService "Fetches the registry snapshot on cold start or reconciliation and posts unmapped observations to quarantine" "Internal HTTP"
            accountService -> portfolioService "Notifies versioned holdings replacements and account lifecycle changes after sync completion, with bounded retries" "Internal HTTP"
            accountService -> auditService "Records credential and account lifecycle audit events" "Internal HTTP"
            accountService -> supabaseLogs "Logs, metrics and traces via log drain" "Log drain"

            assetRegistryService -> supabasePostgres "Reads and writes assets, contracts, decimals, chains, token registries and quarantine" "Supabase Postgres (schema registry, role registry)"
            assetRegistryService -> supabaseVault "Resolves RPC provider credentials" "Supabase Vault (pgsodium)"
            assetRegistryService -> auditService "Records registry change audit events" "Internal HTTP"
            assetRegistryService -> supabaseLogs "Logs, metrics and traces via log drain" "Log drain"

            marketDataService -> supabasePostgres "Reads and writes price observations, partitioned history, provider mapping and freshness state" "Supabase Postgres (schema market, role market)"
            marketDataService -> supabaseVault "Resolves exchange API credentials" "Supabase Vault (pgsodium)"
            marketDataService -> okxExchange "Collects USD ticker observations, candle histories and instrument/pair metadata within a shared request budget" "HTTPS REST/JSON"
            marketDataService -> supabaseLogs "Logs, metrics and traces via log drain" "Log drain"

            portfolioService -> supabasePostgres "Reads and writes portfolios, targets, guardrails, valuations, snapshots, recommendations and projections" "Supabase Postgres (schema portfolio, role portfolio)"
            portfolioService -> accountService "Rebuilds the holdings projection and validates account ownership and active status at link time" "Internal HTTP"
            portfolioService -> assetRegistryService "Rebuilds the asset projection after cold start or a reconciliation gap" "Internal HTTP"
            portfolioService -> marketDataService "Rebuilds the latest-price projection after cold start or schema change" "Internal HTTP"
            portfolioService -> auditService "Records recommendation decision and snapshot audit events" "Internal HTTP"
            portfolioService -> supabaseLogs "Logs, metrics and traces via log drain" "Log drain"

            reportingService -> supabasePostgres "Reads and writes preferences, schedules, briefs, generation log and idempotency keys" "Supabase Postgres (schema reporting, role reporting)"
            reportingService -> portfolioService "Fetches a consistent single-owner brief data set once per brief" "Internal HTTP"
            reportingService -> auditService "Records generation and retention audit events" "Internal HTTP"
            reportingService -> supabaseLogs "Logs, metrics and traces via log drain" "Log drain"

            jobOrchestrationService -> supabasePostgres "Reads and writes job definitions, schedules, run records and heartbeats" "Supabase Postgres (schema job, role job)"
            jobOrchestrationService -> supabaseVault "Reads the shared internal service token used to authenticate run triggers" "Supabase Vault (pgsodium)"
            jobOrchestrationService -> identityService "Triggers scheduled retention runs" "Internal HTTP, service token"
            jobOrchestrationService -> accountService "Triggers scheduled sync runs" "Internal HTTP, service token"
            jobOrchestrationService -> assetRegistryService "Triggers scheduled catalogue-sync and reconciliation runs" "Internal HTTP, service token"
            jobOrchestrationService -> marketDataService "Triggers scheduled price-collection runs" "Internal HTTP, service token"
            jobOrchestrationService -> portfolioService "Triggers scheduled valuation, snapshot, recommendation and reconciliation runs" "Internal HTTP, service token"
            jobOrchestrationService -> reportingService "Triggers scheduled brief runs" "Internal HTTP, service token"
            jobOrchestrationService -> auditService "Triggers scheduled retention runs and records manual-trigger audit events" "Internal HTTP, service token"
            jobOrchestrationService -> alertingChannel "Sends missed-run, failed-run, error-rate and stale-price alerts" "Alert webhook"
            jobOrchestrationService -> supabaseLogs "Logs, metrics and traces via log drain" "Log drain"

            auditService -> supabasePostgres "Appends audit events through an insert-only role, applies archival retention through a separate privileged role, and queries for authorised admins" "Supabase Postgres (schema audit, role audit)"
            auditService -> supabaseStorage "Stores and retrieves export and retention archives in private buckets" "Supabase Storage API"
            auditService -> supabaseVault "Reads and destroys per-subject crypto-shredding keys during erasure" "Supabase Vault (pgsodium)"
            auditService -> identityService "Resolves user profile and owner mapping for exports and anonymisation" "Internal HTTP"
            auditService -> supabaseLogs "Logs, metrics and traces via log drain" "Log drain"

        }

        portfolioOwner -> cpb "Uses read-only portfolio aggregation, valuation, recommendations and briefs" "HTTPS"
        administrator -> cpb "Administers chains, tokens, jobs, audit and privacy operations" "HTTPS"
        cpb -> supabaseAuth "Delegates authentication, MFA and session management" "Supabase Auth over HTTPS"
        cpb -> okxExchange "Reads balances from connected read-only exchange accounts and collects ticker, candle and instrument market data" "HTTPS REST with HMAC request signing"
        cpb -> evmRpcProviders "Reads native and ERC-20 balances on configured EVM chains" "JSON-RPC 2.0 over HTTPS"
        cpb -> solanaRpc "Reads SOL and SPL balances for registered wallets" "JSON-RPC over HTTPS"
        cpb -> supabasePostgres "Stores every service's data in its own schema and role" "SQL over TLS via Supavisor"
        cpb -> supabaseStorage "Stores audit export and retention archives" "Supabase Storage API"
        cpb -> supabaseVault "Encrypts credentials and resolves scoped secret references" "Supabase Vault (pgsodium)"
        cpb -> supabaseLogs "Emits structured logs, metrics and traces with correlation context" "Log drain"
        cpb -> alertingChannel "Routes missed-run, failed-schedule, error-rate and stale-price alerts" "Alert webhook"
        supabaseCron -> jobOrchestrationService "Ticks the scheduler every minute" "HTTPS via pg_net"
        supabaseCron -> supabasePostgres "Runs scheduled retention SQL" "SQL"
        supabaseCron -> alertingChannel "Raises dead-man alerts when scheduler heartbeats stop, via pg_net" "HTTPS via pg_net"

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
            autolayout
        }

        container cpb "Containers" "Containers" {
            include *
            autolayout
        }

        deployment cpb "Production" "ProductionDeployment" "Production Deployment" {
            include *
            autolayout
        }

        dynamic cpb "AccountSyncToValuation" "Account Sync to Valuation" {
            reactSpa -> apiGateway "Requests a bulk account sync and subscribes to SSE progress"
            apiGateway -> accountService "Routes the sync command with the forwarded JWT"
            accountService -> supabasePostgres "Replaces holdings atomically in the account schema"
            accountService -> portfolioService "Notifies the versioned holdings replacement"
            portfolioService -> supabasePostgres "Updates the holdings projection and computes the post-sync valuation"
            autolayout
        }

        dynamic cpb "RecommendationToReporting" "Recommendation to Reporting" {
            jobOrchestrationService -> portfolioService "Triggers the scheduled recommendation run"
            portfolioService -> supabasePostgres "Persists the generated recommendation"
            jobOrchestrationService -> reportingService "Triggers the scheduled brief run"
            reportingService -> portfolioService "Fetches the consistent single-owner brief data set"
            reportingService -> supabasePostgres "Persists the composed brief in the reporting schema"
            reportingService -> auditService "Records the brief generation audit event"
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
        }

        theme default

    }

}
