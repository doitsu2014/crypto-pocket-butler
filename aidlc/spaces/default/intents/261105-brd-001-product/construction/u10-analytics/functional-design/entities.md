# Entity Model — u10-analytics

Inputs: U10 in `inception/units-generation/unit-of-work.md`, requirements FR8.3/FR12.2/FR12.3, and
the Analytics component in `inception/domain-design/components.md`.

```yaml
entities:
  - name: AnalyticsEvent
    description: One consented, PII-free product event.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: eventName
        type: string
        required: true
        description: From the closed catalogue, for example onboarding_completed.
      - name: occurredAt
        type: timestamp
        required: true
      - name: properties
        type: map
        required: false
        description: Allow-listed scalar properties only; no free text.
      - name: userRef
        type: string
        required: false
        description: A salted hash of the user id, never the id itself.
      - name: sessionRef
        type: string
        required: false
      - name: consentPurpose
        type: enum
        required: true
        allowed_values: [essential, analytics]
      - name: ingestedAt
        type: timestamp
        required: true
    constraints:
      - No raw user id, email, wallet address, IP, token, or credential is stored.
      - An event not in the catalogue is refused.
      - A property not on the allow-list is dropped, not stored.

  - name: MetricAggregate
    description: One aggregate figure over a period and cohort.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: metric
        type: enum
        required: true
        allowed_values: [activation, engagement, retention, conversion, quality]
      - name: period
        type: enum
        required: true
        allowed_values: [daily, weekly]
      - name: periodStart
        type: timestamp
        required: true
      - name: cohort
        type: string
        required: true
        description: A non-identifying cohort label, for example signup_week or all.
      - name: value
        type: decimal
        required: true
      - name: computedAt
        type: timestamp
        required: true
    constraints:
      - A cohort label never identifies a person.
      - value is a decimal; percentages are stored 0–1.

  - name: FeatureFlag
    description: One feature flag with governance metadata.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: key
        type: string
        required: true
        unique: true
      - name: owner
        type: string
        required: true
      - name: expiresAt
        type: timestamp
        required: true
      - name: rolloutPercentage
        type: integer
        required: true
        min: 0
        max: 100
      - name: status
        type: enum
        required: true
        allowed_values: [active, disabled, expired]
        default: disabled
      - name: createdAt
        type: timestamp
        required: true
      - name: updatedAt
        type: timestamp
        required: true
    constraints:
      - owner and expiresAt are mandatory at creation; a missing one is refused.
      - An expired flag is treated as off and reported for cleanup.
      - Rollout is deterministic per user hash, so a user does not flip between variants.

  - name: ConsentCheck
    description: The recorded outcome of the consent check for one ingest call.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: purpose
        type: enum
        required: true
        allowed_values: [essential, analytics]
      - name: granted
        type: boolean
        required: true
      - name: checkedAt
        type: timestamp
        required: true
      - name: userRef
        type: string
        required: false
    constraints:
      - An ingest without a granted analytics consent is refused with 403 and no event is stored.
      - The check is per request; a withdrawn consent takes effect on the next call.
      - ConsentCheck rows are kept 90 days and carry no raw identifier.
```

## Summary

Analytics stores consented, PII-free events, the daily and weekly aggregates computed from them,
and feature flags with owner, expiry, and rollout. Consent state itself belongs to Identity (C3);
Analytics asks per request and stores only the decision outcome. No raw identifier, address, or
credential is ever stored.
