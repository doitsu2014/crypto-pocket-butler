# Guide: Using Workflows with Opencode Agent

This guide explains how to interact with the opencode agent for different development scenarios.

## Quick Start

### Scenario 1: Implement a New Feature

**What you say**:
```
"I want to implement a feature for tracking DeFi yields"
```

**What the agent does**:
1. Runs `/speckit.specify` to create specification
2. Runs `/speckit.clarify` to review requirements
3. Runs `/speckit.plan` to design implementation
4. Runs `/speckit.tasks` to break into tasks
5. Runs `/speckit.implement` to execute with TDD

**Your involvement**:
- Review and approve the specification
- Provide clarifications when asked
- Review implementation progress

---

### Scenario 2: Fix a Bug

**What you say**:
```
"The portfolio balance is showing incorrect values for ERC-20 tokens"
```

**What the agent does**:
1. Runs `/speckit.clarify` to document the bug
2. Runs `/speckit.plan` to plan the fix
3. Runs `/speckit.tasks` to create fix tasks
4. Runs `/speckit.implement` to fix with regression tests

**Your involvement**:
- Confirm the bug description
- Review the fix approach
- Verify the fix works

---

### Scenario 3: Enhance Existing Feature

**What you say**:
```
"I want to add price alerts to the portfolio tracking feature"
```

**What the agent does**:
1. Runs `/speckit.clarify` to document enhancement
2. Runs `/speckit.plan` to plan changes
3. Runs `/speckit.tasks` to create tasks
4. Runs `/speckit.implement` to implement

**Your involvement**:
- Define alert requirements
- Review implementation
- Test the feature

---

## Detailed Interaction Patterns

### Starting a New Feature (Full Workflow)

```
You: "I want to add a new feature for tracking NFT portfolios"

Agent: "I'll help you implement NFT portfolio tracking. Let me start by 
creating a specification."

[Agent runs /speckit.specify]

Agent: "I've created the specification at specs/001-nft-portfolio/spec.md. 
Please review the user stories and acceptance criteria. Shall I proceed 
with clarification?"

You: "Yes, proceed"

Agent: [Runs /speckit.clarify]
...
```

### Bug Fix (Abbreviated Workflow)

```
You: "The portfolio balance calculation is wrong for ERC-20 tokens"

Agent: "I'll help fix this bug. Let me first document the issue and 
understand the current behavior."

[Agent runs /speckit.clarify]

Agent: "I've documented the bug. The current implementation doesn't 
account for token decimals correctly. Shall I plan the fix?"

You: "Yes"

Agent: [Runs /speckit.plan]
...
```

### Enhancement (Update Existing Spec)

```
You: "I want to enhance the price tracking to include historical data"

Agent: "I'll help enhance the price tracking feature. Let me first 
review the current specification and plan the enhancement."

[Agent runs /speckit.clarify]

Agent: "I've reviewed the current spec. The enhancement will require:
1. New API endpoint for historical prices
2. Database schema update
3. Frontend chart component

Shall I proceed with planning?"

You: "Yes"

Agent: [Runs /speckit.plan]
...
```

---

## Agent Commands

### Direct Commands

You can also use commands directly:

```
/speckit.specify "Your feature description"
/speckit.clarify
/speckit.plan
/speckit.tasks
/speckit.implement
/speckit.analyze
/speckit.checklist
```

### Git Commands

```
/speckit.git.feature        # Create feature branch
/speckit.git.commit         # Commit changes
/speckit.git.validate       # Validate git state
/speckit.git.remote         # Manage remote
```

---

## Best Practices

### DO:

1. **Be specific** in your feature descriptions
2. **Review specifications** before proceeding
3. **Provide clarifications** when asked
4. **Test the implementation** after completion
5. **Use feature branches** for isolation

### DON'T:

1. **Skip the specification** phase
2. **Rush through clarification** - it prevents rework
3. **Ignore test failures** - fix them immediately
4. **Commit secrets** - use environment variables
5. **Use prohibited dependencies** (TanStack)

---

## Workflow Summary

| Scenario | Workflow | Commands |
|----------|----------|----------|
| New Feature | Full | specify → clarify → plan → tasks → implement |
| Bug Fix | Abbreviated | clarify → plan → tasks → implement |
| Enhancement | Update | clarify → plan → tasks → implement |
| Quality Check | Analysis | analyze → checklist |
| Quick Fix | Direct | clarify → implement |

---

## Getting Help

- **Workflow questions**: Ask the agent "What workflow should I use for X?"
- **Command help**: Ask "What does /speckit.plan do?"
- **Constitution questions**: Ask "What are the project principles?"
- **Technical questions**: Ask about Rust, Next.js, or Tailwind CSS

---

## Additional Resources

- **Full Workflows**: `.specify/memory/workflows.md`
- **Quick Reference**: `.specify/memory/workflow-quick-reference.md`
- **Constitution**: `.specify/memory/constitution.md`
- **Agent Guidance**: `AGENTS.md`
