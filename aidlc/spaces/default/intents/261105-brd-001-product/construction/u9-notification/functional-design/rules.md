# Business Rules — u9-notification

```yaml
rules:
  - id: BR1.1
    statement: An alert is deduplicated by its idempotency key.
    category: constraint
    applies_to: Alert
    trigger: An alert is submitted through C9.
    logic: IF an Alert with the same idempotencyKey exists THEN return deduplicated and send nothing; ELSE create it queued.
    violation: The same event never produces two notifications.
    source: FR7.1, AC5.6.2

  - id: BR1.2
    statement: A guardrail re-crossing produces a new alert only after leaving the guardrail.
    category: policy
    applies_to: Alert
    trigger: Drift is evaluated.
    logic: Portfolio owns the inside/outside state and the key. IF drift is outside and the previous evaluation was inside THEN Portfolio submits an alert keyed (user, portfolio, asset, guardrail, crossingAt); IF it stays outside THEN the same key is resubmitted and Notification dedupes it; IF it returns inside THEN Portfolio clears its state so a later crossing keys anew.
    violation: One crossing, one notification; a new crossing after recovery notifies again.
    source: FR7.4, AC5.6.1–5.6.3

  - id: BR2.1
    statement: A disabled preference suppresses that type and channel.
    category: policy
    applies_to: NotificationPreference
    trigger: An alert is delivered.
    logic: IF the user's preference for (type, channel) is disabled THEN skip that channel (except a mandatory email, BR3.1); ELSE deliver. Saving a disabled preference for security or key_revoked email is refused with a field-level reason.
    violation: A disabled type is not delivered on that channel.
    source: FR7.2, AC6.3.1

  - id: BR2.2
    statement: A preference change applies to the next event.
    category: policy
    applies_to: NotificationPreference
    trigger: Preferences are saved.
    logic: IF the change is saved THEN the next delivery reads the new value; no cache outlives the request.
    violation: A changed preference is never ignored by a later event.
    source: AC6.3.2

  - id: BR2.3
    statement: Preferences show both channels per type.
    category: policy
    applies_to: NotificationPreference
    trigger: Preferences are read.
    logic: IF a type has no row THEN return the default for both channels; ELSE return the stored values.
    violation: The settings screen always shows in-app and email state per type.
    source: AC6.3.3

  - id: BR3.1
    statement: Security and credential emails are mandatory.
    category: constraint
    applies_to: Alert
    trigger: A security or key_revoked event occurs.
    logic: IF the type is security or key_revoked THEN queue email even when the preference is off; in-app follows the preference.
    violation: A credential revocation or new-device sign-in always reaches the verified email.
    source: FR7.3, AC6.4.1–6.4.2

  - id: BR4.1
    statement: Delivery retries are bounded and recorded.
    category: policy
    applies_to: DeliveryAttempt
    trigger: A delivery fails.
    logic: IF the attempt count is below 3 THEN retry after 1 minute and then 5 minutes and record each attempt; ELSE mark abandoned, set the alert failed or partially_delivered, and surface it.
    violation: Every failure has a record and an outcome.
    source: FR7.1, AC6.2.2

  - id: BR4.2
    statement: Email goes to the verified address only.
    category: constraint
    applies_to: DeliveryAttempt
    trigger: An email is delivered.
    logic: IF C3 `GET /v1/users/{userId}/contact` returns a verified address THEN send; ELSE do not send email, record the `address` error category, and (for a mandatory alert) surface it. Notification stores no address.
    violation: An unverified address receives nothing.
    source: AC6.2.1

  - id: BR4.3
    statement: Email off keeps the brief in-app.
    category: policy
    applies_to: Alert
    trigger: A brief is generated with email disabled.
    logic: IF the brief email preference is off THEN deliver in-app only and record no email attempt.
    violation: No email is sent.
    source: AC6.2.3

  - id: BR5.1
    statement: Notification content never carries a credential or full address.
    category: constraint
    applies_to: Alert
    trigger: An alert is created or rendered.
    logic: Each type has a payload allow-list (drift: portfolioId, assetId, driftPoints; sync_failed: accountId, reason; key_revoked/security: event, deviceLabel; brief: briefId). IF a value matches an email, a 0x 40-hex address, a 32–44 base58 string, an IPv4/IPv6 literal, or `^[A-Za-z0-9_-]{20,}$` (sk-/pk-/Bearer/eyJ prefixes) THEN drop the field and record its name in `sanitisedFields`; the notification still delivers without it.
    violation: No secret or full address appears in any channel or delivery log.
    source: FR7.3, AC6.4.3, NFR9
```

## Rules summary

| Rule | Category | Statement |
|---|---|---|
| BR1.1 | constraint | Idempotency-key dedupe |
| BR1.2 | policy | Guardrail crossing semantics |
| BR2.1 | policy | Disabled type/channel suppressed |
| BR2.2 | policy | Preference applies to next event |
| BR2.3 | policy | Both channels shown with defaults |
| BR3.1 | constraint | Mandatory security/credential email |
| BR4.1 | policy | Bounded retries and recorded outcomes |
| BR4.2 | constraint | Verified address only |
| BR4.3 | policy | Email off keeps the brief in-app |
| BR5.1 | constraint | No credentials or full addresses |
