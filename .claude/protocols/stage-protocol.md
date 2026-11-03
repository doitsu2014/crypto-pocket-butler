# Stage Protocol

This protocol is mandatory for every stage in every phase. It defines the
backing loop, the question flow, the approval gate, and the completion report.

## 1. The forwarding loop

You are the conductor. Your entire control structure is:

```
Loop:
  1. directive = run `my-aidlc orchestrate next $ARGUMENTS`
  2. act on directive.kind (see below)
  3. after stage work, run `my-aidlc orchestrate report --stage <slug> --result <outcome>`
  4. resume at step 1 with the directive the report returns, or stop when it
     returns done / error / parked
```

The engine owns all between-stage routing: scope resolution, sequencing,
resume, gate status, and workflow completion. You never re-derive routing in
prose. You own the quality of execution inside the move the engine names.

## 2. Directive kinds

| kind | What you do |
| --- | --- |
| `print` | Do exactly what `directive.message` says, print its output, and stop. |
| `error` | Stop. Print `directive.message` verbatim. Do not retry or work around it. |
| `done` | The workflow is complete. Present the completion summary and stop. |
| `parked` | The workflow is parked. Tell the user how to resume and stop. |
| `ask` | Render `directive.question`, wait for the human, then follow `directive.route`. |
| `run-stage` | Run the named stage body, then report its outcome. |

## 3. Question flow

Use a file-backed question flow for any stage that needs input. The stage's
directive carries `question_budget: { min, max, source }`; ask no more than
`max` and no fewer than `min` (see `question-flow.md`). When `max` is 0, skip
questions and generate directly.

1. Create `<record>/<phase>/<stage>/<stage>-questions.md`.
2. Number the questions. Under each, provide lettered options `A`–`E` plus
   `X` (Other).
3. Ask the human to answer inline with `[Answer]: <letter> <text>`.
4. Detect ambiguity and contradictions. Resolve them inside the stage; never
   carry an open contradiction forward.
5. The questions file is the source of truth for the answers.

For one to three simple options, a structured question UI is also acceptable.
All three flows (guided, self-guided, chat) converge on the questions file.

## 4. Approval gate

Every executed stage ends at a human approval gate.

- Present the artifacts produced, their paths, and a concise summary.
- Offer: **Approve**, **Request Changes**, and (where relevant) **Skip**.
- On Approve, report `--result approved`.
- On Request Changes, report `--result rejected --reason "<feedback>"`, then
  run the Keep / Modify / Redo loop and report `--result revised`.
- The gate is a separate turn: never render it in the same message as a
  question that is still awaiting an answer.

### Keep / Modify / Redo

| Choice | Meaning |
| --- | --- |
| Keep | Accept the artifact as-is. |
| Modify | Edit the existing artifact in place. |
| Redo | Discard partial artifacts and re-run the stage. |

## 5. Completion report

Report every lifecycle outcome exactly once:

```
my-aidlc orchestrate report --stage <slug> --result <outcome> [--user-input "..."] [--reason "..."]
```

Valid results: `in-progress`, `awaiting-approval`, `approved`, `rejected`,
`revised`, `completed`, `skipped`.

A conditional stage that does not apply reports `--result skipped` with a
non-blank `--reason`. Never mark a stage complete that you did not run.

## 6. Talking to the user

Speak as a teammate helping build software, not as a framework narrating
itself. These words stay internal: engine, directive, dispatch, conductor,
harness, scope grid, steering. Report substance — questions, gates, artifacts,
errors — and stay quiet between steps.

## 6. Execution modes: normal and YOLO

The project has one execution mode, resolved scope > project > default:

| Mode | Questions | Approval gates | Use for |
| --- | --- | --- | --- |
| `normal` (default) | Asked, bounded by `question_budget` | Presented to the human | Anything that ships |
| `yolo` | Skipped; pick the recommended answer | Auto-satisfied | Demos, POCs, trusted automation |

Every `run-stage` directive carries `execution_mode`, `auto_approve`, and
`answer_policy`.

In YOLO mode:

- `question_budget` is `{ min: 0, max: 0 }` and `question-flow` is not loaded.
- `answer_policy` is `recommended`: where a stage would ask, pick the option the
  artifact recommends (or the first listed option) and record it as a
  `Recommendation` in the stage notes.
- `auto_approve` is true unless the stage requires review: do **not** present
  the approval gate. Produce the artifacts, report `awaiting-approval`, and the
  engine records `STAGE_AUTO_APPROVED` and advances on its own.
- Reviewers still run; apply their blocking findings before reporting.

### Review checkpoints

A stage can opt out of auto-approval with `review: required` in its
frontmatter. Such a stage always presents the human approval gate, even in
YOLO mode: the directive sets `auto_approve` to `false` and
`review_required` to `true`. Keep auto-picking recommended answers where the
stage asks, but stop at the gate and present the artifacts it names. The
checkpoint covers one stage; every other stage still auto-approves.

### Phase review

A phase can require a review of its whole output. The five phases are native, so
their display name and gate flag come from the engine's projection metadata at
`native/phases/<phase>.md`; set `review: required` there. When the last
applicable stage of that phase completes, the engine does not cross into the
next phase. It returns an `ask` directive with `ask_type: "phase-review"` and
`route: "report-phase"` that lists every stage and artifact in the phase.

Present the phase artifacts, then:

- **Approve phase** →
  `my-aidlc orchestrate report --phase <phase> --result approved`. The engine
  records `PHASE_APPROVED` and continues into the next phase.
- **Request Changes** →
  `my-aidlc orchestrate report --phase <phase> --result rejected --reason "<feedback>"`.
  The engine records `PHASE_REJECTED`, re-opens every stage in the phase, and
  restarts it. Run the phase again with the feedback; it gates again when the
  phase completes.

Phase reviews apply in both normal and YOLO mode. A project can also gate every
phase with the `reviewRequired` toggle (`my-aidlc config --review-required true`),
and a scope can override it with `review_required:`. `my-aidlc status` marks the
phase awaiting review.

YOLO never skips stage work: every stage still runs and still writes its
artifacts. It removes the human in the loop, not the work. The audit log records
every auto-approval, so an unattended run remains auditable.

> Governance note: YOLO trades the AIDLC rule "humans own merges" for speed.
> Use it deliberately, on work you can afford to redo, and review the audit log
> afterwards.

## 7. Context loading

Before running a stage body:

1. Read the lead agent persona from the path named in the directive. The
   Initialization stages report `lead_agent: orchestrator` and no agent file:
   you are the lead there.
2. Read the stage file (`directive.stage_file`) — a native stage body.
3. Read each existing path in `directive.consumes`.
4. Read the active scope file when the directive names one. The team authors it
   in `aidlc/spaces/<space>/scopes/`; its `include` list is the workflow.

Agent names alone are not loaded context. Read the files.

## 8. Where the methodology comes from

The five phases, 33 stages, and 14 agents are **native AWS AI-DLC**, not
my-aidlc originals. The engine reads them from `native/` in the harness
directory, generated by `my-aidlc update` from the release pinned in
`aidlc-version.json`. Read `native/ADAPTATION.md` before acting on a native
engine call: it maps the native runtime's invocations onto my-aidlc's, and it
names the helpers that are reference-only. Never edit `native/`; `my-aidlc
doctor` fails when it drifts from the pin.
