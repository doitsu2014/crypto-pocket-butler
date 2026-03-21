<!--
  SYNC IMPACT REPORT
  ==================
  Version change: N/A → 1.0.0 (initial ratification — template filled for the first time)

  Added sections (all new):
    - Core Principles (6 principles)
    - Architecture Reference Documents (NEW — mandates doc sync)
    - Technology Stack Constraints
    - Development Workflow
    - Governance

  Removed sections: None (initial version)

  Templates reviewed:
    - .specify/templates/plan-template.md    ✅ Compatible — "Constitution Check" placeholder
                                                 references this file; layer gates now explicit
    - .specify/templates/spec-template.md    ✅ Compatible — no constitution-specific gates required
    - .specify/templates/tasks-template.md   ✅ Compatible — phases align with Clean Architecture layers

  Deferred TODOs:
    - RATIFICATION_DATE set to 2026-03-21 (today); update to actual project inception date if known.
-->

# Crypto Pocket Butler Constitution

## Core Principles

### I. Clean Architecture — Strict Layer Separation (NON-NEGOTIABLE)

The backend MUST adhere to a strict four-layer Clean Architecture at all times.

**Layer definitions** (canonical source: `docs/architecture/01-architecture-design/04-code-structure.md`):

| Layer | Location | Responsibility |
|-------|----------|----------------|
| **Domain** | `api/src/domains/` | Business entities, value objects, aggregate roots, domain rules. MUST have zero external dependencies (no DB, no HTTP, no framework imports). |
| **Application** | `api/src/application/` | Use cases (`usecases/`), services (`services/`), repository traits (`repositories/`), DTOs (`dto/`), background jobs (`jobs/`), concurrency utilities (`concurrency/`). MUST NOT import from `infrastructure/` or `transport/`. |
| **Infrastructure** | `api/src/infrastructure/` | Implements application interfaces: persistence (`persistence/`), external APIs (`external/`), caching (`cache/`). MUST NOT be imported by domain or application layers directly. |
| **Transport** | `api/src/transport/http/` | HTTP handlers (`handlers/`), routes (`routes.rs`), error mapping (`error.rs`). Receives injected use cases via Axum `Extension`. MUST NOT import from `infrastructure/` directly. |

**Dependency direction** is always inward: Transport → Application → Domain.
Infrastructure implements interfaces declared by Application (dependency inversion).

**Handler injection rule**: HTTP handlers receive use-case structs via Axum `Extension` state.
Use cases are constructed once at startup in `main.rs`, wrapped in `Arc`, and shared across
handlers. They are NEVER re-instantiated per request.

Any violation MUST be documented in the plan's Complexity Tracking table with explicit
justification before merging.

### II. Domain-Driven Design — Five Bounded Contexts

Business logic lives in the domain and application layers — never in handlers or infrastructure.

**Canonical domain model documentation**: `docs/architecture/01-architecture-design/01-domain-models/`

**Bounded contexts and aggregate roots**:

| Domain | Aggregate Root | Key Invariants |
|--------|---------------|----------------|
| Portfolio | `Portfolio` | Name unique per user; exactly one default per user; target allocation weights sum to 100%; guardrail deviation alerting |
| Account | `Account` | Holdings consistency; credential encryption |
| Asset | `Asset` | Asset price uniqueness per chain |
| Holding & Allocation | *(enriched from Account + Asset)* | Quantities normalised; prices enriched before weight calculation |
| Chain & Token | *(DB-driven, no hardcoded list)* | RPC URL loaded from `evm_chains` table |

**Design rules**:
- Domain entities MUST encapsulate their own validation (e.g., `validate_name`,
  `validate_guardrails`, `check_ownership`).
- Each domain-facing operation MUST be encapsulated in a use-case struct under
  `application/usecases/` (currently: `AccountUseCases`, `ChainUseCases`,
  `PortfolioUseCases`, `RecommendationUseCases`, `SnapshotUseCases`).
- Repository *traits* are declared in `application/repositories/`
  (e.g., `AccountRepository`, `PortfolioRepository`, `AssetRepository`).
  Concrete implementations live in `infrastructure/persistence/`.
  The application layer MUST depend only on the trait — never the implementation.

**Canonical business service documentation**: `docs/architecture/01-architecture-design/03-business-services.md`

### III. Security-First (NON-NEGOTIABLE)

All protected routes MUST authenticate via Keycloak OIDC (JWT Bearer tokens).

- No endpoint accessing user data MAY be served without a valid, verified JWT.
- Admin operations MUST additionally verify the `admin` Keycloak role claim.
- The web layer MUST proxy all API calls through Next.js server routes to prevent
  token exposure to the browser.
- Sessions MUST be stored in `httpOnly` cookies — never in localStorage or client state.
- SQL queries MUST NOT be constructed via raw string interpolation; use SeaORM query
  builders exclusively.
- Any security-sensitive change (auth middleware, token handling, role checks) MUST include
  a security review note in the PR description.

### IV. Configuration-Driven Extensibility

Runtime behaviour MUST be driven by database configuration — not hardcoded enums or match arms.

- EVM chains, tokens, and their RPC URLs MUST be loaded from the `evm_chains` / `evm_tokens`
  DB tables. Adding a new chain requires only a DB row insert — no code changes.
- Feature flags, supported asset types, and connector parameters MUST be externalised to
  configuration wherever practical.
- Hardcoded lists of blockchain-specific variants (old-style Rust enums for chain names,
  RPC URLs, or native symbols) are PROHIBITED for any entity an operator may extend without
  redeployment.

**Canonical chain/token documentation**: `docs/architecture/01-architecture-design/01-domain-models/05-chain-token-domain.md`

### V. Living Architecture Documentation

The architecture documentation in `docs/architecture/01-architecture-design/` is the
authoritative reference for the system design. It MUST be kept in sync with the code.

**Canonical documentation set** (all located under `docs/architecture/01-architecture-design/`):

| File | Covers |
|------|--------|
| `01-domain-models/` | DDD bounded contexts, aggregate roots, invariants, repository interfaces, domain events |
| `02-api-dataflow.md` | API endpoint topology, data-flow sequence diagrams, domain validation flows |
| `03-business-services.md` | Application layer services, use-case catalogue, business logic operations, portfolio construction flow |
| `04-code-structure.md` | Module tree, layer dependency graph, handler structure, DB entity relationships |

**Mandatory update triggers**: Any PR that introduces or changes any of the following MUST
also update the relevant documentation file(s) listed above:
- A new or removed domain aggregate, entity, or value object
- A new or changed use case or application service
- A new or changed HTTP handler or route
- A layer dependency rule exception (with explicit justification)
- A new or changed database entity relationship
- A new external connector (EVM chain, exchange API, price feed)

Failure to update these docs MUST block PR merge.

### VI. Observability & Simplicity

Every operation that touches external systems or user data MUST be observable.

- Structured logging is REQUIRED for all background jobs, external-API calls, and error paths.
- Background jobs MUST emit start, completion, and failure log entries with: job name,
  duration, affected record counts, and error reason.
- YAGNI: complexity MUST be justified. The minimum abstraction needed for the current task
  is the correct one — three similar lines of code is preferable to a premature abstraction.
- Performance goal: API p95 latency SHOULD remain under 500 ms for read operations;
  background job cycles SHOULD complete within their scheduled interval.

## Architecture Reference Documents

The canonical architecture documentation lives at:

```
docs/architecture/01-architecture-design/
├── 01-domain-models/          # DDD bounded contexts (5 domains)
│   ├── 01-portfolio-domain.md
│   ├── 02-account-domain.md
│   ├── 03-asset-domain.md
│   ├── 04-holding-allocation-domain.md
│   └── 05-chain-token-domain.md
├── 02-api-dataflow.md         # API topology & sequence diagrams
├── 03-business-services.md    # Application layer catalogue
└── 04-code-structure.md       # Module tree & layer dependency rules
```

These documents govern the code structure. When they conflict with the code, the code MUST
be updated to conform (or the document amended via the governance process in this file).

Additional reference documents:
- `docs/architecture/02-web-frontend.md` — Next.js/React web layer architecture
- `docs/use-cases/USE_CASES.md` — End-to-end user workflows for all personas
- `docs/README.md` — Documentation index and quick-start guide

## Technology Stack Constraints

The following technology choices are fixed unless amended via the governance process.

| Layer | Technology | Notes |
|-------|-----------|-------|
| API runtime | Rust + Axum | Async, type-safe HTTP server |
| ORM / migrations | SeaORM + SeaORM Migrator | Migration files in `api/migration/src/` |
| Database | PostgreSQL | Primary persistence store |
| Auth provider | Keycloak (OIDC) | JWT validation on every protected route |
| Web framework | Next.js (App Router) + React 19 | TypeScript-only; no plain JS source files |
| Web auth | NextAuth.js | Session via httpOnly cookie |
| EVM connectivity | ethers-rs / RPC URLs from DB | Chain config is DB-driven (Principle IV) |
| Package manager (web) | pnpm | Do not switch to npm/yarn without amendment |
| CI / containerisation | Docker Compose | See `docs/setup/DOCKER_SETUP.md` |

New dependencies MUST be evaluated against security, licence compatibility, and maintenance
status before being added. Prefer Rust crates already in `Cargo.toml`; prefer npm packages
already in `web/package.json`.

## Development Workflow

1. **Branch naming**: all features branch from `main`; format is `<seq>-<feature-slug>`
   (e.g., `183-add-rebalancing-ui`).
2. **Specification first**: non-trivial features MUST have a `specs/<branch>/spec.md` (via
   `/speckit.specify`) before implementation begins.
3. **Plan before code**: run `/speckit.plan` to produce `plan.md` and pass the Constitution
   Check gate before writing any implementation code.
4. **Task-driven implementation**: run `/speckit.tasks` to generate `tasks.md`; work tasks
   in dependency order (Setup → Foundational → User Stories → Polish).
5. **Testing discipline**: unit tests use `cargo test --lib`; integration tests MUST hit a
   real database — mocked-DB tests are discouraged (risk of mock/prod divergence masking
   broken migrations).
6. **PR checklist**:
   - All new public-facing routes have Swagger annotations.
   - No handler imports from `infrastructure/` directly.
   - No hardcoded chain/token lists introduced.
   - Architecture docs in `docs/architecture/01-architecture-design/` updated if structure changed (Principle V).
   - Security-sensitive changes include a security review note.
   - `docs/use-cases/USE_CASES.md` updated if user workflows changed.
7. **Merge coordination**: align with the team before merging during active release-branch cuts.

## Governance

This constitution supersedes all other development practices and conventions documented
elsewhere in the repository. In case of conflict, this document wins.

**Amendment procedure**:
1. Open a PR with the proposed change to `.specify/memory/constitution.md`.
2. Run `/speckit.constitution` to regenerate and validate the document, increment the version,
   and produce a Sync Impact Report.
3. At least one other contributor MUST review and approve the PR.
4. After merge, update any dependent templates or runtime guidance docs flagged in the report.

**Versioning policy** (semantic):
- MAJOR: backward-incompatible governance or principle removals/redefinitions.
- MINOR: new principle or section added, or materially expanded guidance.
- PATCH: clarifications, wording fixes, non-semantic refinements.

**Compliance review**: every PR description MUST include a "Constitution Check" confirming
that Principles I–VI are satisfied, or explicitly document exceptions in the plan's
Complexity Tracking table.

**Runtime development guidance**: see `docs/README.md` for pointers to architecture docs,
coding guidelines, setup guides, and use-case workflows used during day-to-day development.

**Version**: 1.0.0 | **Ratified**: 2026-03-21 | **Last Amended**: 2026-03-21
