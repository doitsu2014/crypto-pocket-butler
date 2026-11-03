# BRD-002 — Users, Roles & Permissions

Draft v1.0 · Product · 2026-11-01. Who uses the product, and how one user's data is protected from another.

## Actors and roles

| Actor | MVP | Can do |
|-------|-----|--------|
| **Portfolio Owner** | Yes | Manage own accounts, portfolios, snapshots, orders, settings |
| **Admin** | Yes | Owner capabilities **plus** manage chains, tokens, and jobs. Role comes from Supabase `app_metadata.role`, projected into the JWT |
| **System** | Yes | Run scheduled jobs over all users' data; no interactive access |
| **Analyst / Viewer** | Post-MVP | Read-only access to shared portfolios |
| **Anonymous visitor** | Yes | Sign-in only |

Authorisation is deny-by-default; an Admin is also an Owner.

## Permission matrix (MVP)

| Capability | Owner | Admin | System |
|------------|:-----:|:-----:|:------:|
| Manage own accounts / portfolios / links / targets | Yes | Yes | No |
| Sync own accounts | Yes | Yes | Scheduled |
| View own allocation, snapshots, orders | Yes | Yes | Yes (writes) |
| Create manual / EOD snapshots | Yes | Yes | EOD |
| Create / cancel own orders; record trades | Yes | Yes | No |
| Manage chains and token registry | No | Yes | No |
| Trigger background jobs manually | No | Yes | Scheduled |
| Access another user's data | No | No | Jobs process all data internally, never expose it |

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-USR-01 | Create an internal user record on first successful authentication, keyed by the identity provider's subject. | Must |
| FR-USR-02 | Every account, portfolio, snapshot, allocation, and order is owned by exactly one internal user. | Must |
| FR-USR-03 | All reads and writes on user-owned resources verify ownership. | Must |
| FR-USR-04 | Requests for another user's resource return not-found/forbidden without disclosing existence. | Must |
| FR-USR-05 | Only `admin`-role users reach administrative management; others are denied. | Must |
| FR-USR-06 | Audit security-sensitive actions: sign-in, account changes, credential changes, order decisions, admin changes. | Should |
| FR-USR-07 | User profile with display name and email, derived from the identity provider. | Should |
| FR-USR-08 | Prevent user deletion while portfolios exist, or define a communicated cascade/anonymisation policy. | Must |
| FR-USR-09 | Support disabling a user without deleting historical data. | Should |
| FR-USR-10 | Support sharing portfolios with Analyst/Viewer users. | Won't (MVP) |

## Business rules

| ID | Rule |
|----|------|
| BR-USR-01 | Passwords and MFA live with the identity provider, never in the application. |
| BR-USR-02 | The internal user ID is stable even if the provider rotates claims. |
| BR-USR-03 | Deny-by-default authorisation. |
| BR-USR-04 | Admin is additive: an Admin is also a Portfolio Owner. |
| BR-USR-05 | System jobs act for no user and never expose one user's data in another's response. |
| BR-USR-06 | Deleting an account removes only the connection record and its portfolio links. |
| BR-USR-07 | A portfolio may link multiple accounts; an account links to one portfolio at a time. |
| BR-USR-08 | An account may never link to another user's portfolio. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-USR-01 | A first-time user's first API request leaves exactly one internal user record mapped to their subject. |
| AC-USR-02 | User B requesting user A's portfolio gets no hint that it exists. |
| AC-USR-03 | A non-admin calling an admin endpoint is denied. |
| AC-USR-04 | An Admin's chain/token change is recorded and affects subsequent syncs. |
| AC-USR-05 | Two accounts linked to one portfolio: allowed. An account already linked, linked again (same or another portfolio): rejected. |
| AC-USR-06 | A scheduled job persists each portfolio's results against the correct owner. |

## Notes

- No email invitations, self-service registration, or shared portfolios in the MVP; the post-MVP model must be designable without migration blockers.
- Audit retention is defined in BRD-012.

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Exact claim/role name for Admin; realm-wide or client-scoped? | Security |
| OQ-002 | Soft-delete with recovery for accounts/portfolios? | Product |
| OQ-003 | Audit-event retention period? | Compliance |
| OQ-004 | User-deletion and data-export (erasure) policy? | Compliance |
