# Business Rules — u2-authorisation

**Upstream:** `requirements.md` (FR15, FR18, FR20), `stories.md` (US2.1–US2.9, US5.1–US5.3, US7.1–US7.4), `contract-summary.md` (C4, C9), `components.md` (Authorisation).

## Source of truth

```yaml
rules:
  - id: BR2.1
    statement: A request that no catalogue entry permits is denied.
    category: authorization
    applies_to: every route and every internal operation
    trigger: an operation that is unknown or whose class no rule permits for the caller
    logic: "IF the operation is not in the catalogue THEN deny as undeclared; ELSE apply the rule its class names"
    violation_behaviour: the request is refused with 403 and the refusal is audited as not_declared
    source: AC2.1.1, AC2.1.2

  - id: BR2.2
    statement: The caller's role is read only from the token's app_metadata.role claim; absent or unrecognised values are owner, and any other role-shaped claim is ignored.
    category: authorization
    applies_to: CallerIdentity
    trigger: every authenticated request
    logic: "IF app_metadata.role is exactly admin THEN admin ELSE IF exactly support THEN support ELSE owner; no other claim, header, body field, or table may grant a role"
    violation_behaviour: a privilege-granting claim in any other position has no effect; the caller is treated as owner
    source: AC2.2.1, AC2.2.2

  - id: BR2.3
    statement: Admin is additive over owner on the admin's own resources, never a substitute for ownership of someone else's, and an administrative operation is decided by the role, not by ownership.
    category: authorization
    applies_to: every owner-scoped and administrative operation
    trigger: an admin calls an owner-scoped or administrative operation
    logic: "IF the operation is administrative THEN only the role decides (BR2.4) and no ownership assertion is evaluated; ELSE IF the role is admin AND resourceOwnerId equals the caller's user id THEN allow exactly as an owner would be allowed"
    violation_behaviour: an admin acting on another user's resource is evaluated by the ownership rule, not by the role
    source: AC2.3.1, AC2.3.2, AC7.1.1

  - id: BR2.4
    statement: Administrative management is reachable only with the admin role, and support is denied it.
    category: authorization
    applies_to: registries, sources, users, roles, jobs, flags, support-grant approval
    trigger: any administrative operation
    logic: "IF the caller's role is not admin THEN deny regardless of ownership"
    violation_behaviour: refused with 403 and no state change; the refusal is audited
    source: AC2.4.1, AC2.4.2, AC7.1.2

  - id: BR2.5
    statement: Every read and write of one user-owned row verifies the caller owns it, and every list, search, or export is answered only with rows whose owner is the caller.
    category: authorization
    applies_to: all user-owned resources
    trigger: an owner-scoped or collection operation
    logic: "IF the operation is owner-scoped THEN IF resourceOwnerId is absent OR differs from the caller's user id THEN deny not_owner; IF the operation is a collection THEN allow and return the owner filter the calling service must apply to every row"
    violation_behaviour: a single-row operation is refused and nothing is returned or written; a collection that returns a row the filter would exclude is a defect
    source: AC2.5.1, AC2.5.2, AC1.3.1, AC1.3.2

  - id: BR2.6
    statement: A refusal never discloses whether another user's resource exists.
    category: authorization
    applies_to: all user-owned resources fetched by id
    trigger: a request for a resource the caller does not own
    logic: "IF the caller does not own the resource THEN return the same 404 a nonexistent id receives, with no field of the resource present"
    violation_behaviour: any response that differs from the nonexistent case, or that carries a field of the refused resource, is a defect
    source: AC2.6.1, AC2.6.2

  - id: BR2.7
    statement: A disabled, pending_deletion, or deleted account is denied every authenticated operation; a pending account may reach only the confirmation operation.
    category: policy
    applies_to: UserStatus, read from u1-user-registry
    trigger: every authenticated request
    logic: "IF status is disabled, pending_deletion, or deleted THEN deny with 403 ELSE IF status is pending AND the operation is not the catalogue's confirmation entry THEN deny ELSE continue"
    violation_behaviour: the request is refused; a valid token never overrides this rule
    source: AC2.7.1, AC2.7.2

  - id: BR2.8
    statement: A scheduled job acts with the system identity, for no user, and never attributes one user's data to another.
    category: constraint
    applies_to: system jobs
    trigger: any background job that reads or writes user-owned rows
    logic: "the job's CallerIdentity is the system variant, which is never an owner and never an admin; it iterates per owner, every row it writes carries the owner it was read for, and it never requests an administrative operation"
    violation_behaviour: a job that writes a row under the wrong owner, requests an administrative operation, or whose response contains another user's data fails the run
    source: AC2.8.1, AC2.8.2

  - id: BR2.9
    statement: The server is authoritative for every decision; client-side gating is presentational only.
    category: authorization
    applies_to: the SPA and the support console
    trigger: any operation an interface hides or shows
    logic: "the same decision is evaluated server-side whether the control was shown, hidden, or invoked directly"
    violation_behaviour: an operation the server does not permit is refused even when the client presents it
    source: AC2.9.1, AC2.9.2

  - id: BR2.10
    statement: A role change is an admin operation, acts for an identified admin, needs a reason, and records the intended change before it writes it.
    category: authorization
    applies_to: ProviderRoleClaim
    trigger: an admin changes a user's role
    logic: "IF the caller's role is not admin THEN deny; IF reason is absent or blank THEN refuse; ELSE record the audit entry naming actor, target, previous role, new role and reason, THEN write the provider claim, THEN record the outcome; IF the audit entry cannot be written nothing is written; IF the provider write fails the recorded entry carries outcome=failed"
    violation_behaviour: no write without a reason and without a prior audit entry; a privileged change is never completed unaudited
    source: AC7.1.1, AC7.2.1, AC7.2.2

  - id: BR2.11
    statement: Every denial is audited by the decision itself, and a denial that cannot be audited fails closed.
    category: policy
    applies_to: every decision the library makes
    trigger: any deny
    logic: "the library writes the audit entry through its AuditSink port before it returns a deny — actor, action, target, timestamp, context, outcome; IF the port fails THEN the decision returns audit_unavailable and the caller answers 503 rather than allowing or silently denying"
    violation_behaviour: a caller cannot return a denial without the entry having been attempted, because the decision does not return until it has
    source: AC2.1.2, AC7.3.1, AC7.3.2

  - id: BR2.12
    statement: No audit record carries a credential, token, secret, or full wallet address.
    category: constraint
    applies_to: every audit entry U2 writes
    trigger: every audit write
    logic: "the fields are chosen so those values cannot appear; a value that would carry one is refused or redacted before the write"
    violation_behaviour: the write is refused or the value redacted; the record never carries the value
    source: AC7.4.1, AC7.4.2

  - id: BR2.13
    statement: A source account links only to portfolios of its own owner, a portfolio may link several accounts and an account several portfolios, and removing a connection removes only that connection and its links.
    category: constraint
    applies_to: account and portfolio linkage
    trigger: link, re-link, value, or remove a source connection
    logic: "IF the account's owner differs from the portfolio's owner THEN refuse and store no link; membership is many-to-many under the existing membership rules and no value is counted twice; removal deletes the connection and its links only"
    violation_behaviour: the cross-owner link is refused and audited; other accounts and the portfolio are untouched by a removal
    source: AC5.1.1, AC5.1.2, AC5.2.1, AC5.2.2, AC5.3.1, AC5.3.2

  - id: BR2.14
    statement: The support role is never an owner and is never granted access to portfolio contents.
    category: authorization
    applies_to: Role
    trigger: a support-role caller requests portfolio contents, valuations, or another user's record
    logic: "IF the role is support THEN deny every owner-scoped operation and every portfolio-content read, with or without a grant"
    violation_behaviour: refused with 403 and audited; a grant changes the metadata boundary, never the role
    source: AC2.4.2, AC3.6.1
```

## Summary

| Rule | Statement | Category | Source |
|---|---|---|---|
| BR2.1 | No catalogue entry, no access | authorization | AC2.1.1–2 |
| BR2.2 | Role only from `app_metadata.role` | authorization | AC2.2.1–2 |
| BR2.3 | Admin is additive; administration is role-decided | authorization | AC2.3.1–2, AC7.1.1 |
| BR2.4 | Admin-only management | authorization | AC2.4.1–2, AC7.1.2 |
| BR2.5 | Ownership on every row; a filter on every collection | authorization | AC2.5.1–2, AC1.3.x |
| BR2.6 | A refusal reveals nothing | authorization | AC2.6.1–2 |
| BR2.7 | Status denial, confirmation excepted | policy | AC2.7.1–2 |
| BR2.8 | Jobs act as system, for no user | constraint | AC2.8.1–2 |
| BR2.9 | The server decides | authorization | AC2.9.1–2 |
| BR2.10 | Role change: admin, reason, audit before write | authorization | AC7.1.1, AC7.2.1–2 |
| BR2.11 | The decision audits its own denials; failure fails closed | policy | AC2.1.2, AC7.3.1–2 |
| BR2.12 | Audit carries no secrets | constraint | AC7.4.1–2 |
| BR2.13 | Linkage stays inside one owner | constraint | AC5.1.1–5.3.2 |
| BR2.14 | Support is never an owner | authorization | AC2.4.2, AC3.6.1 |
