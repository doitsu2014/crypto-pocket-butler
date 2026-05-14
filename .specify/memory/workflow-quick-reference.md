# Crypto Pocket Butler - Developer Workflow Quick Reference

## 🚀 Starting a New Feature

```bash
# Step 1: Create specification
/speckit.specify "Your feature description"

# Step 2: Clarify requirements
/speckit.clarify

# Step 3: Create implementation plan
/speckit.plan

# Step 4: Break into tasks
/speckit.tasks

# Step 5: Implement with TDD
/speckit.implement
```

**Output**: `specs/###-feature-name/` with `spec.md`, `plan.md`, `tasks.md`

---

## 🐛 Fixing a Bug / Enhancement

```bash
# Step 1: Document the issue
/speckit.clarify

# Step 2: Plan the fix
/speckit.plan

# Step 3: Create fix tasks
/speckit.tasks

# Step 4: Implement fix
/speckit.implement
```

**Output**: Updated specification and implementation

---

## 🔍 Quality Check

```bash
# Analyze codebase
/speckit.analyze

# Create verification checklist
/speckit.checklist
```

---

## 📋 Command Reference

| Command | Purpose |
|---------|---------|
| `/speckit.specify` | Create new feature specification |
| `/speckit.clarify` | Review and clarify requirements |
| `/speckit.plan` | Create implementation plan |
| `/speckit.tasks` | Break plan into tasks |
| `/speckit.implement` | Execute tasks |
| `/speckit.analyze` | Analyze codebase |
| `/speckit.checklist` | Create verification checklist |
| `/speckit.git.feature` | Create feature branch |
| `/speckit.git.commit` | Commit changes |

---

## ⚠️ Important Rules

1. **ALWAYS** follow TDD: Write tests first, then implement
2. **NEVER** use TanStack libraries (security vulnerability)
3. **NEVER** commit secrets or credentials
4. **ALWAYS** run `/speckit.clarify` before planning
5. **ALWAYS** check constitution compliance

---

## 📁 File Structure

```
specs/
└── ###-feature-name/
    ├── spec.md         # Feature specification
    ├── plan.md         # Implementation plan
    ├── data-model.md   # Database schema
    ├── contracts/      # API contracts
    ├── tasks.md        # Task breakdown
    └── checklist.md    # QA checklist
```

---

## 🔗 Key Documents

- **Constitution**: `.specify/memory/constitution.md`
- **Full Workflows**: `.specify/memory/workflows.md`
- **Agent Guidance**: `AGENTS.md`
- **Templates**: `.specify/templates/`

---

## 💡 Tips

- Use `/speckit.git.feature` to create feature branch before starting
- Use `/speckit.git.commit` to auto-commit with proper message format
- Run `/speckit.analyze` periodically to check compliance
- Check `tasks.md` for parallelizable tasks marked with `[P]`
