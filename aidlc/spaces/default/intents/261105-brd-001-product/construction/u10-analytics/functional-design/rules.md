# Business Rules — u10-analytics

Constants: `SALT_BYTES = 32`, `SALT_RETENTION_MONTHS = 12`, `SUPPRESSION_MIN_USERS = 10`,
`EVENT_RETENTION_MONTHS = 6`, `CONSENT_CHECK_RETENTION_DAYS = 90`.

```yaml
rules:
  - id: BR1.1
    statement: A non-essential event is accepted only with granted analytics consent.
    category: authorization
    applies_to: AnalyticsEvent
    trigger: An ingest request arrives.
    logic: IF the event is on the essential list THEN accept without a consent check; ELSE IF C3 getConsent for the JWT subject reports analytics granted THEN accept; ELSE refuse with 403 and store nothing.
    violation: No event is stored for a user who has not consented.
    source: FR8.3, AC7.5.1

  - id: BR1.2
    statement: Withdrawn consent stops later events.
    category: authorization
    applies_to: AnalyticsEvent
    trigger: An ingest arrives after withdrawal.
    logic: IF the C3 check reports not granted THEN refuse with 403; the check is per request, never cached.
    violation: A withdrawn user's later event is never stored.
    source: AC7.5.2

  - id: BR1.3
    statement: Consent is never assumed.
    category: policy
    applies_to: ConsentCheck
    trigger: No grant is recorded.
    logic: IF no analytics grant exists THEN treat as not granted.
    violation: An absent record never counts as consent.
    source: AC7.5.3

  - id: BR2.1
    statement: Only catalogue events with their allow-listed properties are stored.
    category: validation
    applies_to: AnalyticsEvent
    trigger: An event is ingested.
    logic: IF eventName is in the closed catalogue THEN keep only the properties that event allows and drop the rest; ELSE refuse with 422.
    violation: An unknown event is not stored; an unlisted property is never stored.
    source: FR12.2

  - id: BR2.2
    statement: No personal data or credential enters Analytics.
    category: constraint
    applies_to: AnalyticsEvent
    trigger: An event or property is ingested.
    logic: IF a value matches an email (contains @ and a dot), a 0x-prefixed 40-hex address, a 32–44 character base58 string, an IPv4/IPv6 literal, or matches `^[A-Za-z0-9_-]{20,}$` with at least one digit and no spaces (including the prefixes sk-, pk-, Bearer, eyJ) THEN drop the property; IF the offending value is `eventName` or `occurredAt` THEN refuse the event.
    violation: No PII, credential, or full address is stored.
    source: FR12.2, NFR13

  - id: BR2.3
    statement: User references are HMAC-SHA256 hashes under a Vault-held salt.
    category: constraint
    applies_to: AnalyticsEvent
    trigger: A user reference is supplied.
    logic: IF a JWT subject is present THEN store HMAC-SHA256(salt_version, subject) as userRef. The salt is 32 random bytes in Vault and is rotated only on suspected compromise, not on a schedule. The current version and every previous version stay in Vault for 12 months (longer than the event-retention window) so cohorts, rollout, and erasure stay stable.
    violation: The raw identifier is unrecoverable from Analytics, and a user's pseudonym does not change in normal operation.
    source: FR12.2, NFR13

  - id: BR3.1
    statement: Aggregates are computed from consented events over defined periods.
    category: calculation
    applies_to: MetricAggregate
    trigger: The nightly aggregation runs.
    logic: IF the day closes THEN compute, from consented events only, activation = users with onboarding_completed within 24 h of signup / signups; engagement = distinct daily active / weekly active; retention = cohort users with a valuation_read on day 7 or 30 / cohort size; conversion = consented users who complete upgrade_started / consented active users; quality = valuations_read with freshness live / all valuations_read. Store activation and conversion daily; engagement daily; retention weekly. Cohorts are signup_week or all.
    violation: An aggregate is never hand-entered and never computed from unconsented events.
    source: FR12.2

  - id: BR3.2
    statement: A cohort below ten distinct users is suppressed.
    category: policy
    applies_to: MetricAggregate
    trigger: An aggregate would cover a cohort.
    logic: IF the cohort has fewer than SUPPRESSION_MIN_USERS distinct userRef values in the period THEN store no aggregate for it and report suppression.
    violation: No aggregate can single out a person.
    source: FR12.2, NFR13

  - id: BR4.1
    statement: A feature flag requires an owner and an expiry.
    category: validation
    applies_to: FeatureFlag
    trigger: A flag is created or updated.
    logic: IF owner or expiresAt is missing, blank, or in the past THEN refuse with 422; ELSE accept and set status active or disabled.
    violation: A flag cannot exist without governance metadata.
    source: FR12.3

  - id: BR4.2
    statement: Rollout is a deterministic percentage per user.
    category: calculation
    applies_to: FeatureFlag
    trigger: A flag is evaluated for a user.
    logic: IF status is active and expiresAt is future THEN bucket = SHA-256(auth subject) mod 100; enable when bucket < rolloutPercentage; ELSE off. The raw subject is not stored; the bucket is computed per request and never flips for a user.
    violation: The same user never flips variants between calls.
    source: FR12.3

  - id: BR4.3
    statement: An expired flag is off and reported.
    category: policy
    applies_to: FeatureFlag
    trigger: A flag is evaluated, or the admin lists expired flags.
    logic: IF expiresAt is past THEN evaluate off and include it in GET /v1/admin/flags?expired=true; the row is not deleted automatically.
    violation: An expired flag never stays on silently.
    source: FR12.3

  - id: BR5.1
    statement: Raw events are kept six months; ConsentCheck rows 90 days.
    category: policy
    applies_to: AnalyticsEvent, ConsentCheck
    trigger: The nightly retention sweep runs.
    logic: IF an event is older than six months THEN delete it; IF a ConsentCheck is older than 90 days THEN delete it; aggregates and flags are kept.
    violation: Raw data is bounded; aggregates survive.
    source: NFR13 (privacy retention for product analytics)

  - id: BR6.1
    statement: An erasure request deletes a user's pseudonymous events.
    category: policy
    applies_to: AnalyticsEvent
    trigger: Identity calls POST /v1/internal/privacy/erase with a userRef.
    logic: IF the caller's service token is Identity's THEN compute HMAC-SHA256 under every retained salt version for the supplied raw auth user id, delete every event whose userRef matches any version, and return 204; aggregates are unaffected because they hold no userRef.
    violation: A failed erase is reported to Identity and retried; events are never silently retained.
    source: NFR13
```

## Rules summary

| Rule | Category | Statement |
|---|---|---|
| BR1.1 | authorization | Analytics consent required for non-essential events |
| BR1.2 | authorization | Withdrawal stops later events |
| BR1.3 | policy | Consent never assumed |
| BR2.1 | validation | Closed catalogue and per-event allow-lists |
| BR2.2 | constraint | PII/credential patterns dropped or refused |
| BR2.3 | constraint | HMAC-SHA256 salted user hash |
| BR3.1 | calculation | Metric formulas over consented events |
| BR3.2 | policy | Cohorts under 10 users suppressed |
| BR4.1 | validation | Flag owner and expiry required |
| BR4.2 | calculation | Deterministic per-user rollout |
| BR4.3 | policy | Expired flag off and listable |
| BR5.1 | policy | Six-month events, 90-day consent checks |
| BR6.1 | policy | Erasure by userRef |
