# Crypto Pocket Butler Architecture Documentation

This folder contains architecture documentation for the Crypto Pocket Butler system.

## Documentation

| File | Description | Updates Needed |
|------|-------------|----------------|
| `ARCHITECTURE.md` | Comprehensive reference (topology, connectors, database schema) | ⚠️ Needs review for Apalis 1.0.0 |
| `backend.md` | Detailed Mermaid diagrams (API backend layer) | ✅ **Current focus** |
| `web.md` | Detailed Mermaid diagrams (Frontend layer) | ⚠️ Needs review |
| `NAMING_CONVENTION.md` | Naming conventions | ✅ Outdated |
| `TECHNICAL_DESIGN.md` | Technical design notes | ⚠️ Needs review |

## Current Focus: Detailed Architecture Diagrams

### `backend.md` - Backend API Architecture (Updated ✅)

**Latest update:** Business Services & Business Logic layer diagrams

**Diagrams included:**
1. **Domain-Driven Layered Architecture** - Domain/Entity/API layers
2. **Portfolio Domain Class Diagram** - core domain models
3. **Asset Domain Class Diagram** - assets, prices, contracts
4. **Holding & Allocation Domain** - entity relationships
5. **Chain & Token Domain** - EVM/Solana tokens
6. **API Endpoint Architecture** - route protection layers
7. **Data Flow: Create Portfolio** - sequence diagram
8. **Domain Model Validation Flow** - validation pipeline
9. **Business Services Layer** - service responsibilities
10. **Business Logic Layer** - validation rules and calculations
11. **Portfolio Construction Flow** - sequence diagram
12. **Business Rules Validation** - flowchart
13. **Updated DDD Map** - domain boundaries with business layers

### `web.md` - Frontend Web Architecture

**Diagrams included:**
1. High-Level Web Architecture
2. Frontend Technology Stack (mindmap)
3. Web Deployment Architecture
4. Frontend Auth Flow (sequence)
5. Frontend Component Structure
6. Frontend State Management
7. Frontend Response Flow (sequence)
8. Frontend Security Architecture
9. Frontend CI/CD Flow
10. Frontend Routing Strategy
11. Frontend Error Boundary Flow

## Recommendations

### For Apalis Migration (PR #157)

The existing `ARCHITECTURE.md` still references `tokio-cron-scheduler`. Consider:

1. Add section: **Background Job System (Apalis 1.0.0-rc.4)** replacing tokio-cron-scheduler
2. Update connector section with new apalis-board integration
3. Add apalis-board architecture to topology diagram

### For Consistency

1. Review `TECHNICAL_DESIGN.md` for alignment with current code
2. Update `NAMING_CONVENTION.md` with current conventions
3. Consider adding apalis-board architecture section to `ARCHITECTURE.md`

## Viewing the Diagrams

### GitHub
Mermaid diagrams will render automatically in GitHub's markdown viewer.

### Mermaid Live Editor
Use https://mermaid.live/ to view/edit diagrams.

### VS Code
Install the "Mermaid Preview" extension for real-time rendering.

---

*Last updated: March 10, 2026*
