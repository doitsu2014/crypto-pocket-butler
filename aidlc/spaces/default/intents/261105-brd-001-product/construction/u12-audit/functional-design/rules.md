# Audit — Business Rules

Source of truth: the YAML block below. The table after it is a summary.

```yaml
rules:
  - id: BR1.1
    statement: Audit events are append-only; the service role may insert and read them and may never change or remove them.
    category: constraint
    applies_to: AuditEvent
    trigger: Any write to an existing audit event, from any code path or role other than the audited retention role.
    logic: >
      IF a request would update or delete an AuditEvent
      THEN refuse it at the storage permission boundary and record the refusal as a security
      AuditEvent.
    violation: The storage layer refuses the statement; the caller receives 403 with a problem document naming the refused operation.
    source: NFR-SEC-13 (append-only audit), docs/architecture/README.md §7

  - id: BR1.2
    statement: Every audit event names what was done, to what, when, with what result, and by whom or by which system.
    category: validation
    applies_to: AuditEvent
    trigger: Append of a new event.
    logic: >
      IF action, target, and outcome are present (the three required fields of contract C10)
      AND occurredAt, when supplied, is a valid timestamp — an absent occurredAt is recorded as
      the service's own receipt time
      AND (actorId is present when actorKind is user or support_agent, or actorKind is system)
      THEN accept the event
      ELSE reject it naming each missing or contradictory field.
    violation: 422 with a problem document listing the offending field paths; nothing is stored.
    source: FR11.3, AC9.2.2, BRD-012 NFR-SEC-13, contract C10

  - id: BR1.3
    statement: An audit event never carries a credential, a token, a seed phrase, or a full wallet address.
    category: validation
    applies_to: AuditEvent
    trigger: Append of a new event whose target or context holds a value shaped like one of those.
    logic: >
      IF any value in target or context matches a credential, token, seed-phrase, or full-address shape
      THEN reject the whole event naming the offending field path
      ELSE accept it.
    violation: 422 with the offending field path only — never the offending value. The event is rejected, never silently redacted, so the emitter fixes its payload.
    detector: >
      Full-address shapes: 0x followed by 40 hex characters (EVM and BNB Smart Chain); base58
      strings of 32 to 44 characters that decode to a 32-byte key (Solana). Token and key shapes: strings matching the project's secret-prefix list
      (API keys, JWTs with three dot-separated base64url segments, and the Vault pgsodium key
      markers). Seed phrases: twelve or twenty-four space-separated lowercase words drawn from a
      word list. The list lives in one shared validation helper so every emitter is held to the
      same check.
    source: NFR-SEC-03, BRD-012 audit logging

  - id: BR1.4
    statement: Event context carries identifiers and metadata only, never a record payload.
    category: constraint
    applies_to: AuditEvent
    trigger: Append of an event whose context holds free-form business data.
    logic: >
      IF context holds only correlation id, request id, source service, rule id, and session or run
      identifiers
      THEN accept it
      ELSE reject it as carrying payload data.
    violation: 422 naming the over-broad context field.
    source: NFR13 (data minimisation)

  - id: BR2.1
    statement: An event is accepted synchronously and the caller learns its id only once the insert has committed.
    category: policy
    applies_to: AuditEvent
    trigger: A service appends an event through the internal API.
    logic: >
      IF the insert commits
      THEN answer 201 with the event id
      ELSE answer a problem document and store nothing.
    violation: The caller sees a 5xx problem document and retries with its idempotency key; no partial record exists.
    source: FR11.3, contract C10

  - id: BR2.2
    statement: A retried append with the same idempotency key stores one event, not two.
    category: constraint
    applies_to: AuditEvent
    trigger: Append of an event whose idempotencyKey already exists.
    logic: >
      IF an event with that idempotencyKey exists
      THEN answer 201 with the existing id and insert nothing
      ELSE insert the new event.
    violation: Not applicable; the rule's failure mode is a duplicate row, which the unique constraint prevents.
    source: project rule — bounded retries with idempotency keys, .claude/rules/development.md

  - id: BR3.1
    statement: A support session starts only with a reason, a second approver who is not the agent, and an expiry.
    category: authorization
    applies_to: SupportAccessRecord
    trigger: A support agent requests access to a user's metadata or tickets.
    logic: >
      IF reason is at least ten characters
      AND secondApproverId is present and differs from supportAgentId
      AND expiresAt is later than startedAt and at most eight hours after it
      THEN record the session as active and write an AuditEvent
      ELSE deny the request and record the denial.
    violation: 403 with a problem document; the denial itself is audited as a security event.
    source: FR11.2, AC9.3.2 — the reason, approver, and expiry requirements come from there; the ten-character floor and the eight-hour ceiling are this design's choices
    design_decision: >
      Ten characters is the shortest reason that carries information rather than a placeholder such
      as "look". Eight hours covers one support shift; the BRD describes the window as configured,
      so the ceiling is a configuration value seeded at eight hours, held in the environment, and
      validated against the same rule.

  - id: BR3.2
    statement: Support access ends at its expiry, whether or not anyone ends it.
    category: policy
    applies_to: SupportAccessRecord
    trigger: Any support read, or the passage of expiresAt.
    logic: >
      IF now is later than expiresAt
      THEN treat the session as expired: refuse the read and record it as denied
      ELSE allow the read within the session's scope.
    violation: A read past expiry is refused with 403 and audited; access never outlives the window.
    source: FR11.2, AC9.3.1

  - id: BR3.3
    statement: Every support read of account metadata or tickets is recorded in the audit trail.
    category: policy
    applies_to: SupportAccessRecord
    trigger: A support agent reads within an active session.
    logic: >
      IF the read is within scope of an active session
      THEN serve it and write an AuditEvent naming the session id, the subject, and the read
      ELSE refuse it.
    violation: A read that is not recorded does not happen; the audit write precedes the response.
    source: FR11.2, AC9.3.3

  - id: BR4.1
    statement: The audit service retains its own audit logs for six calendar months, and a retention run deletes only records past that window.
    category: policy
    applies_to: RetentionRun
    trigger: A scheduled or operator-started audit-log retention run.
    logic: >
      IF a record's recordedAt is older than six calendar months before the run's cutoff
      AND no legal hold applies to it
      THEN delete it
      ELSE keep it.
    violation: The run refuses to delete an in-window or held record and reports it in its outcome.
    source: NFR14 (retention windows, Interview Q5)
    scope_note: >
      The two-year financial-record window is owned by Ledger (U7) and executed by it inside its own
      schema. The audit service never deletes another service's rows and never receives a
      cross-schema grant. It records the run that Ledger reports (BR4.2), not the deletion itself.

  - id: BR4.2
    statement: A retention run executes under a separate audited privileged role and records itself.
    category: security
    applies_to: RetentionRun
    trigger: The start of any retention run, whether the audit service runs it or another service reports one.
    logic: >
      IF the audit-log sweep is executed under the audited retention role, or another service
      reports a completed run under its own audited role
      THEN record the RetentionRun row and write an AuditEvent naming it
      ELSE refuse to run, or refuse the report.
    violation: The audit-log sweep cannot start under the ordinary service role; a report without a role and an outcome is refused. The attempt is audited.
    source: FR11.3, unit responsibility — retention under a separate audited privileged role

  - id: BR4.3
    statement: Records kept past a deletion request are held only under the stated legal basis and are labelled with it at append time.
    category: policy
    applies_to: AuditEvent
    trigger: An account deletion reaches audit records, or an event is appended that must outlive a deletion.
    logic: >
      IF a record must be retained after deletion because of the stated legal basis
      THEN keep it, with legalBasis and deletionRequestId set on the row — both set when the row is
      inserted, never by an update, because BR1.1 forbids changing a stored event
      ELSE let the deletion remove it.
    violation: An unlabelled retained record is a defect surfaced by the retention run's report.
    source: FR8.2, AC7.4.2, NFR14

  - id: BR5.1
    statement: Audit queries and exports are admin-only, deny-by-default; support sessions are support-agent-only.
    category: authorization
    applies_to: [AuditEvent, AuditExport]
    trigger: Any read, export, or support-session request arriving at the audit service.
    logic: >
      IF the caller's validated JWT carries app_metadata.role = admin (queries and exports) or
      support (support sessions), and the record is inside the caller's scope
      THEN serve the request
      ELSE deny it and audit the denial.
    violation: 403 with a problem document; the denial is recorded as a security AuditEvent.
    source: NFR-SEC-07, FR1.5 (admin and support roles are authorised from app_metadata.role)
    scope_note: >
      There is no separate "operator" role. Admin covers audit review and export; support covers the
      support-session paths. A record's scope is: every audit event for admin; only the events of a
      session the agent holds, and only within that session's scope, for support.

  - id: BR5.2
    statement: Every audit export is itself audited, and the produced file carries no credential or token.
    category: policy
    applies_to: AuditExport
    trigger: An admin requests an export of audit records.
    logic: >
      IF the caller holds the admin role
      THEN record the request, produce the file, record the count and outcome, and write an AuditEvent
      ELSE deny it.
    violation: An export that cannot be recorded is not produced.
    source: unit responsibility — admin queries and exports; NFR-SEC-03
```

## Rules summary

| Rule | Category | Applies to | One line |
|---|---|---|---|
| BR1.1 | constraint | AuditEvent | Append-only: insert and read, never change or remove |
| BR1.2 | validation | AuditEvent | Action, target, time, outcome, and actor are all required |
| BR1.3 | validation | AuditEvent | No credential, token, seed phrase, or full address ever |
| BR1.4 | constraint | AuditEvent | Context is metadata, not a record payload |
| BR2.1 | policy | AuditEvent | Accepted synchronously; the id comes back after the commit |
| BR2.2 | constraint | AuditEvent | A retried append with the same key stores one event |
| BR3.1 | authorization | SupportAccessRecord | Reason, a second approver, and an expiry, or no session |
| BR3.2 | policy | SupportAccessRecord | Access ends at expiry, written or not |
| BR3.3 | policy | SupportAccessRecord | Every support read is audited |
| BR4.1 | policy | RetentionRun | Six months of audit logs; financial records belong to Ledger |
| BR4.2 | security | RetentionRun | Retention runs privileged, and records itself |
| BR4.3 | policy | AuditEvent | The legal basis is stamped at append time, never by an update |
| BR5.1 | authorization | AuditEvent, AuditExport | Admin for queries and exports, support for sessions |
| BR5.2 | policy | AuditExport | Exports are audited and secret-free |
