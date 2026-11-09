# Business Rules — u13-support-console

```yaml
rules:
  - id: BR1.1
    statement: Only support and admin roles reach the console.
    category: authorization
    applies_to: SupportApp
    trigger: A user opens the console.
    logic: IF app_metadata.role is support or admin THEN show the shell; ELSE show "not authorised" and load no data.
    violation: No metadata is fetched for a non-support user.
    source: FR11.2

  - id: BR1.2
    statement: A support grant requires a reason, a time bound, and a second approver.
    category: authorization
    applies_to: SupportAccessGrant
    trigger: A grant is requested.
    logic: IF reason is at least 20 characters AND duration is 1–480 minutes AND the approver differs from the requester THEN create the grant as pending; ELSE refuse with a problem document.
    violation: A grant without a reason, bound, or approver is not created.
    source: FR11.2, AC9.3.2

  - id: BR1.3
    statement: Access ends when the grant expires.
    category: authorization
    applies_to: SupportAccessGrant
    trigger: A read is attempted.
    logic: IF the grant is active, expiresAt is in the future, AND the account's owner equals the grant's target user THEN allow; ELSE refuse 403 and show "access ended" or "not your grant". Search is scoped the same way: it returns only accounts owned by the grant's target user.
    violation: An expired grant grants nothing.
    source: AC9.3.1

  - id: BR1.4
    statement: The console never reads a credential or portfolio content.
    category: constraint
    applies_to: AccountMetadataPanel
    trigger: Any read.
    logic: IF the requested field is not metadata (kind, label, venue, status, lastSyncedAt) THEN it is absent from the DTO and never rendered.
    violation: No credential, holding, valuation, or portfolio content is shown.
    source: FR11.2, NFR13

  - id: BR2.1
    statement: Every support read is audited.
    category: policy
    applies_to: SupportRead
    trigger: A metadata read happens.
    logic: IF the read is allowed THEN write a C10 support_metadata_read event with actor, the grant id, a reason hash (SHA-256 prefix; never the free text), the target account id, and outcome.
    violation: A read without an audit record is refused.
    source: AC9.3.3

  - id: BR2.2
    statement: Audit context carries identifiers only.
    category: constraint
    applies_to: SupportRead
    trigger: An audit event is written.
    logic: IF context is built THEN include account id, the grant id, a reason hash, and outcome; never the free-text reason, a label with personal data, credential, or address.
    violation: No sensitive value is stored in the audit trail.
    source: NFR13

  - id: BR3.1
    statement: Only admin declares or updates an incident.
    category: authorization
    applies_to: Incident
    trigger: The incident console is used.
    logic: IF role is admin THEN allow create/update; ELSE hide the console.
    violation: A support user cannot declare an incident.
    source: FR11.1

  - id: BR3.2
    statement: A SEV-1 incident is communicated within 15 minutes.
    category: policy
    applies_to: Incident
    trigger: A SEV-1 is declared.
    logic: IF severity is sev1, or an update escalates sev2 to sev1, THEN ask C9 to notify subscribers immediately (for an escalation, measured from the update) and record the send time; a failed send is retried and surfaced.
    violation: A SEV-1 without a notice within 15 minutes is an alert.
    source: AC8.1.3, NFR-AVL-05

  - id: BR3.3
    statement: A declared incident is visible on the public status page.
    category: policy
    applies_to: Incident
    trigger: An incident is declared or updated.
    logic: IF status is declared or investigating THEN the support API exposes it to u2-web-app's status page with severity, summary, components, and updatedAt.
    violation: The status page never shows an undeclared or resolved incident as active.
    source: AC8.1.2

  - id: BR4.1
    statement: Help articles and the contact channel are U2's; the console reuses them read-only.
    category: policy
    applies_to: SupportApp
    trigger: The help panel is opened.
    logic: IF the console shows help THEN it renders the same article list read-only and links to the customer channel.
    violation: The console does not fork help content.
    source: AC8.1.1
```

## Rules summary

| Rule | Category | Statement |
|---|---|---|
| BR1.1 | authorization | support/admin role only |
| BR1.2 | authorization | reason, bound, second approver |
| BR1.3 | authorization | expiry ends access |
| BR1.4 | constraint | metadata only, never credentials |
| BR2.1 | policy | every read audited |
| BR2.2 | constraint | identifiers only in audit |
| BR3.1 | authorization | admin-only incidents |
| BR3.2 | policy | SEV-1 notice within 15 min |
| BR3.3 | policy | incident on the public status page |
| BR4.1 | policy | help content reused, not forked |
