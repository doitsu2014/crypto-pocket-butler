# Naming Convention

This document describes the naming conventions used throughout the Crypto Pocket Butler project.

## Service Folder Names

The project follows a clear naming convention for service directories:

### API Service
- **Folder Name**: `api`
- **Technology**: Rust (Axum framework)
- **Location**: `/api`
- **Docker Service Name**: `api`

### Web Service
- **Folder Name**: `web`
- **Technology**: React (Next.js framework)
- **Location**: `/web`
- **Docker Service Name**: `web`

## Rationale

The folder names use simple, functional names (`api`, `web`) to:
1. Keep naming concise and immediately understandable
2. Focus on the service role rather than technology stack
3. Provide clarity in monorepo structure
4. Use industry-standard naming conventions

## Docker Compose Services

In `docker-compose.yml`, services use the same descriptive names:
- `api` → builds from `./api`
- `web` → builds from `./web`

This provides consistency across all references to these services.

## References in Documentation

All documentation files (README.md, DOCKER_SETUP.md, etc.) consistently reference:
- `api/` for API-related content
- `web/` for web interface-related content

## Container Names

Docker containers follow the pattern: `crypto-pocket-butler-{service}`
- `crypto-pocket-butler-api`
- `crypto-pocket-butler-web`
- `crypto-pocket-butler-db`
- `crypto-pocket-butler-keycloak`
