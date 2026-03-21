<!--
  SYNC IMPACT REPORT
  ==================
  Version change: 1.1.0 → 1.2.0 (MINOR — new Principle VII: Web Frontend Architecture Rules;
                                   Principle V expanded with frontend doc triggers;
                                   Principle III admin role corrected to "administrator";
                                   02-web-frontend.md updated with task patterns section)

  Modified principles:
    - Principle III: Security-First — corrected Keycloak admin role claim from "admin"
      to "administrator" (matching actual realm role in auth.ts)
    - Principle V: Living Architecture Documentation — 02-web-frontend.md elevated to
      first-class canonical doc with mandatory frontend update triggers

  Added sections:
    - Principle VII: Web Frontend Architecture Rules (NEW — mandates agents consult
      02-web-frontend.md; enumerates non-negotiable frontend design rules)
    - Development Workflow PR checklist: frontend-specific items added

  Removed sections: None

  Templates reviewed:
    - .specify/templates/plan-template.md    ✅ Compatible — Constitution Check covers
                                                 frontend architecture via Principle VII
    - .specify/templates/spec-template.md    ✅ Compatible — no changes required
    - .specify/templates/tasks-template.md   ✅ Compatible — web path conventions noted

  Architecture docs updated:
    - docs/architecture/02-web-frontend.md   ✅ Frontend Development Task Patterns section added

  Deferred TODOs:
    - RATIFICATION_DATE set to 2026-03-21; update to actual project inception date if known.
-->

# Crypto Pocket Butler Constitution

## Core Principles

### I. Clean Architecture + SOLID (NON-NEGOTIABLE)

The backend MUST adhere to a strict four-layer Clean Architecture reinforced by SOLID
principles at all times.

**Canonical source**: `docs/architecture/01-architecture-design/04-code-structure.md`

#### Layer Definitions

| Layer              | Location                  | Responsibility |
| ------------------ | ------------------------- | -------------- |
| **Domain**         | `api/src/domains/`        | Business entities, value objects, aggregate roots, domain rules. MUST have zero external dependencies (no DB, no HTTP, no framework imports). |
| **Application**    | `api/src/application/`    | Use cases (`usecases/`), services (`services/`), repository traits (`repositories/`), DTOs (`dto/`), background jobs (`jobs/`), concurrency utilities (`concurrency/`). MUST NOT import from `infrastructure/` or `transport/`. |
| **Infrastructure** | `api/src/infrastructure/` | Implements application interfaces: persistence (`persistence/`), external APIs (`external/`), caching (`cache/`). MUST NOT be imported by domain or application layers directly. |
| **Transport**      | `api/src/transport/http/` | HTTP handlers (`handlers/`), routes (`routes.rs`), error mapping (`error.rs`). Receives injected use cases via Axum `Extension`. MUST NOT import from `infrastructure/` directly. |

**Dependency direction** is always inward: Transport → Application → Domain.
Infrastructure implements interfaces declared by Application (dependency inversion).

**Handler injection rule**: HTTP handlers receive use-case structs via Axum `Extension` state.
Use cases are constructed once at startup in `main.rs`, wrapped in `Arc`, and shared across
handlers. They are NEVER re-instantiated per request.

#### SOLID Rules

**S — Single Responsibility**
Each handler owns one route concern. Each use case owns one aggregate's operations. Each
repository trait persists one aggregate. A handler MUST NOT contain business logic; a use
case MUST NOT contain HTTP concerns; a domain entity MUST NOT contain persistence logic.

**O — Open/Closed**
The system MUST be open for extension and closed for modification:
- New EVM chains/tokens: add a DB row — no code changes (see Principle IV).
- New use case: add a new struct under `application/usecases/` — no existing use cases or
  handlers modified.
- New repository backend: implement the existing trait in `infrastructure/persistence/` —
  application layer is unaffected.
- New HTTP handler: add a file under `transport/http/handlers/` and register in `routes.rs`
  — no existing handler modified.

**L — Liskov Substitution**
Any `infrastructure/` implementation MUST be fully substitutable for its application-layer
trait without altering the behaviour observed by callers. Implementations that cannot honour
the full trait contract MUST NOT implement it — instead, refine or split the trait.
Test mocks MUST honour the same preconditions, postconditions, and invariants as production
implementations.

**I — Interface Segregation**
Traits MUST be narrow and domain-specific — no "god traits":
- Repository traits are split by aggregate: `AccountRepository`, `PortfolioRepository`,
  `AssetRepository` — never a combined `Repository<T>`.
- A caller MUST NOT be forced to depend on methods it does not use. Prefer minimal sub-traits
  when a consumer requires only a subset of operations.

**D — Dependency Inversion**
High-level modules MUST NOT depend on low-level modules. Both depend on abstractions:
- Application layer declares repository traits; Infrastructure implements them. Application
  NEVER imports concrete impls.
- Transport layer depends on injected use-case structs — never on infrastructure types.
- Wiring of concrete impls to traits happens once in `main.rs` via Axum `Extension`.
- Unit tests substitute production impls with in-memory or mock impls via the same traits.

```
High-level:  Transport  →  Application (use cases, traits)
                                 ↑ implements
Low-level:            Infrastructure (concrete impls)
```

Any violation of the above rules MUST be documented in the plan's Complexity Tracking table
with explicit justification before merging.

### II. Domain-Driven Design — Five Bounded Contexts

Business logic lives in the domain and application layers — never in handlers or infrastructure.

**Canonical domain model documentation**: `docs/architecture/01-architecture-design/01-domain-models/`

**Bounded contexts and aggregate roots**:

| Domain               | Aggregate Root                    | Key Invariants                                                                                                          |
| -------------------- | --------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Portfolio            | `Portfolio`                       | Name unique per user; exactly one default per user; target allocation weights sum to 100%; guardrail deviation alerting |
| Account              | `Account`                         | Holdings consistency; credential encryption                                                                             |
| Asset                | `Asset`                           | Asset price uniqueness per chain                                                                                        |
| Holding & Allocation | _(enriched from Account + Asset)_ | Quantities normalised; prices enriched before weight calculation                                                        |
| Chain & Token        | _(DB-driven, no hardcoded list)_  | RPC URL loaded from `evm_chains` table                                                                                  |

**Design rules**:

- Domain entities MUST encapsulate their own validation (e.g., `validate_name`,
  `validate_guardrails`, `check_ownership`).
- Each domain-facing operation MUST be encapsulated in a use-case struct under
  `application/usecases/` (currently: `AccountUseCases`, `ChainUseCases`,
  `PortfolioUseCases`, `RecommendationUseCases`, `SnapshotUseCases`).
- Repository _traits_ are declared in `application/repositories/`
  (e.g., `AccountRepository`, `PortfolioRepository`, `AssetRepository`).
  Concrete implementations live in `infrastructure/persistence/`.
  The application layer MUST depend only on the trait — never the implementation.

**Canonical business service documentation**: `docs/architecture/01-architecture-design/03-business-services.md`

### III. Security-First (NON-NEGOTIABLE)

All protected routes MUST authenticate via Keycloak OIDC (JWT Bearer tokens).

- No endpoint accessing user data MAY be served without a valid, verified JWT.
- Admin operations MUST additionally verify the `"administrator"` Keycloak realm role claim
  (extracted from `realm_access.roles` in the JWT payload).
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

**Canonical documentation set — Backend** (all under `docs/architecture/01-architecture-design/`):

| File                      | Covers                                                                                                 |
| ------------------------- | ------------------------------------------------------------------------------------------------------ |
| `01-domain-models/`       | DDD bounded contexts, aggregate roots, invariants, repository interfaces, domain events                |
| `02-api-dataflow.md`      | API endpoint topology, data-flow sequence diagrams, domain validation flows                            |
| `03-business-services.md` | Application layer services, use-case catalogue, business logic operations, portfolio construction flow |
| `04-code-structure.md`    | Module tree, layer dependency graph, handler structure, DB entity relationships, SOLID rules           |

**Backend mandatory update triggers**: Any PR that introduces or changes any of the following MUST
also update the relevant backend documentation file(s) listed above:

- A new or removed domain aggregate, entity, or value object
- A new or changed use case or application service
- A new or changed HTTP handler or route (API)
- A layer dependency rule exception (with explicit justification)
- A new or changed database entity relationship
- A new external connector (EVM chain, exchange API, price feed)

**Canonical documentation — Frontend**:

| File                              | Covers                                                                                                       |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `docs/architecture/02-web-frontend.md` | App Router structure, component/hook/context layout, API proxy pattern, state management, auth flow, RBAC, task patterns |

**Frontend mandatory update triggers**: Any PR that introduces or changes any of the following MUST
also update `docs/architecture/02-web-frontend.md`:

- A new page route or route group
- A new shared component, hook, or context
- A change to state management approach
- A change to authentication/session handling
- A change to the API proxy pattern or `apiClient` interface
- A new UI library or charting library
- A new role-based access rule

Failure to update the relevant doc(s) MUST block PR merge.

### VI. Observability & Simplicity (backend)

Every operation that touches external systems or user data MUST be observable.

- Structured logging is REQUIRED for all background jobs, external-API calls, and error paths.
- Background jobs MUST emit start, completion, and failure log entries with: job name,
  duration, affected record counts, and error reason.
- YAGNI: complexity MUST be justified. The minimum abstraction needed for the current task
  is the correct one — three similar lines of code is preferable to a premature abstraction.
- Performance goal: API p95 latency SHOULD remain under 500 ms for read operations;
  background job cycles SHOULD complete within their scheduled interval.

### VII. Web Frontend Architecture Rules (NON-NEGOTIABLE)

**Canonical source**: `docs/architecture/02-web-frontend.md`

Agents MUST read `docs/architecture/02-web-frontend.md` before writing or reviewing any
frontend code. The rules below are derived from and governed by that document.

#### Technology Constraints

- **Framework**: Next.js 16 App Router only — Pages Router MUST NOT be used.
- **Language**: TypeScript only — no plain `.js` source files in `web/`.
- **Styling**: TailwindCSS 4 only — no CSS-in-JS libraries, no Shadcn/UI (not installed).
- **Charts**: Recharts — no additional charting libraries without a constitution amendment.
- **Package manager**: pnpm — MUST NOT be changed to npm or yarn.

#### API Communication Rules

All frontend-to-backend communication MUST go through the unified Next.js API proxy:

```
Client component  →  apiClient("/v1/...")
                       ↓
                  /api/backend/[...path]/route.ts  (server-side, attaches Bearer token)
                       ↓
                  Rust/Axum backend
```

- Browser code MUST NEVER call the backend directly (no `fetch("http://api:...")` in
  client components).
- All client-side API calls MUST use `apiClient<T>()` from `web/lib/api-client.ts`.
- Server components and server-side route handlers MAY use `directBackendClient()` with
  an explicit access token.
- Error handling MUST use `ApiError` types: `auth | validation | server | network | unknown`.

#### State Management Rules

- **Server/async state**: TanStack Query v5 — the ONLY permitted data-fetching library.
  Default stale time is 30 s; refetch on window focus is enabled.
- **Client state**: React Context — the ONLY permitted client state mechanism.
- **No Zustand**: Zustand MUST NOT be added. If global client state grows complex,
  amend this constitution with justification.
- **No localStorage for query data**: TanStack Query cache is in-memory only. Persisting
  query data to localStorage is PROHIBITED.
- All data-fetching hooks live in `web/hooks/`; context providers live in `web/contexts/`.

#### Authentication & Security Rules

- Sessions are managed by NextAuth.js v5 using the **JWT strategy** with encrypted
  `httpOnly` cookies. Tokens are NEVER exposed to client-side JavaScript.
- Token refresh is **proactive**: NextAuth refreshes the access token when it has consumed
  50% of its lifetime (using the `offline_access` / refresh-token grant).
- The admin-protected role claim is **`"administrator"`** (Keycloak realm role from
  `realm_access.roles`). Any page that requires admin access MUST check this claim.
- Middleware (`web/middleware.ts`) protects: `/dashboard`, `/portfolios`, `/api/backend`,
  `/admin`. Any new protected route group MUST be added to the middleware matcher.

#### Component & File Structure Rules

```
web/
├── app/                    # Next.js App Router — pages and route handlers only
│   ├── api/backend/        # Unified proxy — NEVER add auth logic here
│   └── api/auth/           # NextAuth handlers — NEVER customise directly
├── components/             # Shared UI components — no data fetching here
│   └── portfolio/          # Domain-specific components
├── hooks/                  # All useQuery/useMutation hooks — one hook file per domain
├── contexts/               # React Context providers only
├── lib/                    # api-client.ts, query-client.ts — low-level utilities
└── types/                  # TypeScript type definitions
```

- Components MUST NOT contain data-fetching logic — delegate to hooks in `web/hooks/`.
- Hooks MUST use `apiClient` exclusively — no direct `fetch` calls.
- New page routes MUST be added under `web/app/` using the App Router convention
  (`page.tsx`, `layout.tsx`, `loading.tsx`, `error.tsx`).

Any deviation from the above rules MUST be documented in the plan's Complexity Tracking
table with explicit justification before merging.

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

**Frontend architecture** (canonical source for Principle VII):

- `docs/architecture/02-web-frontend.md` — App Router structure, component/hook/context
  layout, unified API proxy, state management, auth flow, RBAC, frontend task patterns

**Additional references**:

- `docs/use-cases/USE_CASES.md` — End-to-end user workflows for all personas
- `docs/README.md` — Documentation index and quick-start guide

## Technology Stack Constraints

The following technology choices are fixed unless amended via the governance process.

| Layer                 | Technology                      | Notes                                       |
| --------------------- | ------------------------------- | ------------------------------------------- |
| API runtime           | Rust + Axum                     | Async, type-safe HTTP server                |
| ORM / migrations      | SeaORM + SeaORM Migrator        | Migration files in `api/migration/src/`     |
| Database              | PostgreSQL                      | Primary persistence store                   |
| Auth provider         | Keycloak (OIDC)                 | JWT validation on every protected route     |
| Web framework         | Next.js (App Router) + React 19 | TypeScript-only; no plain JS source files   |
| Web auth              | NextAuth.js                     | Session via httpOnly cookie                 |
| EVM connectivity      | ethers-rs / RPC URLs from DB    | Chain config is DB-driven (Principle IV)    |
| Package manager (web) | pnpm                            | Do not switch to npm/yarn without amendment |
| CI / containerisation | Docker Compose                  | See `docs/setup/DOCKER_SETUP.md`            |

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
6. **PR checklist — Backend**:
   - All new public-facing API routes have Swagger annotations.
   - No handler imports from `infrastructure/` directly.
   - No hardcoded chain/token lists introduced.
   - Backend architecture docs in `docs/architecture/01-architecture-design/` updated if
     structure changed (Principle V).
   - Security-sensitive changes include a security review note.
   - `docs/use-cases/USE_CASES.md` updated if user workflows changed.

7. **PR checklist — Frontend**:
   - `docs/architecture/02-web-frontend.md` consulted before writing frontend code (Principle VII).
   - `docs/architecture/02-web-frontend.md` updated if app structure, components, hooks,
     contexts, or auth handling changed (Principle V).
   - No direct `fetch()` to backend from client components — `apiClient()` only.
   - No new global state libraries added (no Zustand, no Redux, etc.).
   - No `localStorage` used for query/server state.
   - All new protected routes added to `web/middleware.ts` matcher.
   - New data-fetching logic added as a hook in `web/hooks/`, not inline in components.
8. **Merge coordination**: align with the team before merging during active release-branch cuts.

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
that Principles I–VII are satisfied (backend PRs: I–VI; frontend PRs: III + VII), or
explicitly document exceptions in the plan's Complexity Tracking table.

**Runtime development guidance**: see `docs/README.md` for pointers to architecture docs,
coding guidelines, setup guides, and use-case workflows used during day-to-day development.

**Version**: 1.2.0 | **Ratified**: 2026-03-21 | **Last Amended**: 2026-03-21
