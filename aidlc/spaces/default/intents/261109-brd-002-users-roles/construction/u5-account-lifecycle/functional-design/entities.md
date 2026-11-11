# Entities — u5-account-lifecycle

**Upstream:** `unit-of-work.md` (U5 `u5-account-lifecycle`), `components.md` (AccountLifecycle), `stories.md` (US4.1–US4.5, US7.2), `requirements.md` (FR17).

U5 owns the status machine, its transition record, and the deletion request. The `User` row itself
belongs to u1; u5 is the only unit allowed to transition its status.

## Source of truth

```yaml
entities:
  - name: StatusTransition
    description: One recorded change of a user's account status, append-only.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
        constraints: "event identifier; assigned on insert, never changed"
      - name: userId
        type: uuid
        required: true
        constraints: "the account whose status changed"
      - name: fromStatus
        type: enum
        required: true
        allowed: [pending, active, disabled, pending_deletion, deleted]
      - name: toStatus
        type: enum
        required: true
        allowed: [pending, active, disabled, pending_deletion, deleted]
        constraints: "must be a permitted edge of the state machine (rules.md)"
      - name: actor
        type: string
        required: true
        constraints: "a user id (the owner), an admin user id, or `system` for the expiry job; never a service name"
      - name: reason
        type: string
        required: false
        constraints: "required for an admin disable or restore; 1–500 characters; never carries a credential or token"
      - name: occurredAt
        type: timestamp
        required: true
        constraints: "database clock"
      - name: seq
        type: bigint
        required: true
        unique: true
        constraints: "monotonic sequence; orders transitions that share a timestamp"
    entity_constraints:
      - "Append-only: no UPDATE or DELETE for the application role; a transition is evidence."
      - "The newest transition per user is the account's current status evidence, but the status value read by other units stays on the `users` row (u1 owns the column; u5 writes it through the transition path only)."
    relationships:
      - to: User
        cardinality: many-to-one
        direction: "each transition belongs to one user"

  - name: DeletionRequest
    description: A user's request to delete their account, and the recovery window it opens.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: userId
        type: uuid
        required: true
        constraints: "one open request per user; a second request while one is open is refused"
      - name: requestedAt
        type: timestamp
        required: true
        constraints: "database clock, the moment the request committed"
      - name: recoverableUntil
        type: timestamp
        required: true
        constraints: "midnight UTC on the 30th calendar day after requestedAt"
      - name: cancelledAt
        type: timestamp
        required: false
        constraints: "set when the user restores; a cancelled request is kept as evidence"
      - name: erasedAt
        type: timestamp
        required: false
        constraints: "set by the erasure job when the cascade completes"
    entity_constraints:
      - "At most one open request per user (cancelledAt and erasedAt both null)."
      - "A request is never deleted; it is cancelled or erased."
    relationships:
      - to: User
        cardinality: many-to-one
        direction: "each request belongs to one user"
```

## Summary

| Entity | Identifier | Purpose |
|---|---|---|
| StatusTransition | id (+ seq) | Append-only record of every status change, with actor and reason |
| DeletionRequest | id | The request, its recovery deadline, and its outcome |

## Decision recorded here

The status value has two representations on purpose: the `users.status` column (what every other
unit reads cheaply) and the transition log (the evidence of how it got there). U5 writes both in
one transaction, so the column can never disagree with the newest transition; a mismatch is a
defect, not a normal state. The alternative — deriving status from the log on every read — was
rejected because every authenticated request would then need a log query.

The deletion window is **midnight UTC on the 30th calendar day**, per this stage's answer: a
deadline that is predictable for the expiry job and for the person, rather than 720 hours landing
at an arbitrary minute. The cost is that some windows are a few hours longer than 30 days, recorded
here so the difference from FR17.3's "30 days" is explicit.
