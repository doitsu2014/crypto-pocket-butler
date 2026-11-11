# User Stories — Users, Roles & Permissions

Stories are grouped by requirement group (FR14–FR20), one story per functional sub-requirement. Every story is Must Have unless marked; the only Won't Have is shared portfolios, recorded at the end. Each story carries Given/When/Then acceptance criteria with stable `ACx.y.z` IDs.

## Group US1 — Internal user records (FR14)

### US1.1 — One record per person on first sign-in
**Priority:** Must · **Requirement:** FR14.1 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** the platform to create exactly one user record the first time I authenticate, **so that** my data always attaches to me and never to a duplicate.
**Acceptance criteria:**
- **AC1.1.1** Given a new provider subject, when it authenticates for the first time, then exactly one internal user record is created and mapped to that subject.
- **AC1.1.2** Given a subject that already has a record, when it authenticates again, then no second record is created and the existing one is used.
**INVEST:** Independent of other stories; testable through the Identity service.

### US1.2 — My identity survives provider changes
**Priority:** Must · **Requirement:** FR14.2 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** my internal identity to stay the same when my provider email or display name changes, **so that** my history is never split or lost.
**Acceptance criteria:**
- **AC1.2.1** Given a user whose provider email changes, when they next authenticate, then their internal user ID and all owned resources are unchanged.
- **AC1.2.2** Given a provider claim rotation that changes the display name, when the profile is refreshed, then no new user record is created.
**INVEST:** Testable with a changed token; no dependency on other stories.

### US1.3 — Every resource has exactly one owner
**Priority:** Must · **Requirement:** FR14.3 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** every account, portfolio, transaction, snapshot, order, preference, export, and deletion request to belong to exactly one user, **so that** nothing is shared or orphaned by accident.
**Acceptance criteria:**
- **AC1.3.1** Given any user-owned row, when it is inspected, then it carries exactly one owner user ID.
- **AC1.3.2** Given a write that would leave a user-owned row with no owner or with two, when it is attempted, then the write is rejected.
**INVEST:** Testable at the repository layer; independent.

### US1.4 — My profile holds the right fields
**Priority:** Must · **Requirement:** FR14.4 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** my display name, email, locale, base currency, and timezone in one profile, **so that** the product speaks to me the way I chose.
**Acceptance criteria:**
- **AC1.4.1** Given a provider token containing an email and display name, when the record is created or refreshed, then those values populate the profile unless I have overridden them.
- **AC1.4.2** Given I have set a locale and base currency, when my profile is refreshed, then my chosen values are not overwritten by provider defaults.
**INVEST:** Testable; independent.

### US1.5 — Passwords and MFA live with the provider
**Priority:** Must · **Requirement:** FR14.5 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** my password, MFA factors, and tokens to live only with the identity provider, **so that** the application can never leak them.
**Acceptance criteria:**
- **AC1.5.1** Given the application's schemas, when they are searched, then no password, MFA secret, refresh token, or access token is stored.
- **AC1.5.2** Given the application's logs and responses, when inspected, then none carries a provider token or MFA secret.
**INVEST:** Verifiable by schema and log inspection; independent.

### US1.6 — Recovery goes through the provider
**Priority:** Must · **Requirement:** FR14.6 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** email changes and lost-device recovery to go through the identity provider, **so that** recovering access never creates a way around the role and status rules.
**Acceptance criteria:**
- **AC1.6.1** Given I complete provider recovery, when I next call the API, then the same role, status, and ownership rules apply as for any other request.
- **AC1.6.2** Given a disabled account completing provider recovery, when it calls an authenticated endpoint, then it is still denied.
**INVEST:** Testable; independent.

## Group US2 — Roles and authorisation (FR15)

### US2.1 — Nothing is allowed unless it is permitted
**Priority:** Must · **Requirement:** FR15.1 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** authorisation to deny by default, **so that** a new endpoint cannot accidentally expose my data.
**Acceptance criteria:**
- **AC2.1.1** Given a route with no declared authorisation, when any caller requests it, then the request is denied.
- **AC2.1.2** Given that denial, then an audit record names the actor, action, target, and outcome.
**INVEST:** Testable at the gateway and service boundary.

### US2.2 — My role comes from one claim
**Priority:** Must · **Requirement:** FR15.2 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** my role to be read only from `app_metadata.role` with exact values `admin` and `support`, **so that** no other field, header, or table can silently grant privilege.
**Acceptance criteria:**
- **AC2.2.1** Given a token whose `app_metadata.role` is absent or unrecognised, when an admin endpoint is called, then the caller is treated as a plain user and denied.
- **AC2.2.2** Given a token carrying any other role-shaped claim, when authorisation runs, then that claim is ignored.
**INVEST:** Testable with crafted tokens; independent.

### US2.3 — Admin can still do what an owner can
**Priority:** Must · **Requirement:** FR15.3 · **Persona:** Admin
**As an** Admin, **I want** my admin role to be additive over owner, **so that** I can manage my own portfolio exactly like anyone else.
**Acceptance criteria:**
- **AC2.3.1** Given an admin with their own portfolio, when they read or change it, then the operation succeeds as it would for a plain owner.
- **AC2.3.2** Given an admin's own list endpoints, when they run, then the results are the admin's resources and no other user's.
**INVEST:** Testable; independent.

### US2.4 — Only admins reach administration
**Priority:** Must · **Requirement:** FR15.4 · **Persona:** Admin
**As an** Admin, **I want** administrative management restricted to admin-role callers, **so that** registries, users, roles, jobs, flags, and support approvals cannot be changed by anyone else.
**Acceptance criteria:**
- **AC2.4.1** Given a non-admin calling an admin endpoint, when it is processed, then it is denied with a forbidden response and no state change.
- **AC2.4.2** Given a support-role caller calling an admin endpoint, when it is processed, then it is denied.
**INVEST:** Testable; independent.

### US2.5 — Every read and write checks ownership
**Priority:** Must · **Requirement:** FR15.5 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** every read and write — including lists, searches, and exports — to verify I own the resource, **so that** no endpoint becomes a back door.
**Acceptance criteria:**
- **AC2.5.1** Given user B calling a list endpoint, when user A's resources exist, then none of A's resources appear in B's result.
- **AC2.5.2** Given user B requesting an export of user A's data, when it is processed, then the export is refused and no file is produced.
**INVEST:** Testable per endpoint; independent.

### US2.6 — A refusal reveals nothing
**Priority:** Must · **Requirement:** FR15.6 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** another user's request for my resource to be refused without revealing that it exists, **so that** the platform does not confirm who has what.
**Acceptance criteria:**
- **AC2.6.1** Given user A's portfolio, when user B requests it by id, then B receives the same response a nonexistent id would produce.
- **AC2.6.2** Given the refusal, then no field of A's resource appears in the response.
**INVEST:** Testable by comparing responses; independent.

### US2.7 — Disabled and deleted accounts are shut out
**Priority:** Must · **Requirement:** FR15.7 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** a disabled or deleted account to be denied even with a valid token, **so that** disabling someone actually stops them.
**Acceptance criteria:**
- **AC2.7.1** Given a disabled user with a valid token, when they call any authenticated endpoint, then the request is denied.
- **AC2.7.2** Given a deleted user with a valid token, when they call any authenticated endpoint, then the request is denied.
**INVEST:** Testable; independent.

### US2.8 — Jobs never mix users' data
**Priority:** Must · **Requirement:** FR15.8 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** scheduled jobs to act for no user and never return one person's data under another's identity, **so that** background processing cannot leak between accounts.
**Acceptance criteria:**
- **AC2.8.1** Given a scheduled job over all users, when it persists results, then each result is stored against the correct owner.
- **AC2.8.2** Given any job response, then it contains no other user's data.
**INVEST:** Testable with job run records; independent.

### US2.9 — The server is the authority, not the screen
**Priority:** Must · **Requirement:** FR15.9 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** client-side role and grant gating to be presentational only, **so that** hiding a control is never what protects my data.
**Acceptance criteria:**
- **AC2.9.1** Given a client that hides an admin control, when the caller invokes the underlying endpoint directly, then the server still denies it unless the role permits it.
- **AC2.9.2** Given a tampered client that shows a hidden control, when it is used, then the server refuses the operation.
**INVEST:** Testable with a direct HTTP call; independent.

## Group US3 — Support access (FR16)

### US3.1 — Support gets metadata only, with a reason, on the record
**Priority:** Must · **Requirement:** FR16.1 · **Persona:** Support Agent
**As a** Support Agent, **I want** read-only, time-bound access to a user's account metadata when I have a reason, **so that** I can help without ever holding more access than the case needs.
**Acceptance criteria:**
- **AC3.1.1** Given a support agent opening a user record with a reason, when the access completes, then it appears in the audit log.
- **AC3.1.2** Given the access window ends, when the agent makes another request, then access is refused.
**INVEST:** Testable through the grant journey; independent of admin stories.

### US3.2 — A second admin approves, and the window is short
**Priority:** Must · **Requirement:** FR16.2 · **Persona:** Admin
**As an** Admin, **I want** support grants to last at most 60 minutes and require approval by a different admin, **so that** one person can neither grant themselves access nor keep it open.
**Acceptance criteria:**
- **AC3.2.1** Given a support agent requests a grant for one user record with a reason, when a different admin approves it, then the grant becomes active with an expiry no later than 60 minutes from approval.
- **AC3.2.2** Given the requesting admin attempts to approve their own grant, then approval is refused.
- **AC3.2.3** Given a reason shorter than 1 or longer than 500 characters, when the grant is requested, then it is refused.
**INVEST:** Testable end to end; independent.

### US3.3 — I can see only the agreed metadata fields
**Priority:** Must · **Requirement:** FR16.3 · **Persona:** Support Agent
**As a** Support Agent, **I want** a grant to expose only the agreed metadata fields, **so that** my screen cannot become a way to read someone's finances.
**Acceptance criteria:**
- **AC3.3.1** Given an active grant, when I read a user's record, then only account and portfolio identifiers, labels, source type, sync status and timestamps, display name, and email-verification status are returned.
- **AC3.3.2** Given the same response, then holdings, balances, valuations, transactions, and orders are absent.
**INVEST:** Testable against a response schema; independent.

### US3.4 — Support never sees or resets credentials
**Priority:** Must · **Requirement:** FR16.4 · **Persona:** Support Agent
**As a** Support Agent, **I want** no support screen, export, or action to expose or reset a credential, **so that** a support session can never compromise an account.
**Acceptance criteria:**
- **AC3.4.1** Given every support screen and export, when its fields are inspected, then no credential, holding, balance, or valuation field is present.
- **AC3.4.2** Given a support agent attempts to view or reset a credential, then the action is unavailable and the API refuses it.
**INVEST:** Testable by screen inspection and a direct API call.

### US3.5 — Expired or revoked access stops immediately
**Priority:** Must · **Requirement:** FR16.5 · **Persona:** Support Agent
**As a** Support Agent, **I want** expired, revoked, or never-approved grants to be refused, **so that** access cannot outlive the reason it was given for.
**Acceptance criteria:**
- **AC3.5.1** Given a grant whose expiry has passed, when I call the metadata endpoint, then it returns forbidden and the refusal is audited.
- **AC3.5.2** Given a grant revoked mid-session, when I make the next request, then access is refused.
**INVEST:** Testable; independent.

### US3.6 — Support is never an owner
**Priority:** Must · **Requirement:** FR16.6 · **Persona:** Support Agent
**As a** Support Agent, **I want** to be denied portfolio contents even with a grant, **so that** the least-privilege promise holds at every layer.
**Acceptance criteria:**
- **AC3.6.1** Given a support agent with any grant, when they request portfolio contents or valuations, then the request is denied.
- **AC3.6.2** Given a support agent with no grant, when they request any user record, then it is denied.
**INVEST:** Testable; independent.

## Group US4 — Statuses, disabling, and deletion (FR17)

### US4.1 — My account has a clear status
**Priority:** Must · **Requirement:** FR17.1 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** my status to be one of active, disabled, pending deletion, or deleted, **so that** the product and I agree on whether my account can be used.
**Acceptance criteria:**
- **AC4.1.1** Given a user in each status, when their principal is read, then the status is reported exactly and drives the denial rules.
- **AC4.1.2** Given a status value outside the four, when it is written, then the write is rejected.
**INVEST:** Testable at the domain and API layers; independent.

### US4.2 — Admins can disable without erasing history
**Priority:** Must · **Requirement:** FR17.2 · **Persona:** Admin
**As an** Admin, **I want** to disable a user without deleting their historical data, **so that** an access problem can be fixed without destroying records.
**Acceptance criteria:**
- **AC4.2.1** Given an active user, when an admin disables them, then the user is denied at the next request and their historical data remains.
- **AC4.2.2** Given a disabled user, when they attempt to re-enable themselves, then the attempt is refused; only an admin can restore the account.
**INVEST:** Testable; independent.

### US4.3 — I can delete my account and change my mind
**Priority:** Must · **Requirement:** FR17.3 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** to request in-app deletion and recover the account within 30 days, **so that** leaving is possible without being irreversible by accident.
**Acceptance criteria:**
- **AC4.3.1** Given a deletion request, when I sign in within 30 days, then the account can be restored to active.
- **AC4.3.2** Given the thirty-first day, then the account becomes deleted and its personal data is erased.
- **AC4.3.3** Given a pending-deletion account, when it calls an authenticated endpoint other than restore, then it is refused with the restore path named.
**INVEST:** Testable with a controlled clock; independent.

### US4.4 — Deletion follows BRD-017 and respects legal holds
**Priority:** Must · **Requirement:** FR17.4 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** deletion to follow BRD-017's cascade and preserve records under legal hold, **so that** my rights and the platform's legal duties are both met.
**Acceptance criteria:**
- **AC4.4.1** Given a pending-deletion user, when a new source is connected, then it is refused.
- **AC4.4.2** Given a record under legal hold, when erasure runs, then the record survives and the erasure is recorded.
**INVEST:** Testable with a legal-hold fixture; depends on BRD-017 only for the policy detail.

### US4.5 — Deleted means denied, and erasure is idempotent
**Priority:** Must · **Requirement:** FR17.5 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** a deleted account to be denied and erasure to be safe to repeat, **so that** a retried job cannot resurrect or corrupt anything.
**Acceptance criteria:**
- **AC4.5.1** Given a deleted user's valid token, when any authenticated endpoint is called, then it is denied.
- **AC4.5.2** Given erasure run twice, then the second run changes nothing and reports success.
**INVEST:** Testable; independent.

## Group US5 — Source-account ownership (FR18)

### US5.1 — Accounts and portfolios link many-to-many within my data
**Priority:** Must · **Requirement:** FR18.1 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** a portfolio to link several source accounts and an account to contribute to several portfolios, **so that** I can model my holdings the way they really are.
**Acceptance criteria:**
- **AC5.1.1** Given two source accounts linked to one portfolio, when the portfolio is valued, then both are included.
- **AC5.1.2** Given one account linked to two portfolios, then BRD-006's membership rules decide inclusion and no value is counted twice.
**INVEST:** Testable; independent.

### US5.2 — My account can never link into someone else's portfolio
**Priority:** Must · **Requirement:** FR18.2 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** my source account to be unlinkable to another user's portfolio, **so that** cross-user data mixing is impossible by construction.
**Acceptance criteria:**
- **AC5.2.1** Given user A's source account, when a request links it to user B's portfolio, then the request is refused and no link is stored.
- **AC5.2.2** Given the refusal, then the attempt is audited with actor, target, and outcome.
**INVEST:** Testable; independent.

### US5.3 — Removing a connection removes only the connection
**Priority:** Must · **Requirement:** FR18.3 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** deleting a source connection to remove only that connection and its portfolio links, **so that** nothing else in my data is affected.
**Acceptance criteria:**
- **AC5.3.1** Given a source account linked to a portfolio, when I delete the connection, then the connection and its links are gone.
- **AC5.3.2** Given other accounts in the same portfolio, then they and the portfolio remain unchanged.
**INVEST:** Testable; independent.

## Group US6 — Preferences, consent, and entry gates (FR19)

### US6.1 — I control my preferences and consent
**Priority:** Must · **Requirement:** FR19.1 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** to manage my locale, base currency, timezone, notification preferences, and consent, **so that** the product behaves the way I chose and I know what I agreed to.
**Acceptance criteria:**
- **AC6.1.1** Given I change locale and base currency, when I save, then my profile reflects the change and stored financial values are unchanged.
- **AC6.1.2** Given I withdraw a consent, when it is saved, then the withdrawal is recorded with a timestamp and later stages see it.
**INVEST:** Testable; independent.

### US6.2 — Entry requires terms, risk disclosure, and age
**Priority:** Must · **Requirement:** FR19.2 · **Persona:** Visitor
**As a** Visitor, **I want** to see and accept the terms, risk disclosure, and age confirmation before connecting a source, **so that** I know what I am agreeing to before any data is touched.
**Acceptance criteria:**
- **AC6.2.1** Given a user who has not accepted the terms, when they attempt to connect a source, then the connection is refused and the missing acceptance is named.
- **AC6.2.2** Given the age confirmation is not recorded, when the user proceeds, then they cannot reach the first-source step.
**INVEST:** Testable; independent.

### US6.3 — Availability is checked where I live
**Priority:** Must · **Requirement:** FR19.3 · **Persona:** Visitor
**As a** Visitor, **I want** the platform to enforce jurisdiction restrictions at sign-up and on continued use, **so that** I am not onboarded into a service that is not available to me.
**Acceptance criteria:**
- **AC6.3.1** Given a sign-up from a restricted jurisdiction by the self-declared country or by IP geolocation, when it is submitted, then it is denied before any personal data is stored.
- **AC6.3.2** Given an existing user whose location becomes restricted, when they next use the product, then continued use is blocked and audited.
**INVEST:** Testable with a maintained list fixture; independent.

### US6.4 — A restricted sign-up stores nothing
**Priority:** Must · **Requirement:** FR19.4 · **Persona:** Visitor
**As a** Visitor, **I want** a restricted sign-up to be refused before any personal data is stored, **so that** a refused attempt leaves no trace of me.
**Acceptance criteria:**
- **AC6.4.1** Given a restricted sign-up, when it is submitted, then no user record, profile row, or shredding key is created.
- **AC6.4.2** Given the same attempt, then the refusal is recorded without personal data.
**INVEST:** Testable; independent.

### US6.5 — Age is self-declared in every launch market
**Priority:** Must · **Requirement:** FR19.5 · **Persona:** Visitor
**As a** Visitor, **I want** age assurance to be a self-declaration in this release, **so that** I can proceed without uploading identity documents.
**Acceptance criteria:**
- **AC6.5.1** Given a user confirming they meet the age requirement, when they proceed, then the confirmation is recorded and no document upload is requested.
- **AC6.5.2** Given the confirmation is missing, then the first-source step is unavailable.
**INVEST:** Testable; independent.

## Group US7 — Administration and audit (FR20)

### US7.1 — Admins manage users and roles, not entitlements
**Priority:** Must · **Requirement:** FR20.1 · **Persona:** Admin
**As an** Admin, **I want** to look up users, change roles, disable accounts, and view support grants, **so that** I can run the platform without any billing or entitlement controls that this release does not have.
**Acceptance criteria:**
- **AC7.1.1** Given an admin searching for a user, when the search completes, then they can open the record, change the role, or disable the account.
- **AC7.1.2** Given the admin screen or the API, then no entitlement control is presented or accepted.
**INVEST:** Testable; independent.

### US7.2 — Role and status changes are never silent
**Priority:** Must · **Requirement:** FR20.2 · **Persona:** Admin
**As an** Admin, **I want** every role and status change audited with actor, target, time, reason, and outcome, **so that** any access change can be explained later.
**Acceptance criteria:**
- **AC7.2.1** Given an admin changing a role, when the change commits, then an audit record names the actor, the target, the previous and new role, and the reason.
- **AC7.2.2** Given a change with no reason, when it is submitted, then it is refused.
**INVEST:** Testable; independent.

### US7.3 — Security-sensitive actions are audited
**Priority:** Must · **Requirement:** FR20.3 · **Persona:** Admin
**As an** Admin, **I want** sign-in, account changes, credential changes, order decisions, admin changes, and support access to be audited, **so that** the whole security surface has a record.
**Acceptance criteria:**
- **AC7.3.1** Given each listed action, when it occurs, then an audit record carries actor (or system), action, target, timestamp, context, and outcome.
- **AC7.3.2** Given an audit write failure, when the action is security-critical, then the action fails or is surfaced rather than silently completing without a record.
**INVEST:** Testable per event type; independent.

### US7.4 — Audit records never carry secrets
**Priority:** Must · **Requirement:** FR20.4 · **Persona:** Admin
**As an** Admin, **I want** audit records to never contain credentials, tokens, full addresses, or secrets, **so that** the audit trail cannot become a leak.
**Acceptance criteria:**
- **AC7.4.1** Given an audit record for any listed action, when it is inspected, then no credential, token, full wallet address, or secret appears.
- **AC7.4.2** Given a caller attempts to write such a value into an audit record, then the write is rejected or redacted.
**INVEST:** Testable with a redaction fixture; independent.

## Won't Have

### US8.1 — Share a portfolio with an Analyst or Viewer
**Priority:** Won't (MVP) · **Requirement:** FR-USR-10 · **Persona:** Portfolio Owner
**As a** Portfolio Owner, **I want** to share a portfolio read-only with another person, **so that** they can view it without my credentials.
**Acceptance criteria:**
- **AC8.1.1** Given the MVP release, when a user looks for sharing, then no sharing control exists.
**INVEST:** Recorded so the post-MVP model stays designable without migration blockers.
