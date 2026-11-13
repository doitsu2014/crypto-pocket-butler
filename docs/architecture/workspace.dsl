workspace "Crypto Pocket Butler" "Global retail crypto portfolio aggregation, P&L and decision support" {

    model {

        portfolioOwner = person "Portfolio Owner" "Authenticated retail investor who connects read-only exchange, on-chain and wallet sources, tracks P&L, builds portfolios, reviews valuation, drift and snapshots, and records orders and trades." "Person"
        administrator = person "Admin" "Holder of the Supabase `app_metadata.role` admin claim who manages registries, market-data sources, users, entitlements, jobs, audit and feature flags." "Person"
        supportAgent = person "Support Agent" "Read-only support user who investigates account metadata and tickets under time-bound, audited access; never sees credentials or portfolio contents." "Person"

        exchangeApis = softwareSystem "Exchange APIs (OKX, Binance, Coinbase, Kraken, Bybit)" "Read-only exchange APIs supplying balances and holdings, ticker prices, candle histories and instrument/pair metadata, with regional availability and API-key HMAC signing." "External"
        evmRpcProviders = softwareSystem "EVM JSON-RPC Providers" "Primary and fallback JSON-RPC endpoints per active EVM chain, with provider rotation and per-provider rate-limit budgets, supplying native and ERC-20 balances and token metadata." "External"
        solanaRpc = softwareSystem "Solana RPC Provider" "JSON-RPC provider supplying SOL and SPL token balances and mint metadata." "External"
        bitcoinTronRpc = softwareSystem "Bitcoin & Tron RPC / Indexers" "Bitcoin address and xpub indexing plus Tron JSON-RPC and TRC-20 balances, including USDT." "External"
        walletConnect = softwareSystem "WalletConnect Relay" "Read-only wallet session relay used by the SPA to connect supported wallets without exposing keys." "External"
        priceFxProviders = softwareSystem "Market Data & FX Providers" "Secondary exchange, aggregator and FX sources used for cross-checks, long-tail assets, historical rates and fallback." "External"
        benchmarkProvider = softwareSystem "Benchmark Index Provider" "Licensed benchmark series (BTC, ETH, broad crypto index) used for performance comparison." "External"
        emailProvider = softwareSystem "Transactional Email Provider" "Delivers authentication-adjacent, alert, brief and billing email under consent and unsubscribe rules." "External"
        pushProvider = softwareSystem "Web Push Provider" "Delivers consented web-push notifications to installed PWAs." "External"
        merchantOfRecord = softwareSystem "Merchant of Record" "Global subscription checkout, tax calculation and remittance, invoicing, dunning and refunds." "External"
        helpDesk = softwareSystem "Help Desk & Status Page" "Ticketing, knowledge base, status page and incident communication for support." "External"
        alertingChannel = softwareSystem "Operational Alerting Channel" "Pager/on-call alert consumer for missed runs, failed schedules, error-rate and stale-price alerts." "External"

        group "Supabase" {
            supabaseAuth = softwareSystem "Supabase Auth" "GoTrue identity provider: email/password, magic link, OAuth providers, MFA, passkeys, JWT issuance and refresh, the JWKS endpoint, a custom access-token hook that projects roles into `app_metadata.role`, signed webhooks for user lifecycle events, and the Admin API." "External,Supabase"
            supabasePostgres = softwareSystem "Supabase Postgres" "Managed Postgres with logical database-per-service isolation: one schema and one role per service (identity, account, registry, market, portfolio, ledger, reporting, notification, billing, analytics, job, audit), no cross-schema grants, and the Supavisor connection pooler." "External,Supabase"
            supabaseStorage = softwareSystem "Supabase Storage" "Private buckets for audit exports, retention archives, user data exports and generated PDF briefs, accessed only through signed, access-controlled URLs." "External,Supabase"
            supabaseVault = softwareSystem "Supabase Vault" "pgsodium-backed secrets store for exchange credentials, provider keys, the internal service token, merchant-of-record credentials, email and push provider keys, and per-subject encryption keys, with versioned encryption and no plaintext exposure to logs." "External,Supabase"
            prometheusAlerting = softwareSystem "Prometheus Alertmanager" "Evaluates the alert rules, including the Job Orchestration dead-man check on the heartbeat age (180 s, absent series included) and the job failure and abandonment counters." "External,Observability"
        }

        cpb = softwareSystem "Crypto Pocket Butler" "Global retail platform that aggregates read-only holdings across exchanges, chains and wallets, values portfolios in the user's base currency, computes P&L, drift and performance, records user-initiated order intents and trades, and delivers briefs, alerts, billing and support with privacy and regional controls." {

            reactSpa = container "React Web Client (SPA)" "Authenticated UI for onboarding, dashboard, accounts, portfolios, valuation, P&L, snapshots, orders, briefs, notifications, settings, billing and support. Signs in via Supabase Auth; calls only the API Gateway, plus WalletConnect for read-only wallet sessions." "React 18 + TypeScript 5, Vite, TanStack Query, @supabase/supabase-js, Recharts" "Container"

            supportConsole = container "Support Console" "Least-privilege support UI for account metadata and tickets, plus an administrator-only users section that looks a user up by exact email (address kept in the request body, never a URL) and changes role, disables or restores an account. Requires a recorded reason, is time-bound and audited, and never displays credentials or portfolio contents." "React 18 + TypeScript 5" "Container"

            apiGateway = container "API Gateway (BFF)" "Single public entry point: validates JWTs, resolves the principal, routes deny-by-default with admin and support gating, entitlement checks, rate limits and correlation IDs, and forwards the JWT unchanged. Stateless." "Rust, Axum, tower, jsonwebtoken, reqwest" "Container"

            identityService = container "Identity Service" "Internal users keyed by Supabase auth.users.id, roles, status, locale, base currency, timezone, consent state, support-access grants and their metadata fan-out with an audit outbox, deletion and retention. Holds no passwords or MFA state." "Rust, Axum, jsonwebtoken, sqlx" "Container"

            accountService = container "Account Service" "Read-only exchange, on-chain, WalletConnect, watch-only and CSV accounts; verified read-only credentials; holdings sync; spam and dust filtering; DeFi position labelling; regional coverage; quotas. Never accepts private keys." "Rust, Axum, sqlx, rust_decimal, alloy, solana-client" "Container"

            assetRegistryService = container "Asset Registry Service" "Canonical asset identity: unique symbols, one contract or mint per asset per chain, chains with primary and fallback RPCs, token registries, price-source registry and a quarantine for unmapped observations." "Rust, Axum, sqlx, rust_decimal" "Container"

            marketDataService = container "Market Data Service" "Point-in-time multi-source prices, candles, pair metadata, FX rates and benchmark series under a shared request budget. Deterministic source priority, stale-price fallback and licensing attribution." "Rust, Axum, sqlx, rust_decimal" "Container"

            portfolioService = container "Portfolio Service" "Portfolios and shared-account membership, targets and guardrails, base-currency valuation and drift, immutable snapshots, order intents and trades, performance and attribution. Unpriced and unmapped assets are labelled and excluded from valued totals." "Rust, Axum, sqlx, rust_decimal, tokio" "Container"

            ledgerService = container "Ledger Service" "Append-only transaction ledger, exchange and CSV imports, deduplication, transfer pairing, FIFO and average-cost basis, net invested, realized and unrealized P&L, and reconciliation against synced holdings." "Rust, Axum, sqlx, rust_decimal" "Container"

            reportingService = container "Reporting Service" "Reporting preferences and the scheduled daily and weekly brief pipeline, composed in-app and delivered as email and PDF. Idempotent per owner, portfolio and period." "Rust, Axum, sqlx" "Container"

            notificationService = container "Notification Service" "Alert catalogue, per-user and per-channel preferences, quiet hours, digests, dedupe, delivery log and bounded retries. Dispatches email and web push." "Rust, Axum, sqlx, tokio" "Container"

            billingService = container "Billing Service" "Plans, subscriptions, entitlements, quotas, invoices, dunning, refunds and merchant-of-record integration. Entitlements are enforced server-side." "Rust, Axum, sqlx, rust_decimal" "Container"

            analyticsService = container "Analytics Service" "Consent-aware product event ingestion, activation, retention and conversion aggregation, feature flags and experiment exposure. No PII or secrets." "Rust, Axum, sqlx" "Container"

            jobOrchestrationService = container "Job Orchestration Service (Scheduler / Job Runner)" "Self-ticking scheduler elected by a single OrchestratorLease row: runs the fixed C11 catalogue once per window with a per-window idempotency key, records one row per attempt, retries within a bound, recovers missed windows and operator backfills, and exports heartbeat and failure metrics." "Rust, Axum, sqlx, tokio" "Container"

            auditService = container "Audit Service" "Append-only audit store via an insert-only Postgres role. Admin queries, support-access records, privacy exports to Storage, erasure by crypto-shredding and partition retention." "Rust, Axum, sqlx" "Container"

            observabilityStack = container "Observability Stack" "Self-hosted logs (Loki), metrics (Prometheus) and traces (Tempo) with correlation context; Grafana dashboards and alert routing. No credentials or tokens are ever logged." "Prometheus, Loki, Grafana Tempo, OpenTelemetry" "Container"

            portfolioOwner -> reactSpa "Uses onboarding, dashboard, accounts, portfolios, P&L, orders, briefs, notifications, settings and billing" "HTTPS"
            administrator -> reactSpa "Administers registries, sources, users, entitlements, jobs, audit and flags" "HTTPS"
            supportAgent -> supportConsole "Investigates account metadata and tickets under audited access" "HTTPS"
            reactSpa -> apiGateway "Calls the REST API, entitlement-gated actions and the SSE sync-progress stream" "HTTPS/REST/JSON, Server-Sent Events"
            reactSpa -> supabaseAuth "Signs up, signs in with password, OAuth, magic link, MFA and passkeys, refreshes sessions with PKCE and signs out; the only direct browser-to-Supabase exception" "Supabase Auth over HTTPS"
            reactSpa -> walletConnect "Opens a read-only wallet session" "WalletConnect over HTTPS"
            reactSpa -> pushProvider "Registers for consented web push" "Web Push"
            supportConsole -> apiGateway "Requests read-only account metadata with a recorded reason" "HTTPS/REST/JSON"
            supportConsole -> helpDesk "Works tickets and views status" "HTTPS"

            apiGateway -> identityService "Resolves the internal principal, roles, preferences and consent state, which upserts from the validated JWT on first lookup" "Internal HTTP, forwarded Supabase JWT"
            apiGateway -> accountService "Routes account, sync and sync-status requests including the SSE progress stream" "Internal HTTP, forwarded Supabase JWT"
            apiGateway -> assetRegistryService "Routes asset catalogue and admin chain, token and source requests" "Internal HTTP, forwarded Supabase JWT"
            apiGateway -> marketDataService "Routes price, FX, benchmark and freshness requests" "Internal HTTP, forwarded Supabase JWT"
            apiGateway -> portfolioService "Routes portfolio, valuation, snapshot, order, performance and projection requests" "Internal HTTP, forwarded Supabase JWT"
            apiGateway -> ledgerService "Routes transaction, cost-basis and P&L requests" "Internal HTTP, forwarded Supabase JWT"
            apiGateway -> reportingService "Routes reporting preferences, brief history, generation log and generate-now" "Internal HTTP, forwarded Supabase JWT"
            apiGateway -> notificationService "Routes notification preferences, history and read state" "Internal HTTP, forwarded Supabase JWT"
            apiGateway -> billingService "Routes billing, plan and entitlement requests, and resolves entitlements for gated actions" "Internal HTTP, forwarded Supabase JWT"
            apiGateway -> analyticsService "Forwards consent-aware product events" "Internal HTTP, forwarded Supabase JWT"
            apiGateway -> jobOrchestrationService "Routes admin job listing, scheduling, triggering and run history" "Internal HTTP, forwarded Supabase JWT"
            apiGateway -> auditService "Routes admin audit queries, support-access records and user privacy export and erasure requests" "Internal HTTP, forwarded Supabase JWT"
            apiGateway -> supabaseAuth "Validates Supabase JWTs against the JWKS endpoint with an in-memory cache" "HTTPS JWKS"
            apiGateway -> auditService "Records access-denied and rate-limited security events synchronously" "Internal HTTP"
            apiGateway -> observabilityStack "Emits structured logs, metrics and traces with correlation context" "Prometheus scrape, OTLP export"

            identityService -> supabaseAuth "Verifies webhook signatures, validates JWTs against JWKS and calls the Admin API for sessions, app_metadata, MFA state and deletion" "HTTPS webhooks, JWKS and Admin API"
            identityService -> supabasePostgres "Reads and writes internal users, roles, status, preferences, consent and session events" "Supabase Postgres (schema identity, role identity)"
            identityService -> notificationService "Queues security, account and consent-change alerts" "Internal HTTP"
            identityService -> auditService "Records user lifecycle, role, status, support-access and consent audit events" "Internal HTTP"
            identityService -> accountService "Reads the allowlisted account projection for a support grant" "Internal HTTP, service token"
            identityService -> portfolioService "Reads the allowlisted portfolio projection for a support grant" "Internal HTTP, service token"
            identityService -> observabilityStack "Emits structured logs, metrics and traces with correlation context" "Prometheus scrape, OTLP export"

            accountService -> supabasePostgres "Reads and writes accounts, credential ciphertext, holdings, sync runs, spam state and the registry projection" "Supabase Postgres (schema account, role account)"
            accountService -> supabaseVault "Encrypts and decrypts read-only exchange credentials" "Supabase Vault (pgsodium)"
            accountService -> exchangeApis "Reads balances only with verified read-only API credentials" "HTTPS REST with HMAC request signing"
            accountService -> evmRpcProviders "Reads native and ERC-20 balances and token metadata with provider rotation" "JSON-RPC 2.0 over HTTPS"
            accountService -> solanaRpc "Reads SOL and SPL balances and mint metadata" "JSON-RPC over HTTPS"
            accountService -> bitcoinTronRpc "Reads Bitcoin address and xpub balances plus Tron and TRC-20 balances" "REST and JSON-RPC over HTTPS"
            accountService -> assetRegistryService "Fetches the registry snapshot on cold start or reconciliation and posts unmapped observations to quarantine" "Internal HTTP"
            accountService -> ledgerService "Publishes versioned holdings replacements for ledger reconciliation" "Internal HTTP"
            accountService -> portfolioService "Publishes versioned holdings replacements and account lifecycle changes after sync completion, with bounded retries" "Internal HTTP"
            accountService -> notificationService "Queues sync failures and key-revocation alerts" "Internal HTTP"
            accountService -> billingService "Checks account, coverage and sync-frequency quotas" "Internal HTTP"
            accountService -> analyticsService "Emits sync, coverage and account-type events" "Internal HTTP"
            accountService -> auditService "Records credential and account lifecycle audit events" "Internal HTTP"
            accountService -> identityService "Calls the C4 authorisation decision to verify role, status, and cross-owner linkage before storing a link" "Internal HTTP, service token + forwarded JWT"
            accountService -> observabilityStack "Emits structured logs, metrics and traces with correlation context" "Prometheus scrape, OTLP export"

            assetRegistryService -> supabasePostgres "Reads and writes assets, contracts, decimals, chains, token registries, sources and quarantine" "Supabase Postgres (schema registry, role registry)"
            assetRegistryService -> supabaseVault "Resolves RPC provider credentials" "Supabase Vault (pgsodium)"
            assetRegistryService -> marketDataService "Publishes catalogue and source-registry changes" "Internal HTTP"
            assetRegistryService -> auditService "Records registry and source change audit events" "Internal HTTP"
            assetRegistryService -> observabilityStack "Emits structured logs, metrics and traces with correlation context" "Prometheus scrape, OTLP export"

            marketDataService -> supabasePostgres "Reads and writes price, FX and benchmark observations, partitioned history, source mapping, licences and freshness state" "Supabase Postgres (schema market, role market)"
            marketDataService -> supabaseVault "Resolves exchange, provider and FX credentials" "Supabase Vault (pgsodium)"
            marketDataService -> assetRegistryService "Resolves tracked assets, source mappings and licence references" "Internal HTTP"
            marketDataService -> exchangeApis "Collects prices, candles and instrument/pair metadata within a shared request budget" "HTTPS REST/JSON"
            marketDataService -> priceFxProviders "Collects secondary prices, long-tail quotes and FX rates" "HTTPS REST/JSON"
            marketDataService -> benchmarkProvider "Collects licensed benchmark series" "HTTPS REST/JSON"
            marketDataService -> auditService "Records source, licence and manual-override audit events" "Internal HTTP"
            marketDataService -> observabilityStack "Emits structured logs, metrics and traces with correlation context" "Prometheus scrape, OTLP export"

            portfolioService -> supabasePostgres "Reads and writes portfolios, memberships, targets, guardrails, valuations, snapshots, orders, performance and projections" "Supabase Postgres (schema portfolio, role portfolio)"
            portfolioService -> accountService "Rebuilds the holdings projection and validates account ownership and active status at link time" "Internal HTTP"
            portfolioService -> assetRegistryService "Rebuilds the asset projection after cold start or a reconciliation gap" "Internal HTTP"
            portfolioService -> marketDataService "Rebuilds the latest price, FX and benchmark projections" "Internal HTTP"
            portfolioService -> ledgerService "Reads transactions, cost basis and P&L for performance and attribution" "Internal HTTP"
            portfolioService -> notificationService "Queues drift, guardrail and valuation alerts" "Internal HTTP"
            portfolioService -> billingService "Checks portfolio quotas and entitlements" "Internal HTTP"
            portfolioService -> analyticsService "Emits valuation, drift, allocation and activation events" "Internal HTTP"
            portfolioService -> auditService "Records order decision, snapshot and performance audit events" "Internal HTTP"
            portfolioService -> identityService "Calls the C4 authorisation decision to enforce role, status, and ownership on every read and write" "Internal HTTP, service token + forwarded JWT"
            portfolioService -> observabilityStack "Emits structured logs, metrics and traces with correlation context" "Prometheus scrape, OTLP export"

            ledgerService -> supabasePostgres "Appends transactions and adjustments, stores cost lots, P&L and reconciliation state" "Supabase Postgres (schema ledger, role ledger)"
            ledgerService -> accountService "Reconciles ledger quantity against synced holdings" "Internal HTTP"
            ledgerService -> marketDataService "Resolves prices and historical FX for cost basis and P&L" "Internal HTTP"
            ledgerService -> portfolioService "Publishes P&L, transaction and reconciliation updates" "Internal HTTP"
            ledgerService -> notificationService "Queues reconciliation and P&L coverage alerts" "Internal HTTP"
            ledgerService -> billingService "Checks transaction-import and history quotas" "Internal HTTP"
            ledgerService -> analyticsService "Emits import, coverage and P&L events" "Internal HTTP"
            ledgerService -> auditService "Records import, adjustment and cost-basis method audit events" "Internal HTTP"
            ledgerService -> observabilityStack "Emits structured logs, metrics and traces with correlation context" "Prometheus scrape, OTLP export"

            reportingService -> supabasePostgres "Reads and writes preferences, schedules, briefs, generation log and idempotency keys" "Supabase Postgres (schema reporting, role reporting)"
            reportingService -> portfolioService "Fetches a consistent single-owner brief data set once per brief" "Internal HTTP"
            reportingService -> ledgerService "Fetches P&L and transaction data for the brief" "Internal HTTP"
            reportingService -> notificationService "Queues brief-ready notifications" "Internal HTTP"
            reportingService -> supabaseStorage "Stores generated PDF briefs and exports" "Supabase Storage API"
            reportingService -> billingService "Checks export and reporting entitlements" "Internal HTTP"
            reportingService -> auditService "Records generation and retention audit events" "Internal HTTP"
            reportingService -> observabilityStack "Emits structured logs, metrics and traces with correlation context" "Prometheus scrape, OTLP export"

            notificationService -> supabasePostgres "Reads and writes the alert catalogue, preferences, quiet hours, delivery log and dedupe state" "Supabase Postgres (schema notification, role notification)"
            notificationService -> identityService "Resolves locale, timezone, contact and consent state" "Internal HTTP"
            notificationService -> emailProvider "Sends consented email" "HTTPS API"
            notificationService -> pushProvider "Sends consented web push" "HTTPS API"
            notificationService -> analyticsService "Emits delivery and engagement events" "Internal HTTP"
            notificationService -> auditService "Records delivery and preference-change audit events" "Internal HTTP"
            notificationService -> observabilityStack "Emits structured logs, metrics and traces with correlation context" "Prometheus scrape, OTLP export"

            billingService -> supabasePostgres "Reads and writes plans, subscriptions, entitlements, quotas, invoices and dunning state" "Supabase Postgres (schema billing, role billing)"
            billingService -> supabaseVault "Resolves merchant-of-record credentials" "Supabase Vault (pgsodium)"
            billingService -> merchantOfRecord "Creates checkout, subscriptions, invoices, dunning and refunds" "HTTPS API"
            billingService -> identityService "Resolves the user and billing contact" "Internal HTTP"
            billingService -> notificationService "Queues billing, receipt and payment-failure notices" "Internal HTTP"
            billingService -> analyticsService "Emits subscription, conversion and churn events" "Internal HTTP"
            billingService -> auditService "Records plan, entitlement and refund audit events" "Internal HTTP"
            billingService -> observabilityStack "Emits structured logs, metrics and traces with correlation context" "Prometheus scrape, OTLP export"

            analyticsService -> supabasePostgres "Reads and writes consent-aware events, aggregates, feature flags and experiment exposure" "Supabase Postgres (schema analytics, role analytics)"
            analyticsService -> identityService "Resolves consent state" "Internal HTTP"
            analyticsService -> auditService "Records flag and experiment-change audit events" "Internal HTTP"
            analyticsService -> observabilityStack "Emits structured logs, metrics and traces with correlation context" "Prometheus scrape, OTLP export"

            jobOrchestrationService -> supabasePostgres "Reads and writes job definitions, schedules, run records and heartbeats" "Supabase Postgres (schema job, role job)"
            jobOrchestrationService -> supabaseVault "Reads the shared internal service token used to authenticate run triggers" "Supabase Vault (pgsodium)"
            jobOrchestrationService -> identityService "Triggers scheduled retention and deletion runs" "Internal HTTP, service token"
            jobOrchestrationService -> accountService "Triggers scheduled account sync runs" "Internal HTTP, service token"
            jobOrchestrationService -> assetRegistryService "Triggers scheduled catalogue-sync and reconciliation runs" "Internal HTTP, service token"
            jobOrchestrationService -> marketDataService "Triggers scheduled price, FX and benchmark collection runs" "Internal HTTP, service token"
            jobOrchestrationService -> portfolioService "Triggers scheduled valuation, snapshot and performance runs" "Internal HTTP, service token"
            jobOrchestrationService -> ledgerService "Triggers scheduled transaction sync and reconciliation runs" "Internal HTTP, service token"
            jobOrchestrationService -> reportingService "Triggers scheduled brief runs" "Internal HTTP, service token"
            jobOrchestrationService -> notificationService "Triggers notification dispatch and digest runs" "Internal HTTP, service token"
            jobOrchestrationService -> billingService "Triggers billing renewal and dunning runs" "Internal HTTP, service token"
            jobOrchestrationService -> analyticsService "Triggers analytics aggregation and flag-expiry runs" "Internal HTTP, service token"
            jobOrchestrationService -> auditService "Triggers scheduled retention runs and records manual-trigger audit events" "Internal HTTP, service token"
            jobOrchestrationService -> alertingChannel "Sends missed-run, failed-run, error-rate, stale-price and stale-FX alerts" "Alert webhook"
            jobOrchestrationService -> observabilityStack "Emits structured logs, metrics and traces with correlation context" "Prometheus scrape, OTLP export"

            auditService -> supabasePostgres "Appends audit events through an insert-only role, applies archival retention through a separate privileged role, and queries for authorised admins" "Supabase Postgres (schema audit, role audit)"
            auditService -> supabaseStorage "Stores and retrieves export, brief and retention archives in private buckets" "Supabase Storage API"
            auditService -> supabaseVault "Reads and destroys per-subject crypto-shredding keys during erasure" "Supabase Vault (pgsodium)"
            auditService -> identityService "Resolves user profile and owner mapping for exports and anonymisation" "Internal HTTP"
            auditService -> observabilityStack "Emits structured logs, metrics and traces with correlation context" "Prometheus scrape, OTLP export"

        }

        jobOrchestrationService -> alertingChannel "Exports heartbeat age and failure counters for alerting" "HTTPS via /metrics scrape"

        deploymentEnvironment "Production" {

            swarmCluster = deploymentNode "Docker Swarm Cluster" "Swarm-mode cluster hosting the SPA static assets, the support console, every Rust service and the observability stack as replicated Swarm services behind the ingress routing mesh, with rolling updates, health checks, restart policies and placement across manager and worker nodes" "Docker Swarm (SwarmKit)" {

                containerInstance reactSpa
                containerInstance supportConsole
                containerInstance apiGateway
                containerInstance identityService
                containerInstance accountService
                containerInstance assetRegistryService
                containerInstance marketDataService
                containerInstance portfolioService
                containerInstance ledgerService
                containerInstance reportingService
                containerInstance notificationService
                containerInstance billingService
                containerInstance analyticsService
                containerInstance jobOrchestrationService
                containerInstance auditService
                containerInstance observabilityStack

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
            exclude exchangeApis evmRpcProviders solanaRpc bitcoinTronRpc walletConnect priceFxProviders benchmarkProvider emailProvider pushProvider merchantOfRecord helpDesk alertingChannel supabaseAuth supabasePostgres supabaseStorage supabaseVault
            autolayout
        }

        deployment cpb "Production" "ProductionDeployment" "Production Deployment" {
            include *
            autolayout
        }

        dynamic cpb "AccountSyncToValuation" "Account Sync to Valuation" {
            reactSpa -> apiGateway "Requests a bulk account sync and subscribes to SSE progress"
            apiGateway -> accountService "Routes the sync command with the forwarded JWT"
            accountService -> exchangeApis "Reads balances with a verified read-only key"
            accountService -> evmRpcProviders "Reads native and ERC-20 balances for on-chain accounts"
            accountService -> solanaRpc "Reads SOL and SPL balances for on-chain accounts"
            accountService -> bitcoinTronRpc "Reads Bitcoin address, xpub and Tron TRC-20 balances for on-chain accounts"
            accountService -> supabasePostgres "Replaces holdings atomically in the account schema"
            accountService -> ledgerService "Publishes the versioned holdings replacement for reconciliation"
            accountService -> portfolioService "Publishes the versioned holdings replacement"
            portfolioService -> marketDataService "Resolves latest prices and FX"
            portfolioService -> supabasePostgres "Updates the holdings projection and computes the post-sync valuation"
            autolayout
        }

        dynamic cpb "TransactionToPnl" "Transaction Import to P&L" {
            reactSpa -> apiGateway "Imports transactions or opens P&L"
            apiGateway -> ledgerService "Routes the import and P&L request with the forwarded JWT"
            ledgerService -> accountService "Reconciles ledger quantity against synced holdings"
            ledgerService -> marketDataService "Resolves prices and historical FX"
            ledgerService -> supabasePostgres "Appends transactions and computes cost basis"
            ledgerService -> portfolioService "Publishes P&L and reconciliation updates"
            portfolioService -> supabasePostgres "Updates the portfolio P&L projection"
            autolayout
        }

        dynamic cpb "DriftAlertToNotification" "Drift Alert to Notification" {
            jobOrchestrationService -> portfolioService "Triggers the scheduled valuation run"
            portfolioService -> marketDataService "Resolves latest prices and FX"
            portfolioService -> supabasePostgres "Persists the valuation and drift"
            portfolioService -> notificationService "Queues a drift or guardrail alert"
            notificationService -> supabasePostgres "Records the alert and delivery state"
            notificationService -> emailProvider "Sends consented email"
            notificationService -> pushProvider "Sends consented web push"
            notificationService -> auditService "Records the delivery outcome"
            autolayout
        }

        dynamic cpb "OnboardingToFirstValue" "Onboarding to First Value" {
            portfolioOwner -> reactSpa "Starts onboarding and chooses the address path"
            reactSpa -> apiGateway "Adds a public address"
            apiGateway -> accountService "Creates the account with the forwarded JWT"
            accountService -> evmRpcProviders "Reads native and token balances"
            accountService -> supabasePostgres "Stores holdings and sync status"
            accountService -> portfolioService "Publishes the first holdings replacement"
            portfolioService -> marketDataService "Resolves prices and FX"
            portfolioService -> supabasePostgres "Computes the first valuation and allocation"
            reactSpa -> apiGateway "Views the first valuation"
            apiGateway -> portfolioService "Returns valuation, allocation and coverage"
            autolayout
        }

        dynamic cpb "BillingToEntitlement" "Billing to Entitlement" {
            portfolioOwner -> reactSpa "Starts checkout or changes plan"
            reactSpa -> apiGateway "Requests checkout or a gated action"
            apiGateway -> billingService "Resolves entitlement or starts checkout"
            billingService -> merchantOfRecord "Creates checkout or subscription"
            billingService -> supabasePostgres "Persists subscription and entitlement state"
            billingService -> notificationService "Queues the receipt or payment notice"
            notificationService -> emailProvider "Sends the billing email"
            apiGateway -> accountService "Forwards a gated action after the entitlement check"
            autolayout
        }

        dynamic cpb "SupportAccess" "Support Access" {
            supportAgent -> supportConsole "Opens a ticket and requests account metadata"
            supportConsole -> apiGateway "Requests a grant, an admin approves or revokes it, or the holder reads metadata under a live grant; an administrator also looks a user up by exact email and changes role, disables or restores an account, each with a reason and audited"
            apiGateway -> identityService "Applies role, status and grant rules and resolves the closed metadata projection"
            identityService -> accountService "Reads the allowlisted account projection"
            identityService -> portfolioService "Reads the allowlisted portfolio projection"
            identityService -> supabasePostgres "Persists the grant and its audit outbox row"
            identityService -> auditService "Delivers support-access audit events from the outbox"
            supportConsole -> helpDesk "Works the ticket"
            autolayout
        }

        dynamic cpb "OrderToReporting" "Order to Reporting" {
            reactSpa -> apiGateway "Creates a buy or sell order intent from the portfolio view"
            apiGateway -> portfolioService "Routes the order command with the forwarded JWT"
            portfolioService -> ledgerService "Records the trade against the shared account"
            ledgerService -> supabasePostgres "Appends the trade transaction"
            portfolioService -> supabasePostgres "Persists the order and updates allocation immediately"
            jobOrchestrationService -> reportingService "Triggers the scheduled brief run"
            reportingService -> portfolioService "Fetches the consistent single-owner brief data set"
            reportingService -> ledgerService "Fetches P&L and transaction data"
            reportingService -> notificationService "Queues the brief-ready notification"
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
