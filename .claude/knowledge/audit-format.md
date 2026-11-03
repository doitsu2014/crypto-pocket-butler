# Audit Event Format

The audit log is an append-only JSONL file at `aidlc/audit.log`. Every row is
one JSON object on one line. State transitions are tool-owned: the engine
writes every row. Never hand-write or hand-edit audit rows.

## Row shape

```json
{
  "ts": "2024-01-01T00:00:00.000Z",
  "event": "STAGE_APPROVED",
  "space": "default",
  "intent": "240101-inventory-api",
  "phase": "initialization",
  "stage": "workspace-scaffold",
  "result": "approved",
  "userInput": "Approve",
  "reason": null
}
```

## Event taxonomy

| Event | When |
| --- | --- |
| `WORKSPACE_INITIALIZED` | The workspace is created or scaffolded. |
| `INTENT_CREATED` | A new intent begins. |
| `SCOPE_SELECTED` | A workflow profile is chosen for an intent. |
| `STAGE_STARTED` | A stage directive is issued for the first time. |
| `STAGE_AWAITING_APPROVAL` | The stage artifacts are presented at the gate. |
| `STAGE_REJECTED` | The human requested changes. |
| `STAGE_REVISED` | The stage was revised after requested changes. |
| `STAGE_APPROVED` | The human approved the stage. |
| `STAGE_AUTO_APPROVED` | YOLO mode auto-satisfied the gate with the recommended answer. |
| `STAGE_COMPLETED` | The stage completed without a human gate. |
| `STAGE_SKIPPED` | A conditional stage did not apply. |
| `PHASE_AWAITING_APPROVAL` | A phase that requires review reached its last stage and is presented for review. |
| `PHASE_APPROVED` | The human approved the phase review. |
| `PHASE_REJECTED` | The human requested changes to the phase. |
| `CHANGE_NOTICE` | An input changed after approval. |
| `WORKFLOW_PARKED` | The workflow was parked. |
| `WORKFLOW_COMPLETED` | The workflow reached its end. |

## Rules

- One row per event; never rewrite history.
- `event` must come from the taxonomy above.
- The audit log is append-only and committed with the workspace.
- If an event is missing for a transition, the transition did not happen.
