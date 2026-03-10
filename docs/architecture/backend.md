# Crypto Pocket Butler Backend API Architecture

## High-Level API Architecture

```mermaid
graph TD
    A[Client<br/>Browser/Mobile/API] -->|HTTPS| B[Load Balancer<br/>NGINX/HAProxy]
    B --> C[API Server 1<br/>Rust + Axum + Keycloak]
    B --> D[API Server 2<br/>Rust + Axum + Keycloak]
    B --> E[API Server 3<br/>Rust + Axum + Keycloak]
    
    C -->|Job Queue| F[PostgreSQL Queue<br/>apalis_jobs]
    D -->|Job Queue| F
    E -->|Job Queue| F
    
    F -->|Consumes| G[Apalis Worker 1<br/>Fetch All Coins]
    F -->|Consumes| H[Apalis Worker 2<br/>EOD Snapshot]
    F -->|Consumes| I[Apalis Worker 3<br/>Multi-instance]
    
    G -->|Read/Write| J[PostgreSQL<br/>Primary Database]
    H -->|Read/Write| J
    I -->|Read/Write| J
    
    F -->|API| K[apalis-board<br/>Dashboard<br/>/admin/jobs]
```

---

## API Data Flow: Job Processing

```mermaid
sequenceDiagram
    participant Client
    participant LB as Load Balancer
    participant API
    participant Queue as PostgreSQL Queue
    participant Worker
    participant DB as PostgreSQL

    Client->>LB: POST /api/v1/jobs/fetch-all-coins
    LB->>API: Forward request
    API->>API: Validate auth (Keycloak)
    API->>Queue: PUSH FetchAllCoinsJob
    Queue-->>API: Acknowledgment
    API-->>Client: 200 OK
    
    Queue->>Worker: Notify pending job
    Worker->>Worker: Acquire lock
    Worker->>DB: BEGIN
    Worker->>DB: Update task status: Running
    Worker->>DB: fetch_all_coins()
    Worker->>DB: UPDATE result
    Worker->>DB: COMMIT
    Worker-->>Queue: Update status: Completed
    Queue-->>Worker: Release lock
```

---

## Multi-Instance API Architecture

```mermaid
graph LR
    subgraph "Instance 1"
        A1[API Server 1<br/>:3001]
        W1[Apalis Worker 1]
    end
    
    subgraph "Instance 2"
        A2[API Server 2<br/>:3001]
        W2[Apalis Worker 2]
    end
    
    subgraph "Instance 3"
        A3[API Server 3<br/>:3001]
        W3[Apalis Worker 3]
    end
    
    subgraph "Shared Services"
        Q[PostgreSQL Queue<br/>apalis_jobs]
        D[PostgreSQL DB]
        B[apalis-board]
    end
    
    A1 -->|Push/Consume| Q
    A2 -->|Push/Consume| Q
    A3 -->|Push/Consume| Q
    
    W1 -->|Consume| Q
    W2 -->|Consume| Q
    W3 -->|Consume| Q
    
    Q -->|Read/Write| D
    Q -->|API| B
```

---

## Backend Technology Stack

```mermaid
mindmap
  root((Crypto Pocket Butler Backend))
    Backend(Rust)
      API(Axum 0.8)
      Auth(Keycloak OIDC)
      DB(SeaORM + PostgreSQL)
      Queue(Apalis 1.0.0-rc.4)
        Workers(CronStream + PostgresStorage)
        Storage(PostgreSQL apalis_jobs table)
        Board(apalis-board-api)
    Infrastructure
      Container(Docker Compose)
      Auth(Keycloak)
      Load Balancer(NGINX/HAProxy)
      Monitoring(apalis-board Dashboard)
```

---

## Backend Deployment (Docker)

```mermaid
graph TB
    subgraph "Docker Network: crypto-network"
        LB[nginx:proxy<br/>Port 443]
        
        subgraph "API Service Stack"
            API1[api:3001]
            API2[api:3001]
            API3[api:3001]
        end
        
        PG[postgres:5432]
        KC[keycloak:8080]
    end
    
    LB --> API1
    LB --> API2
    LB --> API3
    
    API1 --> PG
    API2 --> PG
    API3 --> PG
    
    LB --> KC
```

---

## Backend Component Details: apalis-board

```mermaid
graph LR
    User[Admin User] -->|Browser| DB[Dashboard UI]
    DB -->|API| AQ[apalis-board API]
    AQ -->|Queries| PQ[PostgreSQL<br/>apalis_jobs table]
    
    AQ -.->|SSE Events| DB
    DB -.->|Real-time| AQ
    
    style AQ fill:#f9f,stroke:#333,stroke-width:2px
    style PQ fill:#bbf,stroke:#333,stroke-width:2px
```

---

## Backend Job Storage Schema

```mermaid
erDiagram
    apalis_jobs ||--o{ apalis_heartbeats : "has"
    apalis_jobs {
        bigint id PK
        text job_type
        jsonb payload
        text status
        text error_message
        jsonb context
        timestamp created_at
        timestamp started_at
        timestamp completed_at
        bigint priority
    }
    
    apalis_heartbeats {
        bigint id PK
        bigint job_id FK
        text worker_name
        timestamp last_heartbeat
    }
```

---

## Backend Security Architecture

```mermaid
graph TD
    Client[Client] -->|JWT Token| LB[Load Balancer]
    LB -->|Validate| KC[Keycloak]
    KC -->|Verify| DB[(PostgreSQL<br/>users/roles)]
    
    Client -->|Bearer Token| API[API Endpoint]
    API -->|Extract Claims| Auth[Keycloak Auth Layer]
    Auth -->|Check Role| RBAC[Role-Based Access Control]
    
    RBAC -->|Admin Role| Admin[Admin Routes<br/>apalis-board]
    RBAC -->|Authenticated| User[User Routes]
```