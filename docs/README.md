# Documentation

Welcome to the Crypto Pocket Butler documentation. This directory contains all project documentation organized by topic.

## 📁 Documentation Structure

### 🚀 [setup/](./setup/)
Setup and configuration guides for getting started with the project.

- **[DOCKER_SETUP.md](./setup/DOCKER_SETUP.md)** - Docker Compose setup instructions
- **[FRONTEND_SETUP.md](./setup/FRONTEND_SETUP.md)** - Frontend development setup guide
- **[KEYCLOAK_SETUP.md](./setup/KEYCLOAK_SETUP.md)** - Keycloak authentication configuration
- **[SWAGGER_UI_GUIDE.md](./setup/SWAGGER_UI_GUIDE.md)** - API documentation interface guide

### 🏗️ [architecture/](./architecture/)
Architecture decisions and design documentation.

- **[NAMING_CONVENTION.md](./architecture/NAMING_CONVENTION.md)** - Project naming conventions
- **[TECHNICAL_DESIGN.md](./architecture/TECHNICAL_DESIGN.md)** - Overall technical design

### ⚙️ [backend/](./backend/)
Backend (Rust/Axum) specific documentation.

- **[backend-overview.md](./backend/backend-overview.md)** - Backend service overview
- **[CONCURRENCY.md](./backend/CONCURRENCY.md)** - Concurrency and performance patterns
- **[DATABASE_SCHEMA.md](./backend/DATABASE_SCHEMA.md)** - Database schema documentation
- **[OKX_IMPLEMENTATION.md](./backend/OKX_IMPLEMENTATION.md)** - OKX exchange integration
- **[ASSET_IDENTITY.md](./backend/ASSET_IDENTITY.md)** - Asset identity normalization
- **[JSON_SCHEMA.md](./backend/JSON_SCHEMA.md)** - Domain model JSON schemas
- **[jobs.md](./backend/jobs.md)** - Background job system
- **[connectors.md](./backend/connectors.md)** - External service connectors

### 🎨 [frontend/](./frontend/)
Frontend (Next.js/React) specific documentation.

- **[frontend-overview.md](./frontend/frontend-overview.md)** - Frontend service overview
- **[API_INTEGRATION.md](./frontend/API_INTEGRATION.md)** - API integration patterns
- **[UI-STYLE-GUIDE.md](./frontend/UI-STYLE-GUIDE.md)** - UI/UX design system
- **[UI-STANDARDIZATION.md](./frontend/UI-STANDARDIZATION.md)** - UI standardization guidelines
- **[UI-STANDARDIZATION-EXAMPLES.md](./frontend/UI-STANDARDIZATION-EXAMPLES.md)** - UI examples
- **[UI-COMPONENTS-VISUAL-REFERENCE.md](./frontend/UI-COMPONENTS-VISUAL-REFERENCE.md)** - Component reference
- **[components.md](./frontend/components.md)** - Component architecture
- **[hooks.md](./frontend/hooks.md)** - Custom React hooks
- **[portfolio-components.md](./frontend/portfolio-components.md)** - Portfolio-specific components
- **[snapshots.md](./frontend/snapshots.md)** - Portfolio snapshot features
- **[snapshots-implementation.md](./frontend/snapshots-implementation.md)** - Snapshot implementation details

### 🔨 [development/](./development/)
Development workflow and implementation summaries.

- **[CONSOLIDATION_SUMMARY.md](./development/CONSOLIDATION_SUMMARY.md)** - Frontend consolidation work
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

1. **New to the project?** Start with the main [README.md](../README.md) in the root directory
2. **Setting up for development?** Check [setup/DOCKER_SETUP.md](./setup/DOCKER_SETUP.md)
3. **Working on frontend?** See [frontend/frontend-overview.md](./frontend/frontend-overview.md)
4. **Working on backend?** See [backend/backend-overview.md](./backend/backend-overview.md)
5. **Understanding architecture?** Browse [architecture/](./architecture/)

## 📝 Contributing

When adding new documentation:
- Place setup guides in `setup/`
- Place architectural decisions in `architecture/`
- Place backend-specific docs in `backend/`
- Place frontend-specific docs in `frontend/`
- Place implementation summaries in `development/`
- Place planning documents in `planning/`
