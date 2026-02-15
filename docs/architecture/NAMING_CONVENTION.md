# Naming Convention

This document describes the naming conventions used throughout the Crypto Pocket Butler project.

## Service Folder Names

The project follows a clear naming convention for service directories:

### Backend Service
- **Folder Name**: `api`
- **Technology**: Rust (Axum framework)
- **Location**: `/api`
- **Docker Service Name**: `backend`

### Frontend Service
- **Folder Name**: `web`
- **Technology**: React (Next.js framework)
- **Location**: `/web`
- **Docker Service Name**: `frontend`

## Rationale

The folder names use simple, functional names (`api`, `web`) to:
1. Keep naming concise and immediately understandable
2. Focus on the service role rather than technology stack
3. Provide clarity in monorepo structure
4. Use industry-standard naming conventions

## Docker Compose Services

In `docker-compose.yml`, services use descriptive names:
- `backend` → builds from `./api`
- `frontend` → builds from `./web`

This is standard Docker Compose practice where service names are kept descriptive, while `build.context` references the actual directory paths.

## References in Documentation

All documentation files (README.md, DOCKER_SETUP.md, etc.) consistently reference:
- `api/` for backend-related content
- `web/` for frontend-related content

## Container Names

Docker containers follow the pattern: `crypto-pocket-butler-{service}`
- `crypto-pocket-butler-backend`
- `crypto-pocket-butler-frontend`
- `crypto-pocket-butler-db`
- `crypto-pocket-butler-keycloak`
