# U1 Identity Auth Domain — Business Rules

**Unit:** U1 `u1-identity-auth-domain`  
**Inputs:** `unit-of-work.md`, `unit-of-work-story-map.md`, `stories.md`, `requirements.md`, `components.md`, `contract-summary.md`

```yaml
rules:
  - id: BR1.1
    statement: A validated provider subject maps to exactly one internal user, including concurrent first requests.
    category: constraint
    applies_to: InternalUser provisioning
    trigger: First authenticated request for a provider subject
    logic: IF no internal user exists THEN create or retrieve the unique subject mapping atomically.
    violation_behaviour: Return a safe recoverable service error; do not create duplicate users.
    source: FR3.3; AC9.2.2
  - id: BR1.2
    statement: A disabled or deleted internal user is denied even when the provider token remains valid.
    category: authorization
    applies_to: Principal resolution and protected requests
    trigger: Every authenticated request
    logic: IF InternalUser.status is disabled or deleted THEN deny access; otherwise apply the remaining policy.
    violation_behaviour: Return a non-revealing authorization refusal and record the status transition as audited.
    source: FR3.4; AC9.2.1; AC9.2.3
  - id: BR1.3
    statement: The absolute session lifetime is 30 days from first sign-in; refresh does not reset its anchor.
    category: policy
    applies_to: Session refresh eligibility
    trigger: Refresh-token use
    logic: IF current time is at or beyond first_signed_in_at plus 30 days THEN refuse refresh and require fresh sign-in.
    violation_behaviour: Clear the browser session through the consumer flow; do not issue a replacement token.
    source: FR2.1; AC2.1.3
  - id: BR1.4
    statement: A successful refresh rotates the provider refresh token and does not alter first_signed_in_at.
    category: constraint
    applies_to: Session refresh lifecycle
    trigger: Successful provider refresh
    logic: IF refresh succeeds THEN accept only the newly issued provider token pair and preserve the original absolute-cap timestamp.
    violation_behaviour: Treat invalid or expired refresh as terminal for the session; provider outage remains a recoverable failure.
    source: FR2.1; FR2.2; AC2.1.1; AC2.1.2; AC2.1.5
  - id: BR1.5
    statement: Application sign-out clears the in-memory application session before terminating the provider session.
    category: policy
    applies_to: Sign-out
    trigger: User confirms sign-out
    logic: Clear local session state first, then invoke provider sign-out; do not restore the local session if the provider call fails.
    violation_behaviour: Surface provider failure while keeping the user signed out locally.
    source: FR4.1; AC2.4.1; AC2.4.2; AC2.4.3
  - id: BR1.6
    statement: Every security-sensitive state transition creates a sanitized audit intent in the same transaction as the Identity state change.
    category: constraint
    applies_to: Sign-out, MFA, session revocation, signup gates, recovery and account-policy changes
    trigger: A security-sensitive transition is accepted or refused
    logic: Insert the actor, action, target, timestamp, context and outcome into the Identity outbox atomically with the relevant mutation.
    violation_behaviour: Roll back the state mutation if its required local audit intent cannot be committed; never claim an unaudited success.
    source: FR4.1; FR5.4; NFR5; AC2.4.4; AC3.1.4; AC3.3.4; AC5.3.3; AC7.2.4
  - id: BR1.7
    statement: MFA policy and factor status are trusted-side decisions; provider factor secrets and codes are never persisted or audited.
    category: authorization
    applies_to: MFA enrolment, verification, AAL policy and sensitive changes
    trigger: Enrolment, challenge, policy evaluation or sensitive account change
    logic: Identity consults validated provider assurance and trusted factor state; a verified factor produces AAL2, while an exchange-credential user without a verified factor is limited to MFA and settings capabilities.
    violation_behaviour: Refuse invalid or expired verification without changing factor policy; map provider outage to safe recoverable service-unavailable behavior.
    source: FR5.1; FR5.2; FR5.3; FR5.4; AC3.1.1; AC3.1.2; AC3.1.3; AC3.1.4; AC3.2.1; AC3.2.3; AC3.2.4; AC3.2.5; AC3.3.1; AC3.3.2; AC3.3.3; AC3.3.4
  - id: BR1.8
    statement: Sensitive credential, MFA-method and recovery-code changes require current AAL2 proof and revoke every other session on success.
    category: authorization
    applies_to: Password, MFA-method and recovery-code changes
    trigger: Sensitive change request
    logic: IF current valid second-factor proof is absent THEN refuse without mutation; IF change succeeds THEN revoke every other provider and local session, retaining only the initiating session.
    violation_behaviour: Leave state unchanged on failed proof; explicitly report partial revocation failure without claiming all sessions ended.
    source: FR5.3; FR4.6; AC3.3.1; AC3.3.2; AC3.3.3; AC5.4.1; AC5.4.2; AC5.4.3
  - id: BR1.9
    statement: Signup provisioning requires a trusted-side, supported-jurisdiction gate with explicit positive consent and single-use authorization.
    category: policy
    applies_to: SignupGate and account provisioning
    trigger: Signup attempt
    logic: IF jurisdiction is unsupported or undetermined, any required choice is false/missing, or gate authorization is absent, mismatched or consumed THEN refuse provisioning.
    violation_behaviour: Return a safe validation or jurisdiction refusal; never create an account or permit replay of a passed gate.
    source: FR6.1; FR6.2; FR6.3; FR6.4; FR6.5; AC4.1.1; AC4.1.2; AC4.1.3; AC4.1.4; AC4.1.5; AC4.2.1; AC4.2.2; AC4.2.3; AC4.2.4
  - id: BR1.10
    statement: Exchange-credential authorization requires trusted email verification and the applicable MFA assurance level.
    category: authorization
    applies_to: Exchange credential addition and principal policy
    trigger: Exchange credential operation or protected request
    logic: IF email is unverified OR required AAL2 is absent THEN refuse the credential operation or restrict the principal to allowed MFA/settings capabilities.
    violation_behaviour: Offer the safe verification or MFA route; do not trust browser-supplied verification or factor state.
    source: FR4.3; FR5.2; FR6.6; FR6.7; AC3.2.1; AC3.2.2; AC3.2.3; AC4.3.1; AC4.3.4; AC4.3.5
  - id: BR1.11
    statement: Session ownership applies to all user access; provider revocation precedes local revocation and failures preserve local state.
    category: authorization
    applies_to: DeviceSession read, revoke-one and revoke-others
    trigger: Session list or revocation request
    logic: IF caller owns the session THEN list or act; for revocation, call provider first, then update local state and write audit intent. Replaying the same successful idempotency key returns the original outcome.
    violation_behaviour: Non-owner gets non-disclosing not-found; provider failure leaves local row unchanged and is recoverable; bulk partial failure identifies remaining sessions.
    source: FR4.2; FR4.3; FR4.4; FR4.5; FR4.7; FR3.8; AC5.1.1; AC5.1.2; AC5.1.3; AC5.1.4; AC5.2.1; AC5.2.2; AC5.2.3; AC5.2.4; AC5.2.5; AC5.2.6; AC5.3.1; AC5.3.2; AC5.3.3
  - id: BR1.12
    statement: A successful credential or recovery change invalidates all other sessions, subject to the accepted stateless access-token residual window.
    category: policy
    applies_to: Credential changes and completed recovery
    trigger: Password/MFA change or recovery completion
    logic: Revoke other provider sessions before marking local rows revoked; the initiating session is retained for a credential change, while recovery issues no session until completion and revokes all pre-existing sessions.
    violation_behaviour: Surface per-session failures and never claim complete revocation; the remaining unexpired access token may remain usable for up to one hour as the documented limit.
    source: FR4.6; FR8.2; FR8.3; FR3.8; AC5.4.1; AC5.4.2; AC5.4.3; AC7.2.1; AC7.2.2
  - id: BR1.13
    statement: Only a first-seen device triggers the U1 in-app login-alert request; unusual-location detection is not implemented by this Unit.
    category: policy
    applies_to: LoginEvent alert eligibility
    trigger: Successful sign-in
    logic: IF the privacy-preserving device key has not been seen for this user THEN submit one idempotent notification request; otherwise do not create another alert.
    violation_behaviour: Notification failure is surfaced to operations and does not roll back sign-in; repeated delivery with the same key does not duplicate alerts.
    source: FR7.1; FR7.2; FR7.3; AC6.1.1; AC6.1.2; AC6.1.3; AC6.1.4; AC6.1.5
  - id: BR1.14
    statement: Recovery is non-enumerating, support-authorized and requires a distinct administrator's approval under the existing support-grant policy.
    category: authorization
    applies_to: RecoveryCase creation and completion
    trigger: Recovery request or support action
    logic: Return the same neutral response for known and unknown accounts; require an authorized support actor and a different authorized administrator to approve; issue no session before successful proof and approval.
    violation_behaviour: Deny unauthorized or self-approved completion and audit the refusal; failed proof leaves account and sessions unchanged.
    source: FR8.1; FR8.2; FR8.3; FR8.4; AC7.1.1; AC7.1.2; AC7.1.3; AC7.1.4; AC7.2.1; AC7.2.2; AC7.2.3; AC7.2.4; AC7.3.1; AC7.3.2; AC7.3.3
  - id: BR1.15
    statement: Provider outages do not create a fallback identity path; current sessions work only while their validated access token remains valid.
    category: policy
    applies_to: Sign-in, refresh, MFA challenge and protected requests
    trigger: Provider or JWKS dependency outage
    logic: IF an already-issued access token remains within its validated lifetime THEN continue local protected work; otherwise refuse safely. New sign-in, signup, refresh and MFA challenge fail with a non-revealing recoverable response.
    violation_behaviour: Bound retries to transient failures only; return a safe service-unavailable problem without provider details and resume normally after recovery.
    source: FR3.7; FR10.1; FR10.2; FR10.3; AC9.3.1; AC9.3.2; AC9.3.3; AC9.3.4; AC9.3.5
  - id: BR1.16
    statement: Audit and notification payloads exclude secrets and identifying device details beyond approved coarse labels.
    category: constraint
    applies_to: SecurityAuditOutboxEntry and LoginEvent delivery
    trigger: Audit or notification intent creation
    logic: Include only actor, action, opaque target, timestamp, sanitized context and outcome; never include tokens, credentials, factor seeds, recovery codes, full fingerprints, raw IP or full user agent.
    violation_behaviour: Reject unsafe payload before delivery and surface the failure; never log the prohibited value.
    source: FR5.4; FR7.2; FR8.2; NFR5; NFR6; AC3.1.4; AC3.3.4; AC6.1.3; AC7.2.3; AC7.2.4
```

| Rule | Applies to | Main behavior |
|---|---|---|
| BR1.1–BR1.2 | Principal and user lifecycle | Idempotent projection; disabled/deleted users denied |
| BR1.3–BR1.5 | Session lifecycle | Absolute cap from first sign-in; refresh rotation; local-first sign-out |
| BR1.6–BR1.8 | Audit and MFA | Atomic audit intent; trusted AAL2 policy; sensitive changes revoke peers |
| BR1.9–BR1.10 | Signup and eligibility | Trusted single-use gates; verified email and MFA policy enforced server-side |
| BR1.11–BR1.12 | Device sessions and recovery | Ownership; provider-first revocation; partial failures explicit |
| BR1.13–BR1.14 | Alerts and recovery | First-seen-device alert; existing distinct-admin recovery approval |
| BR1.15–BR1.16 | Outages and data disclosure | No fallback auth; sanitized audit and notification records |
