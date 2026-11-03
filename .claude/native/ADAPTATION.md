# Native stage adaptation

The stage bodies under `stages/` are copied byte-for-byte from **AI-DLC 2.11.0** (`awslabs/aidlc-workflows`, tag `v2.11.0`, commit `6a378b53c0a4fe0641ed7d8de8dfff94264d5b6a`). They were written for the native runtime, whose tools live in the harness directory. my-aidlc runs its own Node engine, so this file is the bridge the conductor reads when a stage body names a native call.

## Engine calls

`/aidlc` expands to the harness invocation (`/aidlc` on PI Agent, Claude Code, and DeepSeek; `$aidlc` on Codex). my-aidlc keeps the same subcommand surface under its own binary, so read every native engine call as:

| Native call in stage body | Occurs | Read as |
| --- | --- | --- |
| `/aidlc engine orchestrate` | 36 | `my-aidlc engine orchestrate` |
| `/aidlc engine log` | 6 | `my-aidlc engine log` |
| `/aidlc engine state` | 6 | `my-aidlc engine state` |
| `/aidlc engine workspace` | 5 | `my-aidlc engine workspace` |
| `/aidlc engine gen` | 2 | `my-aidlc engine gen` |
| `/aidlc engine recompose` | 2 | `my-aidlc engine recompose` |
| `/aidlc doctor` | 1 | `my-aidlc doctor` |
| `/aidlc engine config` | 1 | `my-aidlc engine config` |
| `/aidlc engine now` | 1 | `my-aidlc engine now` |
| `/aidlc engine testing-posture` | 1 | `my-aidlc engine testing-posture` |

## Runtime tools

These helpers belong to the native runtime and are **not** shipped by my-aidlc. Treat each as reference-only context: take the equivalent fact from the stage's `inputs`, from the directive, or by asking the human. `my-aidlc doctor` reports the same list.

| Native helper | Occurs | Status |
| --- | --- | --- |
| `bun .claude/tools/aidlc-utility.ts` | 10 | reference only |
| `bun .claude/tools/aidlc-log.ts` | 5 | reference only |
| `bun .claude/tools/aidlc-testing-posture.ts` | 5 | reference only |
| `bun .claude/tools/aidlc-state.ts` | 4 | reference only |

## Not carried over

Native ceremony that my-aidlc's engine does not implement is still named inside the bodies: sensors (`## Sensors`), the learnings ritual (`## Learn`), summary confirmation, plan approval, collaborators, plugins, and Bolt/worktree swarms. my-aidlc's own `core/protocols/stage-protocol.md` governs those concerns; the native sections stay so the method remains faithful and auditable.

