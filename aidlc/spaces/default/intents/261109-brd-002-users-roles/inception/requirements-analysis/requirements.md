# Requirements — Users, Roles & Permissions (BRD-002)

**Stage:** inception/requirements-analysis · **Source anchor:** `docs/brds/BRD-002-users-roles-and-permissions.md` (v1.1), with the Identity service's implemented behaviour from the code knowledge base, the affirmed team practices, and the decisions recorded by the completed BRD-001 intent (`261105-brd-001-product`).

**ID note:** requirement numbering continues the product's record from the BRD-001 intent (FR1–FR13, NFR1–NFR20) so keys stay unique across intents. BRD-001's FR1 (identity, sessions and access) remains the broad statement; the requirements below elaborate the users/roles/permissions area.

## Intent Analysis

The business needs a trustworthy answer to two questions — *who may use this product, and what may each of them see or change?* BRD-002 defines the actors (Portfolio Owner, Admin, Support, System, Anonymous visitor, and a post-MVP Analyst/Viewer), a deny-by-default permission matrix, and the privacy rights that accompany a user record. This requirements set turns that BRD into testable statements for the current release, grounded in what the Identity service already implements and what the code knowledge base shows it does not.

The goals carried from BRD-002:

| ID | Goal |
|----|------|
| G-USR-1 | One internal user record per authenticated person, stable across provider changes |
| G-USR-2 | Deny-by-default access, with Admin additive over Owner and Support deliberately less than Owner |
| G-USR-3 | No user can see or change another user's data, and the refusal never reveals existence |
| G-USR-4 | Support access is least-privilege, time-bound, reason-recorded, and auditable |
| G-USR-5 | Users can see and control their own profile, preferences, and consent |
| G-USR-6 | Users can export and delete their own data, with legal holds respected |
| G-USR-7 | Role and status decisions are enforced server-side and audited, never silent |

## Scope Decisions From This Interview

These decisions refine the BRD baseline; where they narrow a documented Must, the narrowing is the decision of record for this release.

| Area | Decision | Effect on the BRD |
|------|----------|-------------------|
| User statuses | Implement all four: `active`, `disabled`, `pending_deletion`, `deleted` | Resolves the code/BRD mismatch; the Identity schema must grow the two missing statuses |
| Soft-delete recovery | 30 days from the deletion request, then permanent erasure | Resolves BRD-002 OQ-002 |
| Deletion cascade | BRD-017 (privacy and data rights) owns the per-entity cascade and retention policy; this set pins only that legal holds override | Leaves FR-USR-18's detail to BRD-017, which has not been worked yet |
| Admin claim | `app_metadata.role`, realm-wide, exact values `admin` and `support`; anything else is a plain user | Resolves OQ-001 |
| Support access window | At most 60 minutes, approved by a different admin, reason 1–500 characters, every access audited | Matches the implemented Identity grant rules |
| Support metadata boundary | Identifiers, labels, source type, sync status and timestamps, display name, email-verification status — never holdings, balances, values, transactions, orders or credentials | Makes FR-USR-11/19 testable |
| Jurisdiction gating | Maintained sanctions/regulatory list; self-declared country at sign-up plus IP geolocation | Makes FR-USR-16 testable; the list itself is compliance-owned |
| Age assurance | Self-declaration in every launch market; no documentary verification this release | Resolves OQ-004 for this release |
| Entitlements | Billing, plans, and entitlements are removed from this release (BRD-001 intent Q9); Admin changes roles and statuses, not entitlements | Narrows FR-USR-17 |
| Audit retention | Six months for audit records, including support-access records | Carried from BRD-001 intent Q5; resolves OQ-003/OQ-005 for this release |

## Functional Requirements

Every requirement states a testable acceptance criterion in Given/When/Then form.

### FR14 — Internal user records

- **FR14.1** The system shall create exactly one internal user record on the first successful authentication, keyed by the identity provider's subject.
  - *Acceptance:* Given a valid first request from a new provider subject, when it is processed, then exactly one internal user record maps to that subject; given a second request from the same subject, then no second record is created.
- **FR14.2** The internal user ID shall remain stable when the provider rotates claims such as email or display name.
  - *Acceptance:* Given a user whose provider email changes, when they next authenticate, then their internal user ID and every owned resource are unchanged.
- **FR14.3** Every user-owned resource — account, portfolio, transaction, snapshot, allocation, order, preference, export, and deletion request — shall be owned by exactly one internal user.
  - *Acceptance:* Given any user-owned row, when it is inspected, then it carries exactly one owner user ID; given a write that would leave it with none or several, then the write is rejected.
- **FR14.4** The user profile shall hold display name, email, locale, base currency, and timezone, derived from the identity provider where it supplies them and otherwise from the user's selection.
  - *Acceptance:* Given a provider token carrying an email and display name, when the record is created or refreshed, then those values populate the profile unless the user has overridden them.
- **FR14.5** Passwords, MFA factors, and provider tokens shall live only with the identity provider; the application shall store none of them.
  - *Acceptance:* Given the application's schemas, when they are searched, then no password, MFA secret, refresh token, or access token is stored.
- **FR14.6** Account recovery, including email change and lost-device recovery, shall go through the identity provider and shall not bypass role, status, or ownership checks.
  - *Acceptance:* Given a user who completes provider recovery, when they next call the API, then the same role, status, and ownership rules apply as for any other request.

### FR15 — Roles and authorisation

- **FR15.1** Authorisation shall be deny-by-default: a request that no rule explicitly permits is denied.
  - *Acceptance:* Given a route with no declared authorisation, when any caller requests it, then the request is denied and the denial is audited.
- **FR15.2** The caller's role shall be read only from `app_metadata.role`, with exact values `admin` and `support`; a missing or unrecognised value shall be treated as a plain Portfolio Owner.
  - *Acceptance:* Given a token whose `app_metadata.role` is `owner` or absent, when an admin endpoint is called, then the caller is treated as a plain user and denied.
- **FR15.3** Admin shall be additive over Portfolio Owner: an admin can do everything an owner can, on their own resources.
  - *Acceptance:* Given an admin with their own portfolio, when they read or change it, then the operation succeeds exactly as it would for a plain owner.
- **FR15.4** Only callers with the `admin` role shall reach administrative management — registries, sources, users, roles, jobs, flags, and support-grant approval; every other caller shall be denied.
  - *Acceptance:* Given a non-admin calling an admin endpoint, when it is processed, then it is denied with a forbidden response and no state change.
- **FR15.5** Every read and write of a user-owned resource, including list, search, and export endpoints, shall verify that the caller owns the resource.
  - *Acceptance:* Given user B calling a list endpoint, when user A's resources exist, then none of A's resources appear in B's result.
- **FR15.6** A request for another user's resource shall return not-found or forbidden without disclosing existence, and shall be indistinguishable from a request for a resource that does not exist.
  - *Acceptance:* Given user A's portfolio, when user B requests it by id, then B receives the same response a nonexistent id would produce.
- **FR15.7** A user in `disabled` or `deleted` status shall be denied even when presenting a valid provider token.
  - *Acceptance:* Given a disabled user with a valid token, when they call any authenticated endpoint, then the request is denied.
- **FR15.8** System jobs shall act for no user and shall never return or persist one user's data under another user's identity.
  - *Acceptance:* Given a scheduled job over all users, when it persists results, then each result is stored against the correct owner and no response mixes users.
- **FR15.9** Role and grant gating in the SPA and support console shall be presentational only; the server shall be authoritative for every decision.
  - *Acceptance:* Given a client that hides an admin control, when the caller invokes the underlying endpoint directly, then the server still denies it unless the role permits it.

### FR16 — Support access

- **FR16.1** The system shall provide a Support role with read-only, time-bound access to account metadata; every access shall require a reason and shall be audited.
  - *Acceptance:* Given a support agent opening a user record with a reason, when the access completes, then it appears in the audit log and access ends when the grant expires.
- **FR16.2** A support grant shall last at most 60 minutes and shall require approval by an admin who is not the requester; the reason shall be 1–500 characters.
  - *Acceptance:* Given a grant requested by one admin, when the same admin attempts to approve it, then approval is refused; given a reason of 0 or 501 characters, then the request is refused.
- **FR16.3** Under a grant, Support may read only: account and portfolio identifiers, labels, source type, sync status and timestamps, the user's display name, and email-verification status.
  - *Acceptance:* Given an active grant, when Support reads a user's record, then only those fields are returned and every portfolio-content field is absent.
- **FR16.4** Under a grant, Support shall never read holdings, balances, valuations, transactions, orders, credentials, or secrets, and no support screen or export shall contain a credential field.
  - *Acceptance:* Given every support screen and export, when its fields are inspected, then no credential, holding, balance, or valuation field is present.
- **FR16.5** An expired, revoked, or never-approved grant shall be refused; a grant revoked mid-session shall end access at the next request.
  - *Acceptance:* Given a grant whose expiry has passed, when Support calls the metadata endpoint, then it returns forbidden and the refusal is audited.
- **FR16.6** Support shall not be an Owner and shall not read portfolio contents even with a grant.
  - *Acceptance:* Given a support agent with any grant, when they request portfolio contents or valuations, then the request is denied.

### FR17 — Statuses, disabling, and deletion

- **FR17.1** The system shall track user status as `active`, `disabled`, `pending_deletion`, or `deleted`, and shall expose it on the principal.
  - *Acceptance:* Given a user in each status, when their principal is read, then the status is reported exactly and drives the denial rules in FR15.7.
- **FR17.2** An admin shall be able to disable a user without deleting historical data; a disabled user cannot re-enable themselves.
  - *Acceptance:* Given an active user, when an admin disables them, then the user is denied at the next request, their historical data remains, and only an admin can restore the account.
- **FR17.3** A user shall be able to request in-app account deletion; the account enters `pending_deletion` and remains restorable for 30 days, after which it becomes `deleted` and personal data is erased.
  - *Acceptance:* Given a deletion request, when the user signs in within 30 days, then the account can be restored to `active`; given the thirty-first day, then the account is `deleted` and its personal data is erased.
- **FR17.4** Deletion cascade and anonymisation per entity, and legal-hold overrides, shall follow BRD-017; until BRD-017 is approved, no new data shall be collected for a `pending_deletion` user and erasure shall not touch records under legal hold or statutory retention.
  - *Acceptance:* Given a `pending_deletion` user, when a new source is connected, then it is refused; given a record under legal hold, when erasure runs, then the record survives and the erasure is recorded.
- **FR17.5** A `deleted` user shall be denied with a valid token, and erasing the account twice shall leave the same end state.
  - *Acceptance:* Given a deleted user's valid token, when any authenticated endpoint is called, then it is denied; given erasure run twice, then the second run changes nothing and reports success.

### FR18 — Source-account ownership

- **FR18.1** A portfolio may link multiple source accounts, and a source account may contribute to one or more portfolios under BRD-006's membership model.
  - *Acceptance:* Given two source accounts linked to one portfolio, when the portfolio is valued, then both are included; given one account linked to two portfolios, then BRD-006's membership rules decide inclusion.
- **FR18.2** A source account may never link to another user's portfolio.
  - *Acceptance:* Given user A's source account, when a request links it to user B's portfolio, then the request is refused and no link is stored.
- **FR18.3** Deleting a source connection shall remove only the connection record and its portfolio links.
  - *Acceptance:* Given a source account linked to a portfolio, when the user deletes the connection, then the connection and its links are gone and the portfolio's other accounts are unaffected.

### FR19 — Preferences, consent, and entry gates

- **FR19.1** Users shall manage their locale, base currency, timezone, notification preferences, and consent from their account.
  - *Acceptance:* Given a user changing locale and base currency, when they save, then the profile reflects the change and stored financial values are unchanged.
- **FR19.2** Terms acceptance, risk disclosure, and age confirmation shall be required before the first source is connected.
  - *Acceptance:* Given a user who has not accepted the terms, when they attempt to connect a source, then the connection is refused and the missing acceptance is named.
- **FR19.3** Jurisdiction restrictions shall be enforced at sign-up and on continued use against a maintained sanctions/regulatory list, using the self-declared country plus IP geolocation.
  - *Acceptance:* Given a sign-up from a restricted jurisdiction by either signal, when it is submitted, then it is denied before any personal data is stored; given an existing user whose location becomes restricted, then continued use is blocked and audited.
- **FR19.4** A restricted sign-up shall be denied before any personal data is stored.
  - *Acceptance:* Given a restricted sign-up, when it is submitted, then no user record, profile row, or shredding key is created.
- **FR19.5** Age assurance shall be self-declaration in every launch market for this release; no documentary verification is required.
  - *Acceptance:* Given a user confirming they meet the age requirement, when they proceed, then the confirmation is recorded and no document upload is requested.

### FR20 — Administration and audit

- **FR20.1** Admins shall be able to look up users, change roles, disable accounts, and view support grants; entitlement overrides are not part of this release.
  - *Acceptance:* Given an admin searching for a user, when the search completes, then they can open the record, change the role, or disable the account; no entitlement control is presented or accepted.
- **FR20.2** Every role and status change shall be audited with actor, target, time, reason, and outcome, and shall never be silent.
  - *Acceptance:* Given an admin changing a role, when the change commits, then an audit record names the actor, the target, the previous and new role, and the reason.
- **FR20.3** Security-sensitive actions shall be audited: sign-in, account changes, credential changes, order decisions, admin changes, and support access.
  - *Acceptance:* Given each listed action, when it occurs, then an audit record carries actor (or system), action, target, timestamp, context, and outcome.
- **FR20.4** Audit records shall never contain credentials, tokens, full addresses, or secrets.
  - *Acceptance:* Given an audit record for any listed action, when it is inspected, then no credential, token, full wallet address, or secret appears.

## Non-Functional Requirements

| ID | Requirement | Target | Source |
|----|-------------|--------|--------|
| NFR21 | Audit retention | Six months for all audit records, including support-access records; older records are removed automatically | BRD-001 intent Q5; BRD-002 OQ-003/OQ-005 |
| NFR22 | Sensitive identifier handling | The provider subject is stored as a hash for lookup plus a deterministic ciphertext; it never appears in logs, responses, or audit records | codekb business-overview; BRD-012 |
| NFR23 | Least privilege at the data layer | Each service's Postgres role is restricted to its own schema; the audit role is insert-only; no cross-schema grants | project memory; BRD-012 |
| NFR24 | Role and status freshness | Role and status are recomputed from the signed token on every request; a change takes effect at the user's next request | codekb interaction diagram 1; BRD-002 BR-USR-11 |
| NFR25 | Accessibility of role and support screens | WCAG 2.2 AA on the account, admin, and support-console flows: keyboard, screen reader, contrast, motion | BRD-012 NFR-A11Y; BRD-002 v1.1 |
| NFR26 | Localization of user-facing access screens | English and Vietnamese; dates, times, and status labels follow the user's locale | BRD-001 intent Q1/Q8; BRD-018 |

## Constraints

- Role comes only from `app_metadata.role`; no other claim, table, or header may grant `admin` or `support`.
- Passwords, MFA factors, and provider tokens stay in Supabase Auth; the application never stores them.
- Every service owns one Supabase Postgres schema and one Postgres role; another service's data is read only through its HTTP API.
- No message queues; synchronous internal HTTP with bounded retries and idempotency keys.
- The audit store is append-only through an insert-only role.
- Production is the only environment; changes ship as versioned sqlx up/down migration pairs.
- Billing, plans, entitlements, and quotas do not exist in this release (BRD-001 intent Q9), so no entitlement state may gate access.

## Assumptions

- **Sign-in methods:** email/password and supported social providers through Supabase Auth, as recorded in the BRD-001 intent. A provider subject identifies one internal user; the same email arriving through two providers is not merged automatically.
- **Sanctions/regulatory list:** maintained through admin configuration (BRD-010); its initial content is a compliance input, not a product decision.
- **Recovery window start:** the 30 days run from the moment the deletion request is confirmed.
- **Restoration:** a `pending_deletion` user can cancel deletion and return to `active` at any time inside the 30 days.
- **Support grant scope:** a grant is per user record, not per account or per ticket.
- **Email-verification status:** read from the provider claim; the application does not maintain a second verification state.
- **BRD-017 dependency:** the per-entity cascade, anonymisation rules, and legal-hold register are defined by BRD-017; this release implements the account-level state machine and the refusal paths until BRD-017 is approved.

## Out of Scope

- Analyst/Viewer shared-portfolio access (BRD-002 FR-USR-10, marked Won't for the MVP).
- Email invitations, Advisor roles, and shared portfolios.
- Entitlement overrides and any billing or plan management (FR-USR-17 narrowed by the BRD-001 intent decision).
- Documentary age verification in any launch market (Q8).
- Per-entity deletion cascade and anonymisation detail (BRD-017).
- Support tooling specifics such as ticket workflows and the status page (BRD-019).

## Open Questions

- **OQ-A** Which jurisdictions are on the launch sanctions/regulatory list, and who maintains and reviews it? (Compliance; owned with BRD-010/BRD-012.)
- **OQ-B** During `pending_deletion`, may Support read account metadata, or does the grant path refuse a pending-deletion user? (Security; to settle with BRD-019.)
- **OQ-C** What is shown to a user during the 30-day window — a restore prompt, a countdown, or both? (Product/design; to settle with BRD-017 and the mockups.)
- **OQ-D** Should an admin's role change on a user take effect on their existing sessions immediately, or only at the next request? (Security; the code refreshes on each request, so this is a confirmation, not a design choice.)
