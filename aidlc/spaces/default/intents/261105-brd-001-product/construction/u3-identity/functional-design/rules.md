# Business Rules — u3-identity

The YAML block below is the source of truth for the business rules. The table after it is a short
human-readable summary. Rules are grouped: BR1 identity lifecycle, BR2 MFA and access, BR3 consent
and preferences, BR4 support access, BR5 export, BR6 deletion, BR7 jurisdiction, BR8 sessions.

Inputs: the Identity unit in `inception/units-generation/unit-of-work.md`, the requirements in
`inception/requirements-analysis/requirements.md`, and the Identity component in
`inception/domain-design/components.md`.

```yaml
rules:
  - id: BR1.1
    statement: Every request is authenticated by Identity itself against Supabase JWKS, and an internal user is created on the first authenticated request.
    category: policy
    applies_to: User
    trigger: A request arrives carrying a Supabase JWT.
    logic: IF Identity validates the JWT (signature, issuer, audience, expiry) against Supabase JWKS AND no User exists for authUserId THEN create the User with role from app_metadata.role, status pending, locale en, baseCurrency USD, timezone UTC; IF validation fails THEN refuse with 401 and write nothing.
    violation: No principal is returned; the request is refused with 401.
    source: FR1.1, FR13.1

  - id: BR1.2
    statement: Role is taken only from app_metadata.role and is least-privilege.
    category: authorization
    applies_to: User
    trigger: A JWT is resolved or a role claim changes.
    logic: IF app_metadata.role is one of user, admin, support THEN apply it; ELSE treat the role as user and grant no admin or support capability.
    violation: An unknown or missing role never grants admin or support; the denial is audited.
    source: FR1.5

  - id: BR1.3
    statement: User status gates access.
    category: authorization
    applies_to: User
    trigger: A principal is resolved.
    logic: IF status is pending THEN block portfolio data and route to verification/enrolment; IF status is deleted THEN refuse sign-in and explain the state; ELSE allow according to role.
    violation: The request is refused with the status explained; no personal data is returned.
    source: FR1.1

  - id: BR1.4
    statement: Ownership is enforced on every read and write of a user-owned resource.
    category: authorization
    applies_to: ConsentRecord, SupportAccessGrant, DeletionRequest, ExportRequest
    trigger: A user-owned resource is requested by id, list, search, or export.
    logic: IF the caller is not the owning user AND does not hold an active admin or support capability for that read THEN refuse.
    violation: The response is not-found or forbidden and no data is returned.
    source: FR1.6

  - id: BR1.5
    statement: A verified user becomes active on the first authenticated request.
    category: policy
    applies_to: User
    trigger: A principal resolves from a validated JWT while the internal status is pending.
    logic: IF the validated JWT shows a confirmed email (Supabase's confirmed/email_confirmed claim) AND, where the user has exchange credentials, the aal claim is aal2 THEN set status active; ELSE leave pending and return the verification or MFA requirement.
    violation: A pending user never reaches portfolio data; the response says which step is missing.
    source: FR1.1, US1.1

  - id: BR2.1
    statement: MFA is required for users with stored exchange credentials; both facts come from the validated JWT.
    category: policy
    applies_to: User
    trigger: A principal resolves from a validated JWT.
    logic: IF the JWT claim app_metadata.has_exchange_credentials is true (maintained by the Account unit when a credential is stored) AND the JWT aal claim is not aal2 THEN refuse portfolio data and return the enrolment requirement.
    violation: Portfolio data is not returned; the user is routed to MFA enrolment.
    source: FR1.2, NFR10

  - id: BR2.2
    statement: MFA and recovery codes are managed by Supabase Auth, never stored by Identity.
    category: constraint
    applies_to: User
    trigger: MFA enrolment or recovery-code display.
    logic: IF enrolment completes THEN Supabase Auth shows recovery codes once to the user; Identity stores neither codes nor factors.
    violation: Identity never persists a factor or a recovery code.
    source: FR1.2, US7.1

  - id: BR2.3
    statement: A wrong MFA code is refused and rate-limited.
    category: policy
    applies_to: User
    trigger: An MFA verification attempt with a wrong code.
    logic: IF the code is wrong THEN refuse AND count the attempt against the stricter auth rate limit.
    violation: The attempt is refused with 401 and later attempts are throttled with Retry-After.
    source: US7.1, NFR11

  - id: BR2.4
    statement: MFA enrolment is offered to every user, not only to users with exchange credentials.
    category: policy
    applies_to: User
    trigger: A user opens security settings.
    logic: IF the user has no enrolled factor THEN offer enrolment through Supabase Auth; enrolment is optional for a user with no exchange credential.
    violation: The offer is always available; declining it does not block the account.
    source: FR1.2

  - id: BR3.1
    statement: Consent is deny-by-default and never pre-ticked.
    category: policy
    applies_to: ConsentRecord
    trigger: Analytics or marketing processing is about to start, or the consent screen is shown.
    logic: IF no granted ConsentRecord exists for the purpose THEN treat consent as not granted and do not process; the consent screen starts every purpose unticked.
    violation: No non-essential analytics or marketing event is produced.
    source: FR8.3

  - id: BR3.2
    statement: Consent changes are append-only and take effect on the next event.
    category: policy
    applies_to: ConsentRecord
    trigger: The user grants or withdraws consent.
    logic: IF the user changes a purpose THEN insert a new ConsentRecord with the new decision; later events read the newest record per purpose.
    violation: An earlier record is never edited or deleted.
    source: FR8.3

  - id: BR3.3
    statement: Base currency, locale, and timezone are validated and applied to the user's views.
    category: validation
    applies_to: User
    trigger: The user changes a preference.
    logic: IF baseCurrency is an ISO 4217 code AND locale is en or vi AND timezone is a valid IANA name THEN save it and apply it on the next read; ELSE refuse with a field-level reason.
    violation: The change is refused with 422 and the previous preference remains.
    source: FR4.5, FR9.1

  - id: BR4.1
    statement: A support grant needs a reason and a distinct admin second approver.
    category: authorization
    applies_to: SupportAccessGrant
    trigger: A support user requests access to one user's account metadata.
    logic: IF reason is non-blank AND an admin other than the requester approves THEN activate the grant; ELSE leave it pending_approval or refuse.
    violation: The requester cannot self-approve; the request stays pending or is refused.
    source: FR11.2, US9.3

  - id: BR4.2
    statement: Support access expires by default 60 minutes after approval.
    category: constraint
    applies_to: SupportAccessGrant
    trigger: A grant is approved, or a support read is attempted.
    logic: IF now is before expiresAt AND the grant is active THEN read the named metadata; ELSE mark the grant expired and refuse.
    violation: An expired or revoked grant yields no data; the denial is audited.
    source: FR11.2, US9.3

  - id: BR4.3
    statement: Every support read is recorded in the audit trail with the grant.
    category: policy
    applies_to: SupportAccessGrant
    trigger: A support read is served.
    logic: IF a support read succeeds THEN write an audit event naming the support agent, the target user, the grant, and the outcome before the answer is returned.
    violation: When the audit write fails, the read is refused rather than served unrecorded.
    source: FR11.2, US9.3, NFR19

  - id: BR4.4
    statement: Support access is read-only and limited to account metadata.
    category: authorization
    applies_to: SupportAccessGrant
    trigger: A support read is attempted.
    logic: IF the target is credentials, portfolio contents, or any write THEN refuse regardless of the grant.
    violation: The request is refused and the attempt is audited.
    source: FR11.2

  - id: BR5.1
    statement: An export request runs asynchronously and reports progress.
    category: policy
    applies_to: ExportRequest
    trigger: The owning user requests an export.
    logic: IF the request is authorized THEN create the ExportRequest as pending, call each contributing service, advance progress monotonically, assemble one archive, and mark it ready with a signed download.
    violation: A failed contribution marks the request failed with a reason; a partial archive is never presented as complete.
    source: FR8.1, US7.3

  - id: BR5.2
    statement: An export contains the user's data and no credentials or tokens.
    category: constraint
    applies_to: ExportRequest
    trigger: The archive is assembled.
    logic: IF the archive is built THEN include accounts, portfolios, transactions, and snapshots, and exclude every credential, token, and secret.
    violation: The archive is refused and rebuilt; a secret in the archive is a security failure.
    source: FR8.1, US7.3, NFR9

  - id: BR5.3
    statement: An export is readable only by its owner and its download link expires.
    category: authorization
    applies_to: ExportRequest
    trigger: An export status or download is requested.
    logic: IF the caller is the owning user AND the request is ready AND now is before expiresAt THEN serve the signed link; ELSE refuse.
    violation: The response is not-found or forbidden; an expired link serves nothing.
    source: FR1.6, NFR13

  - id: BR5.4
    statement: Export requests and downloads are audited privacy events.
    category: policy
    applies_to: ExportRequest
    trigger: An export is requested, completes, or is downloaded.
    logic: IF the event occurs THEN write an audit event naming the owner, the export id, the action, the time, and the outcome.
    violation: A failed audit write is logged and the action is refused where the privacy rule requires the record.
    source: NFR13, NFR19

  - id: BR6.1
    statement: Account deletion runs as a confirmed, tracked request.
    category: policy
    applies_to: DeletionRequest
    trigger: The user confirms account deletion.
    logic: IF the user confirms THEN create the DeletionRequest as requested, move to in_progress, and finish completed or failed; at most one active request per user.
    violation: An unconfirmed request is not executed; a repeated confirmation reuses the active request.
    source: FR8.2, US7.4

  - id: BR6.2
    statement: Deletion anonymises personal data and labels what retention keeps.
    category: policy
    applies_to: DeletionRequest
    trigger: A deletion request is executed.
    logic: IF each contributing service answers THEN replace personal fields with anonymised values, keep only the labelled legal-retention records, and record retainedCategories with their basis.
    violation: The request stays in_progress or fails with the reason; it is never completed with unanonymised personal data.
    source: FR8.2, US7.4, NFR13

  - id: BR6.3
    statement: The per-subject crypto-shredding key protects Identity's personal fields and is destroyed on completion.
    category: policy
    applies_to: DeletionRequest, ShreddingKey
    trigger: A deletion request completes.
    logic: IF every personal category is anonymised THEN destroy the user's key in Supabase Vault as the last step; records other services retain under a legal basis are outside this key and stay readable.
    violation: A failure to destroy the key leaves the request failed and retryable.
    source: NFR13

  - id: BR6.4
    statement: A sign-in during deletion explains the state and collects no new data.
    category: policy
    applies_to: DeletionRequest
    trigger: A user signs in while a deletion request is requested or in_progress.
    logic: IF an active deletion exists THEN explain the state, collect no new data, and offer no portfolio access.
    violation: No new personal data is stored while the deletion runs.
    source: US7.4

  - id: BR6.5
    statement: Deletion is retryable and never fails silently.
    category: constraint
    applies_to: DeletionRequest
    trigger: A contributing service fails or is unreachable.
    logic: IF a step fails THEN record the failure reason and keep the request retryable; a retry resumes the remaining steps without repeating completed destruction.
    violation: The failure is recorded and surfaced; it is never swallowed.
    source: NFR13

  - id: BR6.6
    statement: Deletion completion is an audited privacy event.
    category: policy
    applies_to: DeletionRequest
    trigger: A deletion request completes or fails.
    logic: IF the status changes THEN write an audit event naming the owner, the request id, the retained categories, the time, and the outcome.
    violation: A failed audit write is logged; the request is never marked completed without its record.
    source: NFR13, NFR19

  - id: BR6.7
    statement: Deletion removes the Supabase Auth identity and its factors.
    category: policy
    applies_to: DeletionRequest, User
    trigger: Every contributing service has confirmed anonymisation.
    logic: IF anonymisation is complete THEN call the Supabase Auth admin user delete, removing email, password, MFA factors, and recovery codes, so the person can no longer sign in; existing audit events are never rewritten and the internal userId remains the pseudonym.
    violation: A failure leaves the request failed and retryable; the request is never completed with the Auth identity present.
    source: FR8.2, US7.4, NFR13

  - id: BR7.1
    statement: The jurisdiction gate runs before any personal data is collected.
    category: policy
    applies_to: JurisdictionRule
    trigger: A visitor starts sign-up.
    logic: IF the declared jurisdiction has an allowed rule THEN continue to the age check; ELSE deny access before any personal data is sent to Supabase Auth.
    violation: Access is denied with a plain explanation; no account is created.
    source: FR8.4, US7.6

  - id: BR7.2
    statement: A supported jurisdiction requires the launch disclosures to be acknowledged.
    category: policy
    applies_to: JurisdictionRule
    trigger: A supported visitor completes sign-up.
    logic: IF disclosuresRequired is true THEN show the disclosures and record their acknowledgement before the account becomes active.
    violation: Sign-up does not complete without the acknowledgement.
    source: US7.6

  - id: BR7.3
    statement: A declared age below the jurisdiction minimum is denied.
    category: validation
    applies_to: JurisdictionRule
    trigger: The visitor declares an age.
    logic: IF age is below minimumAge THEN deny access with a plain explanation before collecting personal data.
    violation: No account is created and no personal data is stored.
    source: US7.6

  - id: BR8.1
    statement: Session listing shows device, coarse location, and last activity from Identity's session view backed by Supabase Auth.
    category: policy
    applies_to: User, DeviceSession
    trigger: The user opens the session list.
    logic: IF the principal is active THEN list the DeviceSession rows for the user, updated from signed Supabase Auth session events, with device, coarse location, and last activity; when the Auth admin API exposes a session list, reconcile against it.
    violation: The list is refused with the error explained; no data is invented.
    source: FR1.4, US7.2

  - id: BR8.2
    statement: Revocation ends a session at its next request and warns when revoking the current one.
    category: policy
    applies_to: User
    trigger: The user revokes a session.
    logic: IF the session is not the current one THEN revoke it in Supabase Auth; IF it is the current one THEN warn first and revoke on confirmation.
    violation: A revoked session is signed out at its next request; the response is recorded.
    source: FR1.4, US7.2

  - id: BR8.3
    statement: Identity stores no tokens, factors, or recovery codes.
    category: constraint
    applies_to: User
    trigger: Any sign-in, refresh, or MFA flow.
    logic: IF a token or factor is produced THEN Supabase Auth keeps it; Identity keeps only the internal User and its own records.
    violation: No token or secret is persisted by Identity.
    source: FR1.3, NFR10
```

## Rules summary

| Rule | Category | Statement |
|---|---|---|
| BR1.1 | policy | Internal user created on first authenticated request |
| BR1.2 | authorization | Role only from `app_metadata.role`, least privilege |
| BR1.3 | authorization | `pending` blocks portfolio data; `deleted` blocks sign-in |
| BR1.4 | authorization | Ownership enforced on every user-owned read and write |
| BR1.5 | policy | Verified user becomes active on the first authenticated request |
| BR2.1 | policy | MFA required for users with exchange credentials, from the validated JWT claims |
| BR2.2 | constraint | Supabase Auth owns factors and recovery codes |
| BR2.3 | policy | Wrong MFA code refused and rate-limited |
| BR2.4 | policy | MFA enrolment offered to every user |
| BR3.1 | policy | Consent deny-by-default, never pre-ticked |
| BR3.2 | policy | Consent changes append-only, effective next event |
| BR3.3 | validation | Base currency, locale, timezone validated |
| BR4.1 | authorization | Support grant needs reason + distinct admin approver |
| BR4.2 | constraint | Support access expires (60 min default) |
| BR4.3 | policy | Every support read audited before the answer |
| BR4.4 | authorization | Support access read-only, metadata only |
| BR5.1 | policy | Export runs asynchronously with progress |
| BR5.2 | constraint | Export has no credentials or tokens |
| BR5.3 | authorization | Export owner-only, link expires |
| BR5.4 | policy | Export request and download are audited |
| BR6.1 | policy | Deletion is confirmed, tracked, single-active |
| BR6.2 | policy | Deletion anonymises and labels retention |
| BR6.3 | policy | Identity's crypto-shredding key destroyed on completion |
| BR6.4 | policy | Sign-in during deletion explained, no new data |
| BR6.5 | constraint | Deletion retryable, never silent |
| BR6.6 | policy | Deletion completion is audited |
| BR6.7 | policy | Deletion removes the Supabase Auth identity and factors |
| BR7.1 | policy | Jurisdiction gate before personal data |
| BR7.2 | policy | Disclosures acknowledged in supported jurisdictions |
| BR7.3 | validation | Below-minimum age denied |
| BR8.1 | policy | Session list from Identity's view backed by Supabase Auth |
| BR8.2 | policy | Revocation ends session at next request |
| BR8.3 | constraint | Identity stores no tokens or factors |
