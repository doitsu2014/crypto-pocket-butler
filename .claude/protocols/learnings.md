# Learnings Protocol

Learnings are how the workflow gets better at your project over time. A
learning is a short, durable statement that should change future behaviour.

## When it runs

Only when the active scope sets `learnings: on`. The engine lists `learnings`
in `directive.protocol_modules` when enabled.

## The diary

During a stage with learnings enabled, keep a diary at the stage's
`memory_path`:

```
<record>/<phase>/<stage>/memory.md
```

Append timestamped bullets under one of four canonical headings:

- **Interpretation** — how you read an ambiguous input
- **Deviation** — where you departed from the plan and why
- **Tradeoff** — a choice you made and what it cost
- **Open question** — something unresolved that later work should revisit

The diary is the only file you maintain by hand.

## The ritual

Before the approval gate:

1. Read the diary and surface candidate learnings.
2. Ask the human: admit / edit / discard each candidate. Always offer at least
   `Nothing to add` and `Add a note`.
3. Run the conflict check: a narrower rule that contradicts a broader rule in
   `memory/org.md` is rejected.
4. Persist accepted learnings to the appropriate memory layer:
   `project.md` for project-specific rules, `team.md` for team-wide rules,
   `org.md` for organisation-wide rules.

## Admission rules

- A learning must be actionable: it changes what someone does next time.
- A learning must be scoped: say where it applies.
- A learning must not contradict a broader standing rule.
- Learnings are advisory and additive; they never block the gate.
