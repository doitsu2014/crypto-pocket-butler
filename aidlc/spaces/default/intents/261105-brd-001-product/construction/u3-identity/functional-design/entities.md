# Entity Model — u3-identity

The YAML block below is the source of truth for the entity model. The prose after it is a short
human-readable summary. Technology-agnostic: no SQL, no framework, no language types.

Inputs: the Identity unit in `inception/units-generation/unit-of-work.md`, the requirements in
`inception/requirements-analysis/requirements.md`, and the Identity component in
`inception/domain-design/components.md`, which supplies the entity set.

```yaml
entities:
  - name: User
    description: The internal identity record for one person, keyed to Supabase Auth.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
        description: Internal user identifier used by every service.
      - name: authUserId
        type: uuid
        required: true
        unique: true
        description: The Supabase Auth subject this record is keyed to.
      - name: role
        type: enum
        required: true
        allowed_values: [user, admin, support]
        default: user
        description: Role copied from app_metadata.role; drives admin and support capabilities.
      - name: status
        type: enum
        required: true
        allowed_values: [pending, active, deleted]
        default: pending
        description: Lifecycle state; pending blocks portfolio data, deleted blocks sign-in.
      - name: locale
        type: string
        required: true
        allowed_values: [en, vi]
        default: en
        description: UI language for the user.
      - name: baseCurrency
        type: string
        required: true
        min_length: 3
        max_length: 3
        default: USD
        description: ISO 4217 base currency used for valuation.
      - name: timezone
        type: string
        required: true
        default: UTC
        description: IANA timezone for rendering timestamps.
      - name: createdAt
        type: timestamp
        required: true
        description: When the internal record was created.
      - name: updatedAt
        type: timestamp
        required: true
        description: When the record last changed.
      - name: anonymisedAt
        type: timestamp
        required: false
        description: Set when account deletion anonymised the personal fields.
    constraints:
      - authUserId is unique and immutable.
      - role is one of user, admin, support; it changes only from a signed role source.
      - status transitions are pending -> active and any state -> deleted; once deleted, no transition back.
      - baseCurrency is an upper-case ISO 4217 code; timezone is a valid IANA name.
    external_facts:
      - name: hasExchangeCredentials
        description: Read from the validated JWT claim app_metadata.has_exchange_credentials, maintained by the Account unit when a credential is stored. Not stored on User.

  - name: ConsentRecord
    description: One recorded decision for one consent purpose, with its full history.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: userId
        type: uuid
        required: true
        references:
          entity: User
          relationship: many ConsentRecords belong to one User
      - name: purpose
        type: enum
        required: true
        allowed_values: [analytics, marketing, disclosures]
        description: The purpose the decision is about. C3 exposes analytics and marketing.
      - name: granted
        type: boolean
        required: true
        description: Whether consent is granted for this purpose.
      - name: recordedAt
        type: timestamp
        required: true
        description: When the decision was recorded.
      - name: source
        type: enum
        required: true
        allowed_values: [onboarding, settings, support]
        default: onboarding
        description: Where the decision was made.
    constraints:
      - Records are append-only; a change inserts a new record and never edits an earlier one.
      - The current state is the newest record per (userId, purpose).
      - No purpose is granted by default; a missing record means not granted.

  - name: SupportAccessGrant
    description: A time-bound, approved, reason-recorded support read grant.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: userId
        type: uuid
        required: true
        references:
          entity: User
          relationship: many grants belong to one User
      - name: supportAgentId
        type: uuid
        required: true
        description: The support user who requested the grant.
      - name: reason
        type: text
        required: true
        min_length: 1
        description: The recorded reason for the access.
      - name: status
        type: enum
        required: true
        allowed_values: [pending_approval, active, expired, revoked]
        default: pending_approval
        description: Grant lifecycle state.
      - name: requestedAt
        type: timestamp
        required: true
      - name: approvedBy
        type: uuid
        required: false
        description: The second approver; must be an admin distinct from supportAgentId.
      - name: approvedAt
        type: timestamp
        required: false
      - name: expiresAt
        type: timestamp
        required: false
        description: Set on approval; default 60 minutes after approval.
      - name: revokedAt
        type: timestamp
        required: false
    constraints:
      - reason is non-blank.
      - approvedBy must hold the admin role and must differ from supportAgentId.
      - expiresAt is at most 60 minutes after approvedAt unless an admin set a shorter bound.
      - Access ends at expiresAt or when a revoke is recorded, whichever comes first.
      - A grant never widens access beyond read-only account metadata.

  - name: DeletionRequest
    description: One account-deletion request and its execution state.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: userId
        type: uuid
        required: true
        references:
          entity: User
          relationship: at most one active request per User
      - name: requestedAt
        type: timestamp
        required: true
      - name: status
        type: enum
        required: true
        allowed_values: [requested, in_progress, completed, failed]
        default: requested
      - name: retainedCategories
        type: list
        required: false
        description: Categories kept under a legal basis, each with its basis label.
      - name: completedAt
        type: timestamp
        required: false
      - name: failureReason
        type: text
        required: false
    constraints:
      - At most one request in requested or in_progress per user.
      - Completed requires every personal category to be anonymised and every retained category labelled.
      - A failed request can be retried, opening a new execution attempt of the same request.

  - name: ExportRequest
    description: One data-export request and its assembly progress.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: userId
        type: uuid
        required: true
        references:
          entity: User
          relationship: many export requests belong to one User
      - name: requestedAt
        type: timestamp
        required: true
      - name: status
        type: enum
        required: true
        allowed_values: [pending, building, ready, failed, expired]
        default: pending
      - name: progress
        type: integer
        required: true
        min: 0
        max: 100
        default: 0
        description: Percentage of contributing services that have answered.
      - name: assemblyDeadline
        type: timestamp
        required: true
        description: When an unfinished request becomes expired.
      - name: fileReference
        type: string
        required: false
        description: Reference to the assembled archive in the export bucket, resolved to a signed download.
      - name: downloadExpiresAt
        type: timestamp
        required: false
        description: When the signed download stops working; set when the request becomes ready.
      - name: failureReason
        type: text
        required: false
      - name: completedAt
        type: timestamp
        required: false
    constraints:
      - Only the owning user can read the request and its download.
      - progress rises monotonically while building.
      - ready requires every contributing service to have answered and a file reference to exist.
      - assemblyDeadline applies while pending/building; downloadExpiresAt applies once ready.
      - An expired request keeps its record but no longer serves the file; the archive is deleted from the bucket after downloadExpiresAt.

  - name: JurisdictionRule
    description: A launch-market rule applied before sign-up.
    attributes:
      - name: code
        type: string
        required: true
        unique: true
        description: ISO 3166-1 alpha-2 country code.
      - name: allowed
        type: boolean
        required: true
        description: Whether sign-up is allowed from this jurisdiction.
      - name: minimumAge
        type: integer
        required: true
        min: 0
        description: The age a person must declare to sign up here.
      - name: disclosuresRequired
        type: boolean
        required: true
        default: true
        description: Whether the launch disclosures must be acknowledged.
      - name: updatedAt
        type: timestamp
        required: true
    constraints:
      - code is unique.
      - A jurisdiction with no rule is treated as blocked until an allowed rule is added.
      - The Compliance specialist seeds and maintains the table; an admin changes it through the audited admin path.
    seed:
      allowed_18: [VN, GB, IE, DE, FR, NL, ES, IT, PT, BE, AT, DK, SE, FI, PL, CZ, SK, SI, HR, RO, BG, EE, LV, LT, LU, MT, CY, GR, HU]
      note: Every seeded market has allowed true, minimumAge 18, and disclosuresRequired true. The list is the launch scope from requirements.md; a country not listed is blocked until Compliance adds it.

  - name: ShreddingKey
    description: The per-subject key that protects Identity's personal fields, held in Supabase Vault.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: userId
        type: uuid
        required: true
        references:
          entity: User
          relationship: one active key per User
      - name: vaultReference
        type: string
        required: true
        description: The Vault secret reference; key material never leaves Vault.
      - name: status
        type: enum
        required: true
        allowed_values: [active, destroyed]
        default: active
      - name: createdAt
        type: timestamp
        required: true
      - name: destroyedAt
        type: timestamp
        required: false
    constraints:
      - Created when the User is created; one active key per user.
      - Protects Identity's personal fields only (User personal columns and any Identity-stored personal data).
      - Records retained by other services under a legal basis are not encrypted under this key; they stay readable after destruction.
      - Destroyed only after every contributing service confirms anonymisation.

  - name: DeviceSession
    description: Identity's own view of a device session, used when Supabase Auth exposes no session list.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
      - name: userId
        type: uuid
        required: true
        references:
          entity: User
          relationship: many device sessions belong to one User
      - name: authSessionId
        type: string
        required: true
        description: The Supabase Auth session identifier.
      - name: device
        type: string
        required: false
        description: A short client label from the user agent.
      - name: location
        type: string
        required: false
        description: Coarse city/country from the gateway's IP geolocation; never the full IP.
      - name: lastActivityAt
        type: timestamp
        required: true
      - name: revokedAt
        type: timestamp
        required: false
    constraints:
      - One row per (userId, authSessionId), updated on each authenticated request.
      - location is coarse and is not retained after the session ends.
      - Revocation calls Supabase Auth and marks the row revoked.

relationships:
  - from: User
    to: ConsentRecord
    cardinality: one-to-many
    direction: User owns ConsentRecord
  - from: User
    to: SupportAccessGrant
    cardinality: one-to-many
    direction: User owns SupportAccessGrant
  - from: User
    to: DeletionRequest
    cardinality: one-to-one-active
    direction: User owns DeletionRequest
  - from: User
    to: ExportRequest
    cardinality: one-to-many
    direction: User owns ExportRequest
  - from: User
    to: ShreddingKey
    cardinality: one-to-one
    direction: User owns ShreddingKey
  - from: User
    to: DeviceSession
    cardinality: one-to-many
    direction: User owns DeviceSession
```

## Summary

Identity owns the internal `User` record that every service keys on, plus the records around it:
the append-only `ConsentRecord` history, the time-bound `SupportAccessGrant`, the
`DeletionRequest` and `ExportRequest` state machines, the per-subject `ShreddingKey` reference, and
the `DeviceSession` view used for FR1.4 when Supabase Auth exposes no session list.
`JurisdictionRule` is configuration applied before sign-up. Sessions, MFA, credentials, and factors
stay in Supabase Auth and are not duplicated here beyond the session view.

**Ownership note.** `ExportRequest`, `JurisdictionRule`, `ShreddingKey`, and `DeviceSession` are
added by this unit; they are not in the shared Entity Ownership table in
`inception/domain-design/components.md`. They belong to Identity and are recorded here so the data
model is complete.
