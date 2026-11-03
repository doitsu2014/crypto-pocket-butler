# Question Flow

The question flow is how a stage gathers the human decisions it needs. It is
file-backed so answers survive context loss and can be reviewed.

## Format

```markdown
# <Stage> Questions

## Q1: <question>

A. <option>
B. <option>
C. <option>
D. <option>
E. <option>
X. Other (describe)

[Answer]:

## Q2: <question>

...
```

## Rules

- Every question has 2–5 lettered options plus `X. Other`.
- The human writes `[Answer]: C` (optionally with extra text).
- Never invent an answer or infer one from silence.
- If an answer is ambiguous or contradicts an earlier answer, ask a follow-up
  and record both the question and the resolution.

## Question budget

Every `run-stage` directive carries `question_budget: { min, max, source }`. The
conductor must honour it:

- Ask **at most `max`** clarifying questions. Combine closely related prompts.
- Ask **at least `min`** questions unless the stage genuinely needs none and
  `min` is 0. Reserve the minimum for decisions that change the artifact.
- When `max` is `0`, ask no questions and generate the artifacts directly. The
  directive then omits the `question-flow` module.
- The budget applies to the stage's clarifying questions, never to the approval
  gate, which is always exactly one decision.

Precedence, most specific first: **stage** `question_budget` → **scope**
`question_budget` → project config `questionBudget` → built-in default
(`min: 0, max: 5`). The `source` field says which layer won.

Configure the project default:

```bash
my-aidlc config --questions-min 1 --questions-max 3   # tighter
my-aidlc config --questions-max 0                     # skip questions entirely
my-aidlc config                                       # show the current budget
```

Override per scope or per stage in frontmatter:

```yaml
question_budget:
  min: 0
  max: 3
```

## Modes

| Mode | How it works |
| --- | --- |
| Guided | The conductor walks the human through each question interactively. |
| Self-guided | The human edits the questions file directly. |
| Chat | The human answers conversationally; the conductor writes the answers into the file. |

All three converge on the same file. The file is the source of truth.

## Contradiction analysis

Before generating artifacts:

1. Read every answered question.
2. Detect ambiguity (an answer that could mean two things) and contradiction
   (two answers that cannot both hold).
3. Resolve each one with the human and update the file.
4. Only then generate artifacts.
