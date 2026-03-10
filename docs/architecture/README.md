# Architecture Design

This folder contains architecture documentation for Crypto Pocket Butler.

## Files

| File | Description |
|------|-------------|
| `01-architecture-design.md` | Backend API architecture (DDD, domain models, business services) |
| `02-web-frontend.md` | Frontend web architecture (Next.js, state management, components) |
| `README.md` | This file |

## Architecture Types

### 1. System Architecture
High-level view of the entire system including all components and their interactions.

### 2. Domain-Driven Architecture
Detailed domain models, entity relationships, and business logic layered architecture.

See `01-architecture-design.md` for:
- Layered Architecture (Domain/Entity/API)
- Domain Class Diagrams (Portfolio, Asset, Holding, Snapshot)
- Business Services Layer
- Business Logic Layer
- Portfolio Construction Flow
- Business Rules Validation

### 3. Frontend Architecture
Frontend web architecture including UI components and state management.

See `02-web-frontend.md` for:
- Next.js App Router structure
- TanStack Query patterns
- NextAuth.js integration
- Component organization

## Viewing

- **GitHub**: Mermaid diagrams render automatically
- **Mermaid Live Editor**: https://mermaid.live/
- **VS Code**: Install "Mermaid Preview" extension