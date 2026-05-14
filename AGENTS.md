# Crypto Pocket Butler - Agent Guidance

## Project Overview

**Crypto Pocket Butler** is a cryptocurrency portfolio management application that helps users track, manage, and optimize their crypto holdings.

## Technology Stack

### Backend
- **Language**: Rust (latest stable)
- **Web Framework**: Axum or Actix-web
- **Database**: PostgreSQL with SQLx or Diesel
- **Authentication**: JWT with refresh tokens
- **Testing**: cargo-nextest

### Frontend
- **Framework**: Next.js 14+ (App Router)
- **UI**: React 18+ with Tailwind CSS (raw utility classes)
- **State**: React Context or Zustand
- **Testing**: Jest + React Testing Library

### Prohibited Dependencies
- TanStack libraries (Query, Table, Form, Router) - security vulnerabilities
- Any library with active unmitigated CVEs

---

## Development Workflows

### Workflow 1: New Feature / User Story

When a developer wants to implement a **new feature or user story**, follow this sequence:

```
/speckit.specify → /speckit.clarify → /speckit.plan → /speckit.tasks → /speckit.implement
```

#### Step-by-Step:

1. **Specify** (`/speckit.specify`)
   - Create a detailed feature specification
   - Define user stories with priorities (P1, P2, P3)
   - Document acceptance criteria and edge cases
   - Output: `specs/###-feature-name/spec.md`

2. **Clarify** (`/speckit.clarify`)
   - Review specification for completeness
   - Resolve any ambiguities or missing details
   - Ensure all requirements are testable
   - Output: Updated `spec.md` with clarifications

3. **Plan** (`/speckit.plan`)
   - Research technical approach
   - Design data models and API contracts
   - Create implementation plan
   - Output: `specs/###-feature-name/plan.md`, `data-model.md`, `contracts/`, `quickstart.md`

4. **Tasks** (`/speckit.tasks`)
   - Break plan into actionable tasks
   - Organize by user story for independent implementation
   - Mark parallelizable tasks with [P]
   - Output: `specs/###-feature-name/tasks.md`

5. **Implement** (`/speckit.implement`)
   - Execute tasks following TDD discipline
   - Write tests first, ensure they fail, then implement
   - Commit after each logical task group
   - Validate with quickstart.md

---

### Workflow 2: Bug Fix / Enhancement

When a developer wants to **fix a bug or enhance an existing feature**, follow this sequence:

```
/speckit.clarify → /speckit.plan → /speckit.tasks → /speckit.implement
```

#### Step-by-Step:

1. **Clarify** (`/speckit.clarify`)
   - Identify the bug or enhancement requirement
   - Document the current behavior vs expected behavior
   - Determine impact on existing specifications
   - Output: Updated `spec.md` with change documentation

2. **Plan** (`/speckit.plan`)
   - Analyze affected components
   - Design minimal change approach
   - Update API contracts if needed
   - Output: Updated `plan.md` with change implementation details

3. **Tasks** (`/speckit.tasks`)
   - Create tasks for the fix/enhancement
   - Include regression tests
   - Output: Updated `tasks.md`

4. **Implement** (`/speckit.implement`)
   - Implement the fix/enhancement
   - Ensure all tests pass
   - Verify no regression in existing functionality
   - Update documentation if needed

---

### Workflow 3: Analysis & Quality Assurance

For ongoing quality checks and analysis:

```
/speckit.analyze → /speckit.checklist
```

1. **Analyze** (`/speckit.analyze`)
   - Review codebase for compliance with constitution
   - Identify technical debt or improvement opportunities
   - Output: Analysis report

2. **Checklist** (`/speckit.checklist`)
   - Create verification checklist
   - Track completion of quality gates
   - Output: `specs/###-feature-name/checklist.md`

---

## Speckit Command Reference

| Command | Purpose | When to Use |
|---------|---------|-------------|
| `/speckit.specify` | Create feature specification | New features |
| `/speckit.clarify` | Review and clarify spec | Before planning |
| `/speckit.plan` | Create implementation plan | After specification |
| `/speckit.tasks` | Break plan into tasks | After planning |
| `/speckit.implement` | Execute tasks | After task creation |
| `/speckit.analyze` | Codebase analysis | Quality checks |
| `/speckit.checklist` | Create verification list | QA process |
| `/speckit.git.feature` | Create feature branch | Before starting work |
| `/speckit.git.commit` | Commit changes | After completing work |

---

## Agent Rules

### MUST Follow:

1. **Constitution Compliance**: All work MUST align with `.specify/memory/constitution.md`
2. **TDD Discipline**: Tests MUST be written before implementation
3. **Security First**: Cryptographic operations MUST use vetted libraries
4. **API First**: Backend APIs MUST be defined before frontend implementation
5. **Documentation**: All features MUST include user-facing documentation

### MUST NOT:

1. **Prohibited Dependencies**: NEVER use TanStack libraries or vulnerable dependencies
2. **Skip Tests**: NEVER merge code without passing tests
3. **Unsafe Rust**: NEVER use `unsafe` without documented justification
4. **Hardcode Secrets**: NEVER commit secrets, keys, or credentials

### SHOULD:

1. **Follow Conventions**: Mimic existing code style and patterns
2. **Small Commits**: Make atomic, well-documented commits
3. **Validate Early**: Run linters and type checkers frequently
4. **Ask for Clarification**: When requirements are unclear, ask before implementing

---

## File Structure Reference

```
crypto-pocket-butler/
├── .opencode/              # Agent commands and configuration
│   └── command/            # Speckit workflow commands
├── .specify/               # Specification and planning templates
│   ├── memory/             # Constitution and project memory
│   ├── templates/          # Document templates
│   └── extensions/         # Git and other extensions
├── specs/                  # Feature specifications
│   └── ###-feature-name/   # Individual feature specs
│       ├── spec.md         # Feature specification
│       ├── plan.md         # Implementation plan
│       ├── tasks.md        # Task breakdown
│       └── contracts/      # API contracts
├── backend/                # Rust backend (future)
│   └── src/
└── frontend/               # Next.js frontend (future)
    └── src/
```

---

## Quick Reference

**Starting a new feature?**
→ Run `/speckit.specify` to create the specification

**Found a bug?**
→ Run `/speckit.clarify` to document the issue

**Ready to code?**
→ Ensure you have `spec.md`, `plan.md`, and `tasks.md` in `specs/###-feature-name/`

**Need to commit?**
→ Run `/speckit.git.commit` to auto-commit with proper message

**Want to analyze code?**
→ Run `/speckit.analyze` for compliance check

---

<!-- SPECKIT START -->
For additional context about technologies to be used, project structure,
shell commands, and other important information, read the current plan
<!-- SPECKIT END -->
