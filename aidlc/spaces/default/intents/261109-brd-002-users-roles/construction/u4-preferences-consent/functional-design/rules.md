# Business Rules — u4-preferences-consent

**Upstream:** `requirements.md` (FR19.1, FR19.2, FR19.5), `stories.md` (US6.1, US6.2, US6.5), `contract-summary.md` (C6), `components.md` (PreferencesConsent).

## Source of truth

```yaml
rules:
  - id: BR4.1
    statement: A preference field is null until the user sets it; a non-null value is the user's explicit choice and is never overwritten by a default, a provider claim, a migration, or another unit.
    category: calculation
    applies_to: UserPreferences
    trigger: any read or partial write of a preference
    logic: "IF the stored field is null THEN the read returns its default with source=default; ELSE the read returns the stored value with source=user. A write sets only the fields it supplies"
    violation_behaviour: a default written over a user's choice, or a partial write that clears an unsupplied field, is a defect
    source: AC6.1.1

  - id: BR4.2
    statement: Preference values come from closed sets — locale en or vi, base currency USD, EUR, GBP, or VND, timezone a known IANA zone, notification channels email and push only.
    category: validation
    applies_to: UserPreferences
    trigger: a preference write
    logic: "IF any supplied value or notification key is outside its allowed set THEN refuse the write and name the field"
    violation_behaviour: 422 naming the offending field or key; the previous value is kept
    source: AC6.1.1, AC6.5.1

  - id: BR4.3
    statement: Preference and consent reads and writes verify that the caller owns the record.
    category: authorization
    applies_to: UserPreferences, ConsentRecord
    trigger: any read or write
    logic: "IF the record's userId differs from the caller's user id THEN refuse with the same response a nonexistent record produces"
    violation_behaviour: refused and audited; no field of another user's record is returned
    source: AC6.1.1, AC6.1.2

  - id: BR4.4
    statement: A consent record is immutable — one row per grant or withdrawal, never updated or deleted.
    category: constraint
    applies_to: ConsentRecord
    trigger: any write to an existing consent row
    logic: "consent rows are inserted; an UPDATE or DELETE is refused by the repository and by the schema"
    violation_behaviour: the write fails; the history stays as recorded
    source: AC6.1.2

  - id: BR4.5
    statement: The current consent state for a kind is the newest event for that user and kind, ordered by the database-assigned timestamp and then by the monotonic sequence.
    category: calculation
    applies_to: ConsentRecord
    trigger: any decision that depends on consent
    logic: "ORDER BY occurredAt DESC, seq DESC LIMIT 1 per (user, kind); occurredAt is written by the database clock and seq breaks ties, so replica clocks cannot invert the state"
    violation_behaviour: a stored 'current' value that disagrees with the events is a defect; none is stored
    source: AC6.1.2

  - id: BR4.6
    statement: Connecting any source requires granted terms, risk-disclosure, and age consent for that user; a missing acceptance is named in the refusal.
    category: policy
    applies_to: every source-connection attempt, not only the first
    trigger: an attempt to connect a source
    logic: "IF any of terms, risk, age is not currently granted THEN refuse and name every missing kind"
    violation_behaviour: the connection is refused and no source record is created
    source: AC6.2.1, AC6.2.2

  - id: BR4.7
    statement: A withdrawal of terms or risk consent takes effect immediately and blocks the next source-connection attempt, wherever it falls in the account's life.
    category: policy
    applies_to: ConsentRecord
    trigger: a withdrawal is recorded
    logic: "the withdrawal row becomes the newest event for its kind, so the next connection check sees it withdrawn and refuses"
    violation_behaviour: any connection succeeding after a withdrawal of terms or risk is a defect, first connection or not
    source: AC6.2.1

  - id: BR4.8
    statement: Every grant and withdrawal is recorded with the database timestamp and a policy version, and is visible to later stages.
    category: constraint
    applies_to: ConsentRecord
    trigger: a consent grant or withdrawal
    logic: "insert one row carrying kind, action, occurredAt (database clock), policyVersion, user id and seq. A grant supplies policyVersion from the client; a withdrawal carries forward the newest grant's policyVersion, resolved server-side, so the caller never sends one"
    violation_behaviour: a grant without a policy version is refused (422); a withdrawal whose kind has no prior grant is refused (409), so there is never a withdrawal without a version
    source: AC6.1.2, AC6.2.1

  - id: BR4.9
    statement: Changing locale or base currency changes presentation only; stored financial values are never re-based.
    category: calculation
    applies_to: UserPreferences
    trigger: a locale or base-currency change
    logic: "the preference row changes; no stored amount, valuation, or transaction is rewritten"
    violation_behaviour: a value rewritten by a preference change is a defect
    source: AC6.1.1

  - id: BR4.10
    statement: Age assurance in this release is a self-declaration; the confirmation is recorded and no identity document is requested.
    category: policy
    applies_to: ConsentRecord (kind age)
    trigger: the age step of the entry gates
    logic: "a granted age event is recorded; the flow never asks for an upload or a document"
    violation_behaviour: a flow that requests a document, or that proceeds without a granted age event, is a defect
    source: AC6.5.1, AC6.5.2

  - id: BR4.11
    statement: Analytics consent governs optional product analytics only; granting or withdrawing it never blocks a connection and never changes a stored financial value.
    category: policy
    applies_to: ConsentRecord (kind analytics)
    trigger: an analytics grant or withdrawal
    logic: "the event is recorded and its current state is exposed on the consent-state query; the analytics path reads it and stops emitting for that user when withdrawn"
    violation_behaviour: an analytics withdrawal blocking a connection, or an analytics event emitted for a user who has withdrawn, is a defect
    source: AC6.1.2
```

## Summary

| Rule | Statement | Category | Source |
|---|---|---|---|
| BR4.1 | Unset is null; explicit is never overwritten | calculation | AC6.1.1 |
| BR4.2 | Closed value sets, including notification channels | validation | AC6.1.1, AC6.5.1 |
| BR4.3 | Ownership on every read and write | authorization | AC6.1.1–2 |
| BR4.4 | Consent rows are immutable | constraint | AC6.1.2 |
| BR4.5 | Current state = newest event, DB clock + sequence | calculation | AC6.1.2 |
| BR4.6 | Terms, risk, and age before every connection | policy | AC6.2.1–2 |
| BR4.7 | A withdrawal blocks the next connection | policy | AC6.2.1 |
| BR4.8 | Timestamp and policy version on every event; withdrawal inherits the grant's version | constraint | AC6.1.2, AC6.2.1 |
| BR4.9 | Presentation changes, never stored values | calculation | AC6.1.1 |
| BR4.10 | Age is self-declared | policy | AC6.5.1–2 |
| BR4.11 | Analytics consent is optional and never blocks | policy | AC6.1.2 |
