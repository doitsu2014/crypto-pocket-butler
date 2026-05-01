# Documentation

Welcome to the Crypto Pocket Butler documentation. This directory contains all project documentation organized by topic.

## 📁 Documentation Structure

### 🚀 [setup/](./setup/)
Setup and configuration guides for getting started with the project.

- **[DOCKER_SETUP.md](./setup/DOCKER_SETUP.md)** - Docker Compose setup instructions
- **[WEB_SETUP.md](./setup/WEB_SETUP.md)** - Web development setup guide
- **[KEYCLOAK_SETUP.md](./setup/KEYCLOAK_SETUP.md)** - Keycloak authentication configuration
- **[SWAGGER_UI_GUIDE.md](./setup/SWAGGER_UI_GUIDE.md)** - API documentation interface guide

### 🏗️ [architecture/](./architecture/)
System architecture, design decisions, and service topology.

- **[ARCHITECTURE.md](./architecture/ARCHITECTURE.md)** ⭐ - **Comprehensive architecture reference** (topology, tech stack, DB schema, request flows, jobs, connectors)
- **[NAMING_CONVENTION.md](./architecture/NAMING_CONVENTION.md)** - Project naming conventions
- **[TECHNICAL_DESIGN.md](./architecture/TECHNICAL_DESIGN.md)** - Overall technical design

### 📐 [coding-guidelines/](./coding-guidelines/)
Project-specific coding conventions for Rust and TypeScript/React.

- **[CODING_GUIDELINES.md](./coding-guidelines/CODING_GUIDELINES.md)** ⭐ - **Comprehensive coding guidelines** (Rust patterns, TypeScript patterns, UI design rules, API conventions, migration rules)

### 📋 [use-cases/](./use-cases/)
End-to-end workflows for all user roles and system automation.

- **[USE_CASES.md](./use-cases/USE_CASES.md)** ⭐ - **Complete use case & workflow reference** (authentication, account management, portfolio workflows, snapshots, rebalancing, admin, background jobs)

### ⚙️ [api/](./api/)
API (Rust/Axum) specific documentation.

- **[api-overview.md](./api/api-overview.md)** - API service overview
- **[CONCURRENCY.md](./api/CONCURRENCY.md)** - Concurrency and performance patterns
- **[DATABASE_SCHEMA.md](./api/DATABASE_SCHEMA.md)** - Database schema documentation
- **[OKX_IMPLEMENTATION.md](./api/OKX_IMPLEMENTATION.md)** - OKX exchange integration
- **[ASSET_IDENTITY.md](./api/ASSET_IDENTITY.md)** - Asset identity normalization
- **[JSON_SCHEMA.md](./api/JSON_SCHEMA.md)** - Domain model JSON schemas
- **[jobs.md](./api/jobs.md)** - Background job system
- **[connectors.md](./api/connectors.md)** - External service connectors

### 🎨 [web/](./web/)
Web (Next.js/React) specific documentation.

- **[web-overview.md](./web/web-overview.md)** - Web service overview
- **[API_INTEGRATION.md](./web/API_INTEGRATION.md)** - API integration patterns
- **[UI-STYLE-GUIDE.md](./web/UI-STYLE-GUIDE.md)** - UI/UX design system
- **[UI-STANDARDIZATION.md](./web/UI-STANDARDIZATION.md)** - UI standardization guidelines
- **[UI-STANDARDIZATION-EXAMPLES.md](./web/UI-STANDARDIZATION-EXAMPLES.md)** - UI examples
- **[UI-COMPONENTS-VISUAL-REFERENCE.md](./web/UI-COMPONENTS-VISUAL-REFERENCE.md)** - Component reference
- **[components.md](./web/components.md)** - Component architecture
- **[hooks.md](./web/hooks.md)** - Custom React hooks
- **[portfolio-components.md](./web/portfolio-components.md)** - Portfolio-specific components
- **[snapshots.md](./web/snapshots.md)** - Portfolio snapshot features
- **[snapshots-implementation.md](./web/snapshots-implementation.md)** - Snapshot implementation details

### 🔨 [development/](./development/)
Development workflow and implementation summaries.

- **[CONSOLIDATION_SUMMARY.md](./development/CONSOLIDATION_SUMMARY.md)** - Web consolidation work
- **[IMPLEMENTATION_SUMMARY.md](./development/IMPLEMENTATION_SUMMARY.md)** - Job runner implementation
- **[IMPLEMENTATION-SUMMARY.md](./development/IMPLEMENTATION-SUMMARY.md)** - General implementation notes
- **[PORTFOLIO_SETTINGS_IMPLEMENTATION.md](./development/PORTFOLIO_SETTINGS_IMPLEMENTATION.md)** - Portfolio settings feature
- **[SECURITY_SUMMARY.md](./development/SECURITY_SUMMARY.md)** - Security analysis and summaries

### 📋 [planning/](./planning/)
Project planning documents and roadmaps.

- **[README.md](./planning/README.md)** - Planning overview
- **[00-project-overview.md](./planning/00-project-overview.md)** - Project overview
- **[01-mvp-scope.md](./planning/01-mvp-scope.md)** - MVP scope definition
- **[02-data-sources.md](./planning/02-data-sources.md)** - Data source integration plans
- **[03-portfolio-model.md](./planning/03-portfolio-model.md)** - Portfolio model design
- **[04-rebalancing-risk.md](./planning/04-rebalancing-risk.md)** - Rebalancing risk considerations
- **[05-openclaw-agent.md](./planning/05-openclaw-agent.md)** - OpenClaw agent design
- **[06-security-audit.md](./planning/06-security-audit.md)** - Security audit planning
- **[07-notion-reporting.md](./planning/07-notion-reporting.md)** - Notion integration plans
- **[08-roadmap.md](./planning/08-roadmap.md)** - Project roadmap
- **[09-technical-design.md](./planning/09-technical-design.md)** - Technical design details
- **[10-core-split-infra-vs-portfolios.md](./planning/10-core-split-infra-vs-portfolios.md)** - Infrastructure architecture

## 🚦 Quick Start

1. **New to the project?** Start with [architecture/ARCHITECTURE.md](./architecture/ARCHITECTURE.md) for the full system overview
2. **Setting up for development?** Check [setup/DOCKER_SETUP.md](./setup/DOCKER_SETUP.md)
3. **Writing code?** Read [coding-guidelines/CODING_GUIDELINES.md](./coding-guidelines/CODING_GUIDELINES.md) first
4. **Understanding user workflows?** See [use-cases/USE_CASES.md](./use-cases/USE_CASES.md)
5. **Working on web?** See [web/web-overview.md](./web/web-overview.md)
6. **Working on api?** See [api/api-overview.md](./api/api-overview.md)

## 📝 Contributing

When adding new documentation:
- Place setup guides in `setup/`
- Place architectural decisions and system design in `architecture/`
- Place coding standards and patterns in `coding-guidelines/`
- Place use case and workflow documentation in `use-cases/`
- Place API-specific docs in `api/`
- Place web-specific docs in `web/`
- Place implementation summaries in `development/`
- Place planning documents in `planning/`
