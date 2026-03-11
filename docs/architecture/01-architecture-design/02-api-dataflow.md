# Backend Architecture - API & Data Flow

## API Endpoint Architecture

```mermaid
graph TD
    PR[Public Routes]
    PT[Protected Routes]
    PF[Portfolio Routes]
    AC[Account Routes]
    AD[Admin Routes]
    H[Health Endpoint]
    AU[Auth]
    PH[Portfolio Handlers]
    AH[Account Handlers]
    AA[Admin Auth]
    
    PR --> H
    PT --> AU
    PF --> PH
    AC --> AH
    AD --> AA
    AU --> PF
    AU --> AC
    AA --> AD
```

---

## Data Flow: Create Portfolio

```mermaid
sequenceDiagram
    participant C as Client
    participant A as API
    participant AU as Auth
    participant H as PortfolioHandler
    participant D as PortfolioDomain
    participant E as PortfolioEntity
    participant DB as Database

    C->>A: POST /api/portfolios
    A->>AU: Validate JWT Token
    AU-->>A: Decoded Claims
    A->>H: CreatePortfolioRequest
    H->>D: portfolio_to_domain
    D->>D: validate_name
    D->>D: validate_guardrails
    D->>E: new portfolio
    E->>DB: INSERT
    DB-->>E: Return UUID
    E-->>D: Portfolio
    D-->>H: Domain Portfolio
    H-->>A: PortfolioResponse
    A-->>C: 201 Created
```

---

## Domain Model Validation Flow

```mermaid
graph TD
    R[Request] --> P[Parse]
    P --> V[Validate]
    V --> VL[Valid]
    VL --> DM[Domain Model]
    DM --> B[Business Rules]
    B --> PS[Pass]
    PS --> T[Transform]
    T --> DB[Database]
    DB --> S[Success]
    V --> IV[Invalid]
    IV --> E1[Error 400]
    B --> FL[Fail]
    FL --> E2[Error 422]
    DB --> CF[Conflict]
    CF --> E3[Error 409]
    S --> RP[Response]
```

---

## Business Rules Validation

```mermaid
flowchart TD
    ST[Start] --> CN[Check Name]
    CN -->|Invalid| E1[Error 400]
    CN -->|Valid| CD[Check Default]
    CD -->|Set Default| UO[Unset Other]
    CD -->|Not Default| CG[Check Guardrails]
    UO --> CG
    CG -->|Invalid| E2[Error 400]
    CG -->|Valid| SV[Save]
    SV --> SC[Success 201]
    SV -->|Conflict| E3[Error 409]
```