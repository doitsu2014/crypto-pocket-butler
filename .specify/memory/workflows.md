# Development Workflows

This document defines the standard workflows for the Crypto Pocket Butler project.

## Overview

All development work follows the **Speckit** workflow system. Speckit provides structured commands to ensure consistent, high-quality implementation.

## Workflow Categories

### 1. New Feature Development

**Trigger**: Developer wants to implement a new feature or user story

**Sequence**:
```
/speckit.specify → /speckit.clarify → /speckit.plan → /speckit.tasks → /speckit.implement
```

**Detailed Steps**:

| Step | Command | Input | Output | Purpose |
|------|---------|-------|--------|---------|
| 1 | `/speckit.specify` | Feature description | `specs/###/spec.md` | Create detailed specification |
| 2 | `/speckit.clarify` | spec.md | Updated spec.md | Resolve ambiguities |
| 3 | `/speckit.plan` | spec.md | `plan.md`, `data-model.md`, `contracts/` | Design implementation approach |
| 4 | `/speckit.tasks` | plan.md | `tasks.md` | Break into actionable tasks |
| 5 | `/speckit.implement` | tasks.md | Working code | Execute with TDD |

**Example Flow**:
```
Developer: "I want to add a feature for tracking NFT portfolios"

1. /speckit.specify "NFT portfolio tracking"
   → Creates specs/001-nft-portfolio/spec.md

2. /speckit.clarify
   → Reviews and clarifies requirements in spec.md

3. /speckit.plan
   → Creates plan.md, data-model.md, contracts/

4. /speckit.tasks
   → Creates tasks.md with task breakdown

5. /speckit.implement
   → Implements tasks following TDD
```

---

### 2. Bug Fix / Enhancement

**Trigger**: Developer wants to fix a bug or enhance existing functionality

**Sequence**:
```
/speckit.clarify → /speckit.plan → /speckit.tasks → /speckit.implement
```

**Detailed Steps**:

| Step | Command | Input | Output | Purpose |
|------|---------|-------|--------|---------|
| 1 | `/speckit.clarify` | Bug/enhancement description | Updated spec.md | Document the issue/change |
| 2 | `/speckit.plan` | Updated spec.md | Updated plan.md | Plan the fix/enhancement |
| 3 | `/speckit.tasks` | Updated plan.md | Updated tasks.md | Create fix tasks |
| 4 | `/speckit.implement` | tasks.md | Fixed code | Implement with regression tests |

**Example Flow**:
```
Developer: "The portfolio balance calculation is incorrect for ERC-20 tokens"

1. /speckit.clarify
   → Documents bug in spec.md, identifies affected components

2. /speckit.plan
   → Updates plan.md with fix approach

3. /speckit.tasks
   → Creates tasks including regression tests

4. /speckit.implement
   → Fixes bug, ensures all tests pass
```

---

### 3. Quality Assurance & Analysis

**Trigger**: Developer wants to check code quality or compliance

**Sequence**:
```
/speckit.analyze → /speckit.checklist
```

**Detailed Steps**:

| Step | Command | Input | Output | Purpose |
|------|---------|-------|--------|---------|
| 1 | `/speckit.analyze` | Codebase | Analysis report | Review compliance |
| 2 | `/speckit.checklist` | Analysis results | `checklist.md` | Track quality gates |

---

## Agent Guidance

### When to Use Each Workflow

| Scenario | Workflow | Starting Command |
|----------|----------|------------------|
| New user story | New Feature | `/speckit.specify` |
| New feature request | New Feature | `/speckit.specify` |
| Bug report | Bug Fix | `/speckit.clarify` |
| Performance issue | Bug Fix | `/speckit.clarify` |
| UI enhancement | Bug Fix | `/speckit.clarify` |
| Code review | QA | `/speckit.analyze` |
| Pre-release check | QA | `/speckit.analyze` |

### Workflow Transitions

```
New Feature Workflow:
  specify ──→ clarify ──→ plan ───→ tasks ──→ implement
     │          │          │         │          │
     └──────────┴──────────┴─────────┴──────────┘
              (iterate if needed)

Bug Fix Workflow:
  clarify ──→ plan ──→ tasks ──→ implement
     │          │         │          │
     └──────────┴─────────┴──────────┘
              (iterate if needed)
```

### Parallel Execution Opportunities

Tasks marked with `[P]` in `tasks.md` can be executed in parallel:

- Different user stories can be implemented in parallel
- Tests for different components can run in parallel
- Frontend and backend work can proceed in parallel (after API contract is defined)

---

## Integration with Git

### Branch Strategy

1. **Feature Branch**: Created via `/speckit.git.feature` before starting work
2. **Commits**: Auto-committed via `/speckit.git.commit` after each logical task
3. **Pull Request**: Created after all tasks complete and tests pass

### Commit Message Format

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

---

## Quality Gates

Before marking any workflow as complete, ensure:

- [ ] All tests pass
- [ ] Code coverage meets minimum (80% for business logic)
- [ ] Linting passes (clippy, ESLint)
- [ ] Type checking passes
- [ ] Documentation is updated
- [ ] Security review completed (for crypto operations)
- [ ] Constitution compliance verified

---

## Troubleshooting

### Common Issues

**Issue**: Specification is unclear
**Solution**: Run `/speckit.clarify` to identify and resolve ambiguities

**Issue**: Tasks are too large
**Solution**: Run `/speckit.tasks` again to break down further

**Issue**: Tests failing after implementation
**Solution**: Review test output, fix implementation, ensure TDD discipline

**Issue**: Constitution violation detected
**Solution**: Review `.specify/memory/constitution.md`, adjust implementation

---

## Additional Resources

- Constitution: `.specify/memory/constitution.md`
- Templates: `.specify/templates/`
- Commands: `.opencode/command/`
- Extensions: `.specify/extensions/`
