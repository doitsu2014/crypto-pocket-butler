# Business Rules — u1-user-registry

```yaml
rules:
  - id: BR1.1
    statement: The first successful authentication for a provider subject creates exactly one internal user record; later authentications reuse it.
    category: calculation
    applies_to: User
    trigger: an authenticated request carrying a valid provider subject
    logic: "IF no user exists for the subject hash THEN create one with status pending ELSE refresh the existing record"
    violation_behaviour: a write that would create a second record for one subject is rejected by the unique constraint
    source: AC1.1.1
  - id: BR1.2
    statement: A user's internal id never changes when the provider rotates claims such as email or display name.
    category: constraint
    applies_to: User
    trigger: any refresh from provider claims
    logic: "the id is assigned at creation and is never derived from mutable claims"
    violation_behaviour: a refresh that would change the id is refused
    source: AC1.2.1
  - id: BR1.3
    statement: Every user-owned resource carries exactly one owner user id, and a write leaving it with none or several is rejected.
    category: constraint
    applies_to: all user-owned resources
    trigger: any write of a user-owned row
    logic: "IF owner_user_id is absent or a second owner is attached THEN reject the write"
    violation_behaviour: the write fails and the error is surfaced
    source: AC1.3.1
  - id: BR1.4
    statement: Profile fields are derived from the provider unless the user has overridden them.
    category: calculation
    applies_to: User
    trigger: provisioning or claim refresh
    logic: "IF a field has no user override THEN set it from the claim ELSE keep the user's value"
    violation_behaviour: an invalid claim value leaves the field unchanged and is logged
    source: AC1.4.1
  - id: BR1.5
    statement: No password, MFA secret, refresh token, or access token is stored by the application.
    category: constraint
    applies_to: User
    trigger: schema inspection and every write
    logic: "the user table holds no credential or token column; such a value is never persisted"
    violation_behaviour: the write is refused at review and by the schema
    source: AC1.5.1
  - id: BR1.6
    statement: Account recovery, including email change and lost-device recovery, happens at the provider and does not bypass role, status, or ownership checks.
    category: authorization
    applies_to: User
    trigger: a request after provider recovery
    logic: "the same role, status, and ownership rules apply as for any other request"
    violation_behaviour: a disabled or deleted user is still denied after recovery
    source: AC1.6.1
  - id: BR1.7
    statement: The user status is one of the defined values and drives denial; `pending` is transient and denies authenticated use except confirmation.
    category: policy
    applies_to: User
    trigger: principal resolution and every authenticated request
    logic: "IF status is disabled, deleted, or pending_deletion THEN deny unless the request is the restore path; IF pending THEN allow only the confirmation step"
    violation_behaviour: the request is refused with a forbidden or not-found response
    source: AC4.1.1
  - id: BR1.8
    statement: The subject is stored only as a hash for lookup plus a deterministic ciphertext; it never appears in responses, logs, or audit records.
    category: constraint
    applies_to: User
    trigger: every write and read
    logic: "the plaintext subject is hashed and encrypted at the boundary; only those two forms are persisted"
    violation_behaviour: an attempt to persist or log the plaintext subject is refused
    source: AC1.3.1, NFR22
  - id: BR1.9
    statement: An admin may look up a user record; changing a role is u2's endpoint, not a registry write.
    category: authorization
    applies_to: User
    trigger: an admin lookup
    logic: "IF the caller is admin THEN return the record within the metadata boundary ELSE deny"
    violation_behaviour: a non-admin lookup is denied and audited
    source: AC7.1.1
  - id: BR1.10
    statement: A status value outside the defined set is rejected.
    category: validation
    applies_to: User
    trigger: any status write
    logic: "IF status is not one of the allowed values THEN reject"
    violation_behaviour: the write fails
    source: AC4.1.2
```

## Summary

| Rule | Statement | Category | Source |
|---|---|---|---|
| BR1.1 | One record per provider subject | calculation | AC1.1.1 |
| BR1.2 | Stable internal id | constraint | AC1.2.1 |
| BR1.3 | Exactly one owner per user-owned resource | constraint | AC1.3.1 |
| BR1.4 | Profile from provider unless overridden | calculation | AC1.4.1 |
| BR1.5 | No credentials or tokens stored | constraint | AC1.5.1 |
| BR1.6 | Recovery at the provider, no bypass | authorization | AC1.6.1 |
| BR1.7 | Status values drive denial | policy | AC4.1.1 |
| BR1.8 | Subject hashed and encrypted only | constraint | AC1.3.1 |
| BR1.9 | Admin lookup; role change is u2 | authorization | AC7.1.1 |
| BR1.10 | Invalid status rejected | validation | AC4.1.2 |
