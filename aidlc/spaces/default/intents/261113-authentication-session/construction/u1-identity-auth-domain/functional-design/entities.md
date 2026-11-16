# U1 Identity Auth Domain — Entities

**Unit:** U1 `u1-identity-auth-domain`  
**Inputs:** `unit-of-work.md`, `unit-of-work-story-map.md`, `stories.md`, `requirements.md`, `components.md`, `contract-summary.md`

```yaml
entities:
  - name: InternalUser
    description: Identity-owned projection of a provider subject and the account policy needed by application use cases.
    attributes:
      - { name: user_id, type: opaque identifier, required: true, unique: true }
      - { name: auth_subject, type: encrypted provider subject, required: true, unique: true, constraints: [decryptable only through the per-user shredding key] }
      - { name: role, type: role enum, required: true, allowed: [user, admin, support] }
      - { name: status, type: user status enum, required: true, allowed: [pending, active, disabled, pending_deletion, deleted] }
      - { name: email_verified, type: boolean, required: true, default: false }
      - { name: first_signed_in_at, type: UTC timestamp, required: true, constraints: [immutable after initial provisioning] }
      - { name: locale, type: locale code, required: false }
      - { name: consent_state, type: consent summary, required: true }
      - { name: jurisdiction, type: country code, required: false }
    constraints:
      - A validated provider subject maps to at most one InternalUser; concurrent first requests converge on that row.
      - Disabled and deleted statuses do not grant access, regardless of provider-token validity.
      - Provider credentials, access tokens and refresh tokens are not stored on this entity.
  - name: DeviceSession
    description: Identity-owned, privacy-minimized application view of a provider session.
    attributes:
      - { name: session_id, type: opaque identifier, required: true, unique: true }
      - { name: user_id, type: identifier, required: true, references: InternalUser.user_id }
      - { name: device_label, type: bounded display string, required: true, max: 120 }
      - { name: coarse_location, type: bounded coarse location, required: false, max: 80 }
      - { name: first_seen_at, type: UTC timestamp, required: true }
      - { name: last_seen_at, type: UTC timestamp, required: true }
      - { name: revoked_at, type: UTC timestamp, required: false }
      - { name: is_current, type: boolean, required: true }
      - { name: state, type: session state enum, required: true, allowed: [active, revoked] }
    constraints:
      - One session belongs to exactly one InternalUser.
      - A session marked revoked has revoked_at set; an active session has revoked_at unset.
      - The current-session marker is derived from the validated caller context, not request input.
      - No raw IP, full user-agent string, token, or full device fingerprint is stored or returned.
  - name: SignupGate
    description: Trusted-side record of consent and jurisdiction checks authorizing one provisioning attempt.
    attributes:
      - { name: gate_id, type: opaque identifier, required: true, unique: true }
      - { name: auth_subject, type: encrypted provider subject, required: true }
      - { name: jurisdiction, type: country code, required: true }
      - { name: terms_accepted, type: boolean, required: true, allowed: [true] }
      - { name: risk_acknowledged, type: boolean, required: true, allowed: [true] }
      - { name: age_confirmed, type: boolean, required: true, allowed: [true] }
      - { name: outcome, type: gate outcome enum, required: true, allowed: [passed, refused, consumed] }
      - { name: created_at, type: UTC timestamp, required: true }
      - { name: consumed_at, type: UTC timestamp, required: false }
      - { name: idempotency_key, type: opaque operation key, required: true }
    constraints:
      - A passed gate authorization is bound to its subject and one signup attempt and cannot authorize a second account.
      - A refused jurisdiction or incomplete consent cannot provision an account.
  - name: MfaPolicyState
    description: Identity's policy projection for whether the caller must reach AAL2; provider owns factor secrets and factor operations.
    attributes:
      - { name: user_id, type: identifier, required: true, unique: true, references: InternalUser.user_id }
      - { name: mfa_required, type: boolean, required: true }
      - { name: mfa_enrolled, type: boolean, required: true }
      - { name: aal, type: assurance level enum, required: true, allowed: [aal1, aal2] }
      - { name: exchange_credential_present, type: boolean, required: true }
      - { name: changed_at, type: UTC timestamp, required: true }
    constraints:
      - A user with an exchange credential and no verified factor is limited to MFA and settings capabilities.
      - A client-provided role, factor status, or AAL is never authority; policy uses validated claims and trusted provider state.
      - TOTP secret, challenge code and recovery codes are not persisted in Identity.
  - name: RecoveryCase
    description: Audited support-assisted account recovery lifecycle; identity re-proof occurs without the application receiving credentials or factor secrets.
    attributes:
      - { name: recovery_case_id, type: opaque identifier, required: true, unique: true }
      - { name: user_id, type: identifier, required: false, references: InternalUser.user_id, constraints: [not disclosed to unauthenticated requester] }
      - { name: requester_contact, type: protected contact reference, required: true }
      - { name: reason, type: bounded text, required: true, max: 1000 }
      - { name: status, type: recovery state enum, required: true, allowed: [pending_approval, approved, identity_proof_failed, completed, expired, rejected] }
      - { name: requested_at, type: UTC timestamp, required: true }
      - { name: approved_by, type: identifier, required: false, references: InternalUser.user_id }
      - { name: completed_at, type: UTC timestamp, required: false }
      - { name: idempotency_key, type: opaque operation key, required: true }
    constraints:
      - The approver is a distinct authorized administrator under the existing support-grant approval rule; requester and approver cannot be the same actor.
      - Failed proof leaves account and sessions unchanged; no session is issued before successful completion.
      - Responses to known and unknown accounts are indistinguishable.
      - Passwords, provider secrets, factor seeds, recovery codes and proof evidence are not stored in the case.
  - name: LoginEvent
    description: Minimal Identity-side event used to identify a first-seen device and request an in-app alert after successful sign-in.
    attributes:
      - { name: event_id, type: opaque identifier, required: true, unique: true }
      - { name: user_id, type: identifier, required: true, references: InternalUser.user_id }
      - { name: device_key, type: keyed opaque digest, required: true, constraints: [not reversible to a full fingerprint] }
      - { name: device_label, type: bounded display string, required: true, max: 120 }
      - { name: coarse_location, type: bounded coarse location, required: false, max: 80 }
      - { name: occurred_at, type: UTC timestamp, required: true }
      - { name: alert_eligible, type: boolean, required: true }
      - { name: notification_status, type: delivery outcome enum, required: true, allowed: [not_required, pending, delivered, failed] }
      - { name: idempotency_key, type: opaque operation key, required: true }
    constraints:
      - U1 alert eligibility is based on first-seen device only; unusual-location detection is excluded pending a measurable product rule.
      - Notification failure does not undo sign-in; delivery is synchronous, bounded and idempotent through the existing Notification port.
      - No token, credential, IP, full user-agent or full device fingerprint enters this entity or the notification request.
  - name: SecurityAuditOutboxEntry
    description: Identity-local transactional intent for delivering a security event to the append-only Audit owner.
    attributes:
      - { name: event_id, type: opaque identifier, required: true, unique: true }
      - { name: actor, type: opaque actor reference or system, required: true }
      - { name: action, type: bounded action name, required: true }
      - { name: target, type: opaque target reference, required: true }
      - { name: occurred_at, type: UTC timestamp, required: true }
      - { name: context, type: sanitized structured context, required: true }
      - { name: outcome, type: outcome enum, required: true, allowed: [success, refused, failed] }
      - { name: delivery_state, type: delivery enum, required: true, allowed: [pending, delivered, dead_lettered] }
      - { name: attempt_count, type: non-negative integer, required: true, default: 0 }
    constraints:
      - State change and audit intent commit in the same Identity database transaction.
      - Delivery uses the existing bounded retry/idempotency behavior; dead-lettered work is surfaced, never silently dropped.
      - Context excludes credentials, tokens, factor seeds, recovery codes and full device fingerprints.
relationships:
  - { from: InternalUser, to: DeviceSession, cardinality: one-to-many, direction: InternalUser owns DeviceSession }
  - { from: InternalUser, to: MfaPolicyState, cardinality: one-to-zero-or-one, direction: InternalUser has policy projection }
  - { from: InternalUser, to: RecoveryCase, cardinality: one-to-many, direction: InternalUser may have recovery cases }
  - { from: InternalUser, to: LoginEvent, cardinality: one-to-many, direction: InternalUser has sign-in observations }
  - { from: InternalUser, to: SecurityAuditOutboxEntry, cardinality: one-to-many, direction: audit actor/target references are opaque }
  - { from: SignupGate, to: InternalUser, cardinality: zero-or-one-to-one, direction: passed single-use gate permits one provisioning attempt }
```

Identity owns the user's internal projection and policy, application session view, signup-gate result, recovery case, first-seen-device decision, and transactional audit intent. Supabase Auth remains authoritative for credentials, token lifecycle, email verification, TOTP factors and provider session validity; Notification and Audit retain ownership of their records.