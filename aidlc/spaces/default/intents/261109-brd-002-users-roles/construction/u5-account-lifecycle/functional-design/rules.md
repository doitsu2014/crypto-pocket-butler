# Business Rules — u5-account-lifecycle

**Upstream:** `requirements.md` (FR17.1–FR17.5), `stories.md` (US4.1–US4.5, US7.2), `contract-summary.md` (C3, C7), `components.md` (AccountLifecycle).

## Source of truth

```yaml
rules:
  - id: BR5.1
    statement: A status moves only along a permitted edge of the state machine; every other transition is refused.
    category: policy
    applies_to: User.status
    trigger: any status change request
    logic: "permitted edges: active→disabled, disabled→active, active→pending_deletion, pending_deletion→active, pending_deletion→deleted, pending→active; anything else is 409"
    violation_behaviour: refused with 409 and no state change; the attempt is audited
    source: AC4.1.2

  - id: BR5.2
    statement: Only an admin disables or restores an account, and both require a recorded reason.
    category: authorization
    applies_to: StatusTransition
    trigger: an admin disable or restore
    logic: "IF the caller is not admin THEN 403; IF the reason is absent or blank THEN 422; ELSE transition and record the reason"
    violation_behaviour: refused; a disabled user can never restore themselves
    source: AC4.2.1, AC4.2.2

  - id: BR5.3
    statement: A user's deletion request moves the account to pending_deletion and opens a recovery window ending at midnight UTC on the 30th calendar day.
    category: policy
    applies_to: DeletionRequest
    trigger: a user requests deletion
    logic: "insert one open request with requestedAt=now and recoverableUntil = midnight UTC of the 30th day; transition active→pending_deletion in the same transaction"
    violation_behaviour: a second open request for the same user is refused (409); nothing partial is written
    source: AC4.3.1

  - id: BR5.4
    statement: A user may restore within the window, moving pending_deletion back to active and cancelling the request.
    category: policy
    applies_to: DeletionRequest
    trigger: the user signs in to restore before recoverableUntil
    logic: "IF now < recoverableUntil THEN set cancelledAt, transition pending_deletion→active; ELSE refuse, the account is due for erasure"
    violation_behaviour: a restore after the deadline is refused with 410
    source: AC4.3.1

  - id: BR5.5
    statement: When the window ends, the expiry job moves the account to deleted and erases its personal data.
    category: policy
    applies_to: DeletionRequest, User
    trigger: the scheduled expiry job finds an open request past recoverableUntil
    logic: "transition pending_deletion→deleted, then run the erasure cascade through the retention role, then set erasedAt; one audit entry records the run"
    violation_behaviour: a row left pending past its deadline without a run is a defect surfaced by the job's own metrics
    source: AC4.3.2

  - id: BR5.6
    statement: Erasure is idempotent — running it twice leaves the same end state and reports success.
    category: constraint
    applies_to: the erasure cascade
    trigger: a repeated erasure run
    logic: "each step deletes by user id and tolerates an already-absent row; a request already erasedAt is skipped"
    violation_behaviour: a second run that fails or changes anything else is a defect
    source: AC4.5.2

  - id: BR5.7
    statement: A pending_deletion account is read-only: it may sign in, restore, and read its own data, and may not connect a new source or write.
    category: policy
    applies_to: the account in pending_deletion
    trigger: any authenticated request from that account
    logic: "IF status is pending_deletion AND the operation is not restore or a read THEN refuse with 403 naming the restore path"
    violation_behaviour: a write or a new source connection succeeding is a defect
    source: AC4.3.3, AC4.4.1

  - id: BR5.8
    statement: A disabled or deleted account is denied every authenticated request, whatever token it presents.
    category: policy
    applies_to: every request
    trigger: any authenticated request
    logic: "status denial is evaluated by u2's decision on the status u5 maintains; u5 never exempts itself"
    violation_behaviour: a request succeeding for a disabled or deleted account is a defect
    source: AC4.5.1

  - id: BR5.9
    statement: Every status change is audited, durably, with actor, target, previous status, new status, reason, timestamp, and outcome.
    category: constraint
    applies_to: StatusTransition
    trigger: every transition
    logic: "insert the transition row and a status_audit_outbox row in the same transaction as the status write; the worker loop drains the outbox to C9 with an idempotency key. An accepted transition always has a durable audit intent; delivery is a bounded-lag SLI, not a promise C9 can break"
    violation_behaviour: a transition whose outbox row cannot be written is refused, so no status change is ever accepted without its audit intent
    source: AC7.2.1, AC7.2.2

  - id: BR5.10
    statement: Until BRD-017's legal-hold register and a statutory retention schedule exist, no account is exempt from erasure — an accepted deviation from FR17.4 and AC4.4.2, not a satisfied rule.
    category: policy
    applies_to: the expiry job
    trigger: a due erasure
    logic: "the job erases every due account; when the register or a retention schedule lands it gains a hold check, and the exception is recorded on the run's audit entry"
    violation_behaviour: skipping a due account with no recorded hold is a defect; but erasing a record that a future retention schedule would have kept is a compliance risk this rule does not remove
    source: AC4.3.2, AC4.4.2
```

## Summary

| Rule | Statement | Category | Source |
|---|---|---|---|
| BR5.1 | Only permitted edges | policy | AC4.1.2 |
| BR5.2 | Admin-only disable/restore, with a reason | authorization | AC4.2.1–2 |
| BR5.3 | Request opens a 30-day window | policy | AC4.3.1 |
| BR5.4 | Restore within the window | policy | AC4.3.1 |
| BR5.5 | Expiry job deletes and erases | policy | AC4.3.2 |
| BR5.6 | Erasure is idempotent | constraint | AC4.5.2 |
| BR5.7 | Pending deletion is read-only | policy | AC4.3.3, AC4.4.1 |
| BR5.8 | Disabled and deleted are denied | policy | AC4.5.1 |
| BR5.9 | Every transition audited | constraint | AC7.2.1–2 |
| BR5.10 | No legal-hold exemption yet | policy | AC4.3.2, AC4.4.2 |
