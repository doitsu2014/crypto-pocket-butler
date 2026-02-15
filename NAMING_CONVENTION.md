# Naming Convention

This document describes the naming conventions used throughout the Crypto Pocket Butler project.

## Service Folder Names

The project follows a clear naming convention for service directories:

### Backend Service
- **Folder Name**: `backend-rust`
- **Technology**: Rust (Axum framework)
- **Location**: `/backend-rust`
- **Docker Service Name**: `backend`

### Frontend Service
- **Folder Name**: `frontend-react`
- **Technology**: React (Next.js framework)
- **Location**: `/frontend-react`
- **Docker Service Name**: `frontend`

## Rationale

The folder names explicitly include the technology stack (`rust`, `react`) to:
1. Make the technology choice immediately clear when browsing the repository
2. Allow for potential future additions (e.g., alternative frontend implementations)
3. Provide clarity in monorepo structure
4. Help new contributors quickly understand the project architecture

## Docker Compose Services

In `docker-compose.yml`, services use simplified names:
- `backend` → builds from `./backend-rust`
- `frontend` → builds from `./frontend-react`

This is standard Docker Compose practice where service names are kept short and descriptive, while `build.context` references the actual directory paths.

## References in Documentation

All documentation files (README.md, DOCKER_SETUP.md, etc.) consistently reference:
- `backend-rust/` for backend-related content
- `frontend-react/` for frontend-related content

## Container Names

Docker containers follow the pattern: `crypto-pocket-butler-{service}`
- `crypto-pocket-butler-backend`
- `crypto-pocket-butler-frontend`
- `crypto-pocket-butler-db`
- `crypto-pocket-butler-keycloak`
