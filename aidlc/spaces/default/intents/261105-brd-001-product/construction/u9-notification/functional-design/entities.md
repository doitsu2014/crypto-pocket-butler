# Entity Model — u9-notification

Inputs: U9 in `inception/units-generation/unit-of-work.md`, `inception/requirements-analysis/requirements.md`
(FR7.1–FR7.4), the Notification component in `inception/domain-design/components.md`, and C9.

```yaml
entities:
  - name: NotificationPreference
    description: One user's on/off choice for one notification type and channel.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: userId
        type: uuid
        required: true
      - name: type
        type: enum
        required: true
        allowed_values: [drift, sync_failed, key_revoked, security, brief]
      - name: channel
        type: enum
        required: true
        allowed_values: [in_app, email]
      - name: enabled
        type: boolean
        required: true
        default: true
      - name: updatedAt
        type: timestamp
        required: true
    constraints:
      - (userId, type, channel) is unique.
      - security and key_revoked are mandatory on email; their preference row may exist but cannot be disabled.
      - A missing row means the default (enabled for in-app; email on for brief, off for drift/sync_failed).

  - name: Alert
    description: One notification event for one user, after dedupe.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: userId
        type: uuid
        required: true
      - name: type
        type: enum
        required: true
        allowed_values: [drift, sync_failed, key_revoked, security, brief]
      - name: idempotencyKey
        type: string
        required: true
        unique: true
      - name: payload
        type: map
        required: false
        description: Allow-listed fields only; never a credential, token, or full address.
      - name: status
        type: enum
        required: true
        allowed_values: [queued, delivered, partially_delivered, failed]
        default: queued
      - name: sanitisedFields
        type: array
        required: false
        description: Field names dropped from the payload because they carried a credential or full address.
      - name: createdAt
        type: timestamp
        required: true
    constraints:
      - idempotencyKey is unique; a repeat returns deduplicated and sends nothing.
      - A mandatory type (security, key_revoked) is always queued for email regardless of preferences.

  - name: DeliveryAttempt
    description: One delivery try for one alert and channel.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: alertId
        type: uuid
        required: true
        references:
          entity: Alert
          relationship: many attempts per alert
      - name: channel
        type: enum
        required: true
        allowed_values: [in_app, email]
      - name: attempt
        type: integer
        required: true
        min: 1
      - name: status
        type: enum
        required: true
        allowed_values: [sent, failed, abandoned]
      - name: errorCategory
        type: enum
        required: false
        allowed_values: [provider, network, address, content]
      - name: attemptedAt
        type: timestamp
        required: true
    constraints:
      - At most 3 attempts per alert and channel (initial + 2 retries), with 1 and 5 minutes between them; the third failure sets abandoned.
      - An abandoned email leaves the alert partially_delivered or failed and is visible.
```

## Summary

Notification owns the per-user preferences, the deduplicated alerts, and every delivery attempt.
Security and credential emails are mandatory and cannot be turned off. Content never includes a
credential, token, or full address, and every failure is recorded rather than swallowed.
