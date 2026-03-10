# Crypto Pocket Butler Architecture Documentation

This folder contains Mermaid diagrams for Crypto Pocket Butler's system architecture.

## Documentation

| File | Description |
|------|-------------|
| `README.md` | This file |
| `backend.md` | Backend API architecture (Part 1) |
| `web.md` | Frontend web architecture (Part 2) |

---

## Part 1: Backend API Architecture (`backend.md`)

### High-Level API Architecture
- Client → Load Balancer → Multiple API Servers
- API Servers → PostgreSQL Queue (apalis_jobs)
- Apalis Workers → PostgreSQL Database
- apalis-board Dashboard for monitoring

### Key Features
- Rust + Axum 0.8 API
- Keycloak OIDC authentication
- SeaORM PostgreSQL integration
- Apalis 1.0.0-rc.4 job queue system
- Multi-instance scaling support

### Diagrams Included
1. High-Level API Architecture
2. API Data Flow: Job Processing (sequence diagram)
3. Multi-Instance API Architecture
4. Backend Technology Stack (mindmap)
5. Backend Deployment (Docker)
6. apalis-board Component Details
7. Job Storage Schema (ER diagram)
8. Backend Security Architecture

---

## Part 2: Frontend Web Architecture (`web.md`)

### High-Level Web Architecture
- Browser → Load Balancer → Next.js 16 App
- Next.js → TanStack Query → API Server
- NextAuth.js → Keycloak for authentication

### Key Features
- Next.js 16 with app router
- TailwindCSS 4 for styling
- TanStack Query for data fetching
- NextAuth.js v5 for authentication
- PKCE flow for Keycloak

### Diagrams Included
1. High-Level Web Architecture
2. Frontend Technology Stack (mindmap)
3. Web Deployment Architecture
4. Frontend Data Flow: Authentication (sequence diagram)
5. Frontend Component Structure
6. Frontend State Management
7. Frontend Response Flow (sequence diagram)
8. Frontend Security Architecture
9. Frontend Deployment Flow (CI/CD)
10. Frontend Routing Strategy
11. Frontend Error Boundary Flow

---

## Viewing the Diagrams

### GitHub
The diagrams will render automatically in GitHub's markdown viewer.

### Mermaid Live Editor
Use https://mermaid.live/ to view/edit diagrams.

### VS Code
Install the "Mermaid Preview" extension for real-time rendering.

---

## Source

All diagrams are sourced from:
- `docs/architecture/backend.md` - Part 1: Backend API
- `docs/architecture/web.md` - Part 2: Web Frontend