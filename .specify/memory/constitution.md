<!--
Sync Impact Report
==================
Version change: N/A → 1.0.0 (initial constitution)
Modified principles: N/A (new creation)
Added sections:
  - Core Principles (5 principles)
  - Technology Stack Constraints
  - Development Workflow
  - Governance
Removed sections: None
Templates requiring updates:
  - .specify/templates/plan-template.md ✅ (no changes needed)
  - .specify/templates/spec-template.md ✅ (no changes needed)
  - .specify/templates/tasks-template.md ✅ (no changes needed)
Follow-up TODOs: None
-->

# Crypto Pocket Butler Constitution

## Core Principles

### I. Security-First Design

All cryptographic operations and user data handling MUST prioritize security above all else.

- Private keys and sensitive data MUST NEVER be stored in plaintext
- All API endpoints MUST implement proper authentication and authorization
- Cryptographic operations MUST use well-vetted libraries (e.g., `ring`, `rust-crypto`)
- Input validation MUST be applied at all boundaries (API, CLI, UI)
- Secrets management MUST follow industry best practices (environment variables, secret managers)

**Rationale**: As a cryptocurrency application, security vulnerabilities can result in direct financial loss.

### II. Rust Backend Excellence

The backend MUST leverage Rust's safety guarantees and performance characteristics.

- All backend services MUST be written in Rust with safe defaults
- Unsafe code blocks MUST include documentation justifying necessity
- Error handling MUST use `Result<T, E>` types, NOT panics
- Concurrency MUST use Rust's ownership model to prevent data races
- Memory safety MUST be verified through compilation without `unsafe` where possible

**Rationale**: Rust's memory safety and zero-cost abstractions are critical for reliable cryptocurrency operations.

### III. Modern Frontend Stack

The frontend MUST use React, Next.js, and raw Tailwind CSS for UI development.

- UI components MUST be built with React functional components and hooks
- Styling MUST use Tailwind CSS utility classes directly (NO TanStack libraries due to security vulnerabilities)
- State management MUST use React Context or Zustand (lightweight alternatives)
- Pages MUST leverage Next.js App Router for optimal routing and SSR
- Component structure MUST follow atomic design principles

**Rationale**: Raw Tailwind CSS avoids the TanStack vulnerability issue while maintaining development velocity.

### IV. Test-Driven Development (NON-NEGOTIABLE)

All features MUST follow TDD discipline: Tests written → User approved → Tests fail → Then implement.

- Unit tests MUST achieve minimum 80% code coverage for business logic
- Integration tests MUST verify service interactions and database operations
- Contract tests MUST validate API endpoint behavior
- E2E tests MUST cover critical user flows (authentication, transactions)
- Red-Green-Refactor cycle MUST be strictly enforced

**Rationale**: Financial applications require comprehensive testing to prevent costly bugs in production.

### V. API-First Architecture

Backend services MUST expose functionality through well-defined APIs before building frontend consumers.

- API contracts MUST be defined using OpenAPI/Swagger specifications
- Endpoints MUST follow RESTful conventions with clear resource naming
- Request/response schemas MUST be validated using JSON Schema
- API versioning MUST be implemented from the start (e.g., `/api/v1/`)
- Error responses MUST follow RFC 7807 Problem Details format

**Rationale**: API-first design enables parallel frontend/backend development and future mobile/web clients.

## Technology Stack Constraints

### Backend Requirements

- **Language**: Rust (latest stable)
- **Web Framework**: Axum or Actix-web (chosen based on team expertise)
- **Database**: PostgreSQL with SQLx or Diesel ORM
- **Authentication**: JWT tokens with refresh token rotation
- **Testing**: Built-in Rust test framework + cargo-nextest

### Frontend Requirements

- **Framework**: Next.js 14+ with App Router
- **UI Library**: React 18+
- **Styling**: Tailwind CSS 3+ (raw utility classes, NO component libraries with known vulnerabilities)
- **State Management**: React Context API or Zustand
- **Testing**: Jest + React Testing Library

### Prohibited Dependencies

- TanStack libraries (Query, Table, Form, Router, etc.) - known security vulnerabilities
- Any library with active CVEs without documented mitigation

## Development Workflow

### Branch Strategy

- `main` branch MUST always be deployable
- Feature branches MUST follow `###-feature-name` convention
- Pull requests MUST pass CI checks before merge

### Code Review Requirements

- All PRs MUST verify compliance with these principles
- Security-sensitive changes MUST have two approvals
- API contract changes MUST update OpenAPI spec

### Quality Gates

- Linting MUST pass (clippy for Rust, ESLint for TypeScript)
- Type checking MUST pass (TypeScript strict mode)
- All tests MUST pass before merge
- Code coverage MUST NOT decrease

## Governance

This constitution supersedes all other development practices. Any amendments require:

1. Documentation of the proposed change with rationale
2. Review and approval by project maintainers
3. Migration plan for any breaking changes
4. Version increment following semantic versioning rules

All pull requests and code reviews MUST verify compliance with these principles. Complexity must be justified with clear business value.

**Version**: 1.0.0 | **Ratified**: 2026-05-14 | **Last Amended**: 2026-05-14
