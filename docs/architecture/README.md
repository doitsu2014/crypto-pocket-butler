# Architecture Design

This folder contains architecture documentation for Crypto Pocket Butler.

## Structure

```
docs/architecture/
├── README.md                          # This file
├── 01-architecture-design/            # Backend API architecture (DDD)
│   ├── README.md
│   ├── 01-domain-models.md            # 5 diagrams
│   ├── 02-api-dataflow.md             # 4 diagrams
│   ├── 03-business-services.md        # 4 diagrams
│   └── 04-code-structure.md           # 4 diagrams
└── 02-web-frontend.md                 # Frontend architecture (11 diagrams)
```

## Files

| File | Description | Diagrams |
|------|-------------|----------|
| `01-architecture-design/` | Backend API (DDD, domain models) | 17 |
| `02-web-frontend.md` | Frontend (Next.js, components) | 11 |
| `README.md` | This file | - |

## Architecture Types

### 1. Backend Architecture (01-architecture-design/)
Domain-Driven Design with business services and logic layers.

**See:**
- `01-domain-models.md` - Portfolio, Asset, Holding, Chain domains
- `02-api-dataflow.md` - API endpoints, data flow, validation
- `03-business-services.md` - Business services, logic, DDD principles
- `04-code-structure.md` - Code structure, modules, entities

### 2. Frontend Architecture (02-web-frontend.md)
Frontend web architecture including UI components and state management.

**See:**
- Next.js App Router structure
- TanStack Query patterns
- NextAuth.js integration
- Component organization

## Viewing

- **GitHub**: Mermaid diagrams render automatically
- **Mermaid Live Editor**: https://mermaid.live/
- **VS Code**: Install "Mermaid Preview" extension