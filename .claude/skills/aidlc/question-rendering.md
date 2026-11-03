# Question Rendering

How the conductor renders a question to the human on each harness.

## File-backed questions (default)

Write `<stage>-questions.md` and ask the human to answer inline with
`[Answer]: <letter> <text>`. This is the portable form and the source of truth
on every harness.

## Structured questions (1–3 simple choices)

Where the harness supports a structured picker, present the options natively
and write the chosen answer back into the questions file. The file remains
authoritative.

| Harness | Structured mechanism |
| --- | --- |
| Claude Code | `AskUserQuestion` |
| Codex CLI | `request_user_input` |
| PI Agent | interactive question in the conversation |

## Rules

- Always include an equivalent `X. Other` escape in the file-backed form.
- Never treat a non-answer as an answer.
- The approval gate is never merged into a question message; it is its own turn.
