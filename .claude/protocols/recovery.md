# Recovery Protocol

Use this protocol when a session resumes, when state looks inconsistent, or
when a change event invalidates an approval.

## On resume

1. Run `my-aidlc status` to read the active intent, scope, and stage.
2. Run `my-aidlc orchestrate next` to get the authoritative next move.
3. Re-read the stage file and the consumed artifacts before continuing.
4. Do not re-run already-approved stages unless the human asks.

## When an input changes after approval

If an artifact that a later stage consumed changes, the later stage's approval
is stale.

- Under `guard_policy: strict`, re-open the affected approval.
- Under `guard_policy: relaxed` or `off`, announce the change and continue.
- In all cases, record the change in the audit log.

## When state is inconsistent

Run `my-aidlc doctor`. It checks:

- the workspace exists and is initialized
- the active intent and scope resolve to real files
- the current stage exists in the compiled graph
- produced artifacts referenced by completed stages exist

If doctor reports a problem, fix the reported path or re-initialize rather
than hand-editing state. State transitions are tool-owned.

## Parking

A long workflow need not finish in one session. Run
`my-aidlc orchestrate park` to park cleanly at an inter-stage boundary. The
next session resumes with `my-aidlc orchestrate next --resume`.
