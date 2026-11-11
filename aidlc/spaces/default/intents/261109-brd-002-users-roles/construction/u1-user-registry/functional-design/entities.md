# Entities — u1-user-registry

**Upstream:** `unit-of-work.md` (U1 `u1-user-registry`), `requirements.md` (FR14, FR15.7, FR17.1), `components.md` (UserRegistry).

The unit owns the internal user record and the per-subject shredding-key reference. Profile fields live on the user record; preferences (locale, base currency, timezone, notifications) are owned by u4 and are not duplicated here.

```yaml
entities:
  - name: User
    description: One internal user, keyed by the identity provider's subject.
    attributes:
      - name: id
        type: uuid
        required: true
        unique: true
        constraints: "primary identifier; stable for the life of the person's account"
      - name: providerSubjectHash
        type: string
        required: true
        unique: true
        constraints: "SHA-256 of the provider subject; indexed lookup only"
      - name: providerSubjectCipher
        type: string
        required: true
        unique: true
        constraints: "deterministic pgsodium ciphertext of the subject; never logged or returned"
      - name: displayName
        type: string
        required: false
        max: 200
        constraints: "from the provider claim unless the user overrides it"
      - name: email
        type: string
        required: false
        constraints: "RFC 5322 shape; from the provider claim unless overridden"
      - name: emailVerified
        type: boolean
        required: true
        default: false
        constraints: "read from the provider's verified-email claim"
      - name: status
        type: enum
        required: true
        default: pending
        allowed: [pending, active, disabled, pending_deletion, deleted]
        constraints: "pending is the transient provisioning state; the four public states are active, disabled, pending_deletion, deleted; transitions are owned by u5-account-lifecycle"
      - name: createdAt
        type: timestamp
        required: true
      - name: updatedAt
        type: timestamp
        required: true
    entity_constraints:
      - "The provider subject is personal data: it is stored only as a hash plus a deterministic ciphertext, never in plaintext."
      - "The id never changes, even when the provider rotates claims."
      - "Status may be written only through the u5 lifecycle transition request, never by a direct registry update."
    relationships:
      - to: ShreddingKey
        cardinality: one-to-one
        direction: "each user has one per-subject key reference created before the data it protects"
  - name: ShreddingKey
    description: The per-subject pgsodium key reference used by erasure.
    attributes:
      - name: userId
        type: uuid
        required: true
        unique: true
        constraints: "one key reference per user"
      - name: keyRef
        type: string
        required: true
        constraints: "Vault key identifier; the key material never lives in the database"
      - name: createdAt
        type: timestamp
        required: true
    entity_constraints:
      - "The key reference is created before the personal data it protects."
    relationships:
      - to: User
        cardinality: one-to-one
        direction: "belongs to exactly one user"
```

## Summary

| Entity | Identifier | Purpose |
|---|---|---|
| User | id | The internal person: stable id, hashed/ciphertext subject, profile fields, status |
| ShreddingKey | userId | The Vault key reference that erasure uses |

## Decision recorded here

The BRD's four public statuses are kept, and the existing transient `pending` provisioning state remains internal: a principal reports `active`, `disabled`, `pending_deletion`, or `deleted`, and a `pending` user is denied authenticated use except the confirmation step that promotes them. This avoids a fifth public status while not breaking the existing provisioning flow.
