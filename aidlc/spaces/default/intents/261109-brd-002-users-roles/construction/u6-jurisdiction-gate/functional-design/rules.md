# Business Rules — u6-jurisdiction-gate

```yaml
rules:
  - id: BR1.1
    statement: A check evaluates the self-declared country and the IP signal against the current list version and returns allowed or denied with the sources that restricted it.
    category: calculation
    applies_to: JurisdictionRule
    trigger: a sign-up gate check or a continued-use check
    logic: "IF either signal's country is restricted THEN deny, restrictedBy = the restricted signal(s) ELSE allow"
    violation_behaviour: n/a — this rule produces the decision
    source: AC6.3.1
  - id: BR1.2
    statement: A restricted self-declared country denies the check.
    category: policy
    applies_to: JurisdictionRule
    trigger: the self-declared country resolves to status restricted
    logic: "IF self_declared = restricted THEN deny and name self_declared"
    violation_behaviour: the refusal is returned and recorded; nothing is stored
    source: AC6.3.1
  - id: BR1.3
    statement: A restricted IP-resolved country denies the check, and the stricter signal wins when the two disagree.
    category: policy
    applies_to: JurisdictionRule
    trigger: the IP signal resolves to status restricted
    logic: "IF ip = restricted THEN deny and name ip (when both restrict, name both)"
    violation_behaviour: the refusal is returned and recorded; nothing is stored
    source: AC6.3.1
  - id: BR1.4
    statement: A country code must be a valid ISO 3166-1 alpha-2 value.
    category: validation
    applies_to: check request
    trigger: any check request
    logic: "IF countryCode does not match ^[A-Z]{2}$ THEN reject as invalid input"
    violation_behaviour: 422 invalid input; no decision is recorded
    source: AC6.3.1
  - id: BR1.5
    statement: When the geolocation signal is unavailable the check fails closed.
    category: policy
    applies_to: check request
    trigger: the geolocation provider errors or times out
    logic: "IF the ip signal cannot be evaluated THEN return service-unavailable; do not allow"
    violation_behaviour: the caller surfaces a retry; no sign-up proceeds and nothing is stored
    source: AC6.3.1
  - id: BR1.6
    statement: A decision is never made against a partially loaded or checksum-mismatched list.
    category: constraint
    applies_to: JurisdictionListVersion
    trigger: startup or a publish of a new list version
    logic: "IF checksum does not match the stored entries THEN refuse to serve and log loudly"
    violation_behaviour: startup fails or the gate returns service-unavailable; no silent allow
    source: AC6.3.1
  - id: BR1.7
    statement: Every check records evidence — country codes, signals used, list version, outcome — with no personal data.
    category: policy
    applies_to: check request
    trigger: every decision, allowed or denied
    logic: "IF a decision is made THEN write an audit event with actor (or system), action, target, timestamp, context, outcome"
    violation_behaviour: a failed audit write is surfaced; the decision is not silently completed
    source: AC6.4.2
  - id: BR1.8
    statement: Continued use re-evaluates an existing user's location and blocks access when they become restricted.
    category: authorization
    applies_to: existing user sessions
    trigger: an authenticated request after the re-evaluation window
    logic: "IF the current signals resolve to restricted THEN block the request and audit it"
    violation_behaviour: the request is refused; the user sees the unavailability notice
    source: AC6.3.2
  - id: BR1.9
    statement: A blocked check returns before any personal data is written.
    category: constraint
    applies_to: sign-up provisioning order
    trigger: a sign-up attempt
    logic: "IF the gate denies THEN the caller must not create a user record, profile row, or shredding key"
    violation_behaviour: the provisioning path refuses; a write attempted after a denial is a defect
    source: AC6.4.1
  - id: BR1.10
    statement: A decision cites the list version it used.
    category: policy
    applies_to: decision evidence
    trigger: every decision
    logic: "the response and the audit evidence carry the JurisdictionListVersion.version"
    violation_behaviour: a decision without a version is rejected at the boundary
    source: AC6.4.2
```

## Summary

| Rule | Statement | Category | Source |
|---|---|---|---|
| BR1.1 | Evaluate both signals against the current list and decide | calculation | AC6.3.1 |
| BR1.2 | A restricted self-declared country denies | policy | AC6.3.1 |
| BR1.3 | A restricted IP country denies; stricter wins | policy | AC6.3.1 |
| BR1.4 | Country code must be ISO alpha-2 | validation | AC6.3.1 |
| BR1.5 | Geolocation unavailable → fail closed | policy | AC6.3.1 |
| BR1.6 | Never decide on a partial or mismatched list | constraint | AC6.3.1 |
| BR1.7 | Every decision records evidence, no personal data | policy | AC6.4.2 |
| BR1.8 | Continued use re-evaluates and blocks | authorization | AC6.3.2 |
| BR1.9 | Denial returns before any personal data is written | constraint | AC6.4.1 |
| BR1.10 | Decisions cite the list version | policy | AC6.4.2 |
