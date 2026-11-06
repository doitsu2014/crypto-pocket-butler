# Audit — Entity Model

Source of truth: the YAML block below. The prose after it is a summary.

```yaml
entities:
  - name: AuditEvent
    description: >
      One append-only record of something that happened: who or what acted, what it did, what it
      acted on, when, in which context, and how it turned out. Written once through an insert-only
      role and never changed afterwards.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
        description: Assigned at append time and returned to the caller.
      - name: actorId
        type: uuid
        required: false
        references: User (Identity)
        description: The acting user or support agent. Absent for system-attributed events.
      - name: actorKind
        type: enum
        required: true
        allowed_values: [user, system, support_agent]
        default: system
        description: Who the actor is. `system` means the service itself acted.
      - name: action
        type: string
        required: true
        max_length: 120
        description: Dotted action name in noun.verb form, for example `asset.contract.added`.
      - name: target
        type: string
        required: true
        max_length: 240
        description: >
          The logical identifier the action applied to. Never a credential, a token, or a full
          wallet address.
      - name: occurredAt
        type: timestamp
        required: true
        description: When the action happened, as reported by the emitter.
      - name: recordedAt
        type: timestamp
        required: true
        default: now
        description: When the audit service stored the event.
      - name: context
        type: map
        required: false
        description: >
          Correlation identifier, request identifier, emitting service, and rule identifier. Never
          a record payload, a credential, or a token.
      - name: outcome
        type: enum
        required: true
        allowed_values: [success, denied, failed]
        description: How the action turned out.
      - name: sensitivity
        type: enum
        required: true
        allowed_values: [routine, security]
        default: routine
        description: Security events are the ones an operator reviews first.
      - name: idempotencyKey
        type: string
        required: false
        unique: true
        max_length: 128
        description: >
          Sent by the emitter in the `Idempotency-Key` request header, stored on the row so a retried
          append does not duplicate the event. Reused with a different payload it answers 409.
      - name: eventClass
        type: enum
        required: true
        allowed_values: [security, domain]
        default: domain
        description: >
          Security events must be stored before the caller's action commits; domain events are
          best-effort and the caller's action proceeds if the append fails.
      - name: legalBasis
        type: string
        required: false
        max_length: 120
        description: >
          The stated legal basis under which this record is kept past a deletion request. Set when
          the row is inserted, never by an update.
      - name: deletionRequestId
        type: uuid
        required: false
        description: The deletion request this record outlived, when it outlived one.
    constraints:
      - "Rows are inserted and read only; no update or delete is permitted to the service role."
      - "actorId is present when actorKind is user or support_agent, and absent when it is system."
      - "target and context carry no credential, token, or full wallet address."
    relationships:
      - entity: User
        owned_by: Identity
        cardinality: many-to-one
        direction: AuditEvent references User by actorId
        optional: true

  - name: SupportAccessRecord
    description: >
      One time-bound support session over a named user's metadata or tickets, with its reason,
      its second approver, and its outcome.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: supportAgentId
        type: uuid
        required: true
        references: User (Identity)
        description: The support agent who asked for the access.
      - name: subjectUserId
        type: uuid
        required: true
        references: User (Identity)
        description: The user whose metadata or tickets are in scope.
      - name: reason
        type: string
        required: true
        min_length: 10
        max_length: 500
        description: Why the access is needed, in the agent's own words.
      - name: secondApproverId
        type: uuid
        required: true
        references: User (Identity)
        description: A different operator who approved the session.
      - name: scope
        type: enum
        required: true
        allowed_values: [account_metadata, tickets]
        description: What the session may read. Portfolio contents and credentials are never in scope.
      - name: startedAt
        type: timestamp
        required: true
      - name: expiresAt
        type: timestamp
        required: true
        description: The instant the session stops granting access.
      - name: endedAt
        type: timestamp
        required: false
        description: Set when the session ends before its expiry.
      - name: state
        type: enum
        required: true
        allowed_values: [requested, active, expired, revoked, denied]
        default: requested
        description: >
          `requested` while the second approver has not answered, `active` once approved, then
          `expired`, `revoked`, or `denied`. Only `active` grants any read.
      - name: approvedAt
        type: timestamp
        required: false
        description: When the second approver agreed.
      - name: outcome
        type: enum
        required: false
        allowed_values: [approved, denied, revoked, expired]
        description: Set when the record reaches a terminal state; absent while requested or active.
    constraints:
      - "secondApproverId differs from supportAgentId."
      - "expiresAt is later than startedAt and no more than eight hours after it."
      - "state is expired once now is past expiresAt, whether or not anything wrote that."
    relationships:
      - entity: User
        owned_by: Identity
        cardinality: many-to-one
        direction: SupportAccessRecord references User as supportAgentId
      - entity: User
        owned_by: Identity
        cardinality: many-to-one
        direction: SupportAccessRecord references User as subjectUserId
      - entity: User
        owned_by: Identity
        cardinality: many-to-one
        direction: SupportAccessRecord references User as secondApproverId
      - entity: AuditEvent
        cardinality: one-to-many
        direction: a session's reads appear as AuditEvent rows whose context names the session id

  - name: RetentionRun
    description: >
      One execution of a retention sweep, under a separate privileged role, recording what it
      deleted and how it ended. The audit service runs the audit-log sweep itself, inside its own
      schema, and records the sweeps other services report to it; it never deletes another
      service's rows.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: category
        type: enum
        required: true
        allowed_values: [audit_logs, reported]
        description: >
          `audit_logs` for a sweep the audit service runs in its own schema; `reported` for a sweep
          another service ran in its own schema and reported through the run endpoint.
      - name: owner
        type: enum
        required: true
        allowed_values: [audit, ledger, account, portfolio, reporting, notification, analytics, job_orchestration, market_data, asset_registry, identity]
        description: The service that executed the sweep.
      - name: state
        type: enum
        required: true
        allowed_values: [running, success, failed]
        default: running
      - name: cutoffAt
        type: timestamp
        required: true
        description: >
          Records whose recordedAt is strictly older than this instant were in scope. The window is
          measured against recordedAt, not occurredAt, so a late-arriving event is not swept on the
          run after it lands. Six calendar months means the same day six calendar months earlier,
          clamped to the last day of the month.
      - name: startedAt
        type: timestamp
        required: true
      - name: finishedAt
        type: timestamp
        required: false
      - name: deletedCount
        type: integer
        required: true
        default: 0
        min: 0
      - name: privilegedRole
        type: string
        required: true
        max_length: 64
        description: The audited role the sweep ran under.
      - name: outcome
        type: enum
        required: false
        allowed_values: [success, failed]
        description: Set when the run reaches its terminal state.
      - name: failureReason
        type: string
        required: false
        max_length: 500
    constraints:
      - "An audit_logs run never deletes a record still inside the six-month window."
      - "The two-year financial-record window is executed by Ledger in its own schema and reported here; the audit service never deletes Ledger's rows."
      - "Every run writes an AuditEvent naming its id, whatever its outcome."
    relationships:
      - entity: AuditEvent
        cardinality: one-to-many
        direction: a run's events reference the run id in context

  - name: AuditExport
    description: >
      One request to extract audit records, with the filter used and how it ended. The export
      itself is produced elsewhere; this row is the audited record of the request.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: requestedBy
        type: uuid
        required: true
        references: User (Identity)
      - name: filter
        type: map
        required: true
        description: Action prefixes, target, and the time range. Holds no free-text query.
      - name: requestedAt
        type: timestamp
        required: true
      - name: completedAt
        type: timestamp
        required: false
      - name: rowCount
        type: integer
        required: true
        default: 0
        min: 0
      - name: state
        type: enum
        required: true
        allowed_values: [requested, success, denied, failed]
        default: requested
      - name: outcome
        type: enum
        required: false
        allowed_values: [success, denied, failed]
        description: Set when the export reaches a terminal state.
      - name: contentSha256
        type: string
        required: false
        max_length: 64
        description: >
          Hash of the body that was streamed to the operator, so an export can be proven to match
          what the service sent. No file is kept: the body is the export.
    constraints:
      - "Only the admin role may request an export."
      - "The produced file carries no credential, token, or full wallet address."
    relationships:
      - entity: User
        owned_by: Identity
        cardinality: many-to-one
        direction: AuditExport references User by requestedBy
```

## Entity summary

Four entities, three of them aggregates in their own right and one of them a record of a request.

| Entity | What it is | Why it exists |
|---|---|---|
| `AuditEvent` | An append-only record of an action, its actor, target, time, context, and outcome | The core of the unit and the store every other service writes into |
| `SupportAccessRecord` | A time-bound, reason-recorded, second-approved support session | Makes support access bounded and visible rather than implicit |
| `RetentionRun` | One retention sweep and what it deleted | Proves the retention policy ran, and that it ran under a separate privileged role |
| `AuditExport` | One audited export request and its result | Exports leave the system, so the request has to be recorded |

`AuditEvent` is the only entity every other service touches; it is written through the internal
append API and read only by operator queries. `SupportAccessRecord`, `RetentionRun`, and
`AuditExport` are written by the audit service itself in response to its own workflows.

Nothing here holds a credential, a token, a seed phrase, or a full wallet address — the entity
constraints say so, and business rule BR1.3 enforces it at append time.

## Sources

- `inception/units-generation/unit-of-work.md` — Unit U12's responsibility statement
- `inception/requirements-analysis/requirements.md` — FR11.2, FR11.3, FR8.1, FR8.2, NFR13, NFR14
- `inception/domain-design/components.md` — the Audit component; `AuditEvent` and
  `SupportAccessRecord` come from it, and `RetentionRun` and `AuditExport` are functional-design
  elaborations of the component's "retention and export of audit data" responsibility. Both must
  join the C4 model and `docs/architecture/README.md` in the next architecture update, per the
  project rule that the two move together.
- `inception/contract-design/contract-summary.md` — contract C10 (append), extended here with the
  support-session, query, export, and privacy-deletion operations this unit needs; and C11 (job run
  endpoint) for retention runs.
