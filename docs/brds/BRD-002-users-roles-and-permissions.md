# BRD-002 — Users, Roles & Permissions

Draft v1.1 · Product · 2026-11-04. Who uses the product, and how one user's data is protected from another's. v1.1 adds the Support role, user preferences, self-service rights, and global jurisdiction handling.

## Actors and roles

| Actor | MVP | Can do |
|-------|-----|--------|
| **Portfolio Owner** | Yes | Manage own accounts, portfolios, transactions, snapshots, orders, preferences |
| **Admin** | Yes | Owner capabilities **plus** manage chains, tokens, jobs, support tools, and feature flags. Role comes from Supabase `app_metadata.role`, projected into the JWT |
| **Support** | Yes | Read-only, time-bound, reason-recorded access to account metadata and tickets; never secrets or portfolio edits |
| **System** | Yes | Run scheduled jobs over all users' data; no interactive access |
| **Analyst / Viewer** | Post-MVP | Read-only access to shared portfolios |
| **Anonymous visitor** | Yes | Sign-in and public pages only |

Authorisation is deny-by-default; an Admin is also an Owner. Support is **not** an Owner and cannot read credentials at any time.

## Permission matrix (MVP)

| Capability | Owner | Admin | Support | System |
|------------|:-----:|:-----:|:-------:|:------:|
| Manage own accounts / portfolios / links / targets | Yes | Yes | No | No |
| Sync own accounts | Yes | Yes | No | Scheduled |
| View own allocation, snapshots, orders, P&L | Yes | Yes | No (metadata only) | Yes (writes) |
| Create manual / EOD snapshots | Yes | Yes | No | EOD |
| Create / cancel own orders; record trades | Yes | Yes | No | No |
| Manage chains and token registry | No | Yes | No | No |
| Trigger background jobs manually | No | Yes | No | Scheduled |
| View account metadata for support | No | Yes | Yes, time-bound | No |
| View credentials or secrets | No | No | **No** | No (jobs use Vault) |
| Manage users, roles, entitlements | No | Yes | No | No |
| Access another user's portfolio data | No | No | No | Jobs process internally, never expose |
| Self-service export / delete own data | Yes | Yes | No | Scheduled |

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-USR-01 | Create an internal user record on first successful authentication, keyed by the identity provider's subject. | Must |
| FR-USR-02 | Every account, portfolio, transaction, snapshot, allocation, and order is owned by exactly one internal user. | Must |
| FR-USR-03 | All reads and writes on user-owned resources verify ownership. | Must |
| FR-USR-04 | Requests for another user's resource return not-found/forbidden without disclosing existence. | Must |
| FR-USR-05 | Only `admin`-role users reach administrative management; others are denied. | Must |
| FR-USR-06 | Audit security-sensitive actions: sign-in, account changes, credential changes, order decisions, admin changes, support access. | Must |
| FR-USR-07 | User profile with display name, email, locale, base currency, and timezone, derived from the identity provider where possible. | Must |
| FR-USR-08 | Prevent user deletion while portfolios exist, or apply the communicated cascade/anonymisation policy in BRD-017. | Must |
| FR-USR-09 | Support disabling a user without deleting historical data. | Must |
| FR-USR-10 | Support sharing portfolios with Analyst/Viewer users. | Won't (MVP) |
| FR-USR-11 | Provide a Support role with read-only, time-bound access to account metadata; every access requires a reason and is audited. | Must |
| FR-USR-12 | Let users manage locale, base currency, timezone, notification preferences, and consent. | Must |
| FR-USR-13 | Provide account recovery through the identity provider, including email change and lost-device paths. | Must |
| FR-USR-14 | Provide self-service data export and in-app account deletion. | Must |
| FR-USR-15 | Require terms, risk disclosure, and age confirmation before the first source. | Must |
| FR-USR-16 | Detect and enforce jurisdiction restrictions at sign-up and on continued use. | Must |
| FR-USR-17 | Let Admins look up users, change roles, disable accounts, and override entitlements with audit. | Must |
| FR-USR-18 | Define and execute deletion cascade/anonymisation per entity, with legal-hold overrides. | Must |
| FR-USR-19 | Support cannot view, export, or reset credentials; the UI never renders them. | Must |
| FR-USR-20 | Track user status: `active`, `disabled`, `pending_deletion`, `deleted`. | Must |
| FR-USR-21 | Audit every role and entitlement change with actor, target, time, and reason. | Must |

## Business rules

| ID | Rule |
|----|------|
| BR-USR-01 | Passwords and MFA live with the identity provider, never in the application. |
| BR-USR-02 | The internal user ID is stable even if the provider rotates claims. |
| BR-USR-03 | Deny-by-default authorisation. |
| BR-USR-04 | Admin is additive: an Admin is also a Portfolio Owner. |
| BR-USR-05 | System jobs act for no user and never expose one user's data in another's response. |
| BR-USR-06 | Deleting an account removes only the connection record and its portfolio links. |
| BR-USR-07 | A portfolio may link multiple accounts; an account may contribute to one or more portfolios under BRD-006's membership model. |
| BR-USR-08 | An account may never link to another user's portfolio. |
| BR-USR-09 | Support access is least-privilege, time-bound, reason-recorded, and audited. |
| BR-USR-10 | Support never asks for or sees a password, private key, seed phrase, or full API secret. |
| BR-USR-11 | A disabled user is denied even with a valid provider token. |
| BR-USR-12 | Jurisdiction and age restrictions are enforced before data collection. |
| BR-USR-13 | Deletion preserves only records under legal hold or statutory retention. |
| BR-USR-14 | Role and entitlement changes are never silent. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-USR-01 | A first-time user's first API request leaves exactly one internal user record mapped to their subject. |
| AC-USR-02 | User B requesting user A's portfolio gets no hint that it exists. |
| AC-USR-03 | A non-admin calling an admin endpoint is denied. |
| AC-USR-04 | An Admin's chain/token change is recorded and affects subsequent syncs. |
| AC-USR-05 | Two accounts linked to one portfolio: allowed. An account linked to multiple portfolios follows BRD-006's membership rules. |
| AC-USR-06 | A scheduled job persists each portfolio's results against the correct owner. |
| AC-USR-07 | Support opens a user record with a reason; access appears in the audit log and expires automatically. |
| AC-USR-08 | No support screen or export contains a credential field. |
| AC-USR-09 | A user deletes their account in-app; the cascade follows BRD-017 and legal holds are preserved. |
| AC-USR-10 | A user changes locale and base currency; stored values are unchanged. |
| AC-USR-11 | A jurisdiction-restricted sign-up is denied before any personal data is stored. |
| AC-USR-12 | A role change is visible in the audit log with actor and reason. |

## Notes

- No email invitations, shared portfolios, or Advisor/Viewer roles in the MVP; the post-MVP model must be designable without migration blockers.
- Audit retention is defined in BRD-012 and BRD-017.
- Support tooling is specified in BRD-019.

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Exact claim/role name for Admin; realm-wide or client-scoped? | Security |
| OQ-002 | Soft-delete recovery window for accounts and portfolios? | Product |
| OQ-003 | Audit-event retention period? | Compliance |
| OQ-004 | Which launch markets require age verification beyond self-declaration? | Compliance |
| OQ-005 | Retention period for support-access audit records? | Compliance |
