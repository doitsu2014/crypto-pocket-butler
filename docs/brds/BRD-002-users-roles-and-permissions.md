# BRD-002 — Users, Roles & Permissions

| Field | Value |
|-------|-------|
| Document ID | BRD-002 |
| Version | 1.0 |
| Status | Draft — requires product and security sign-off |
| Owner | Product |
| Last updated | 2026-11-01 |
| Related documents | BRD-003, BRD-006, BRD-010, BRD-012 |

## 1. Purpose

Define who interacts with Crypto Pocket Butler, what each actor may do, how user identity maps to application data, and the access-control rules that protect one user's data from another. This document covers both the MVP (single-owner accounts with an administrative role) and the post-MVP multi-user model.

## 2. Actors

| Actor | Description | MVP status |
|-------|-------------|------------|
| **Portfolio Owner** | Authenticated end user who owns accounts and portfolios | In scope |
| **Admin** | Authenticated user holding the `admin` role; also owns portfolios | In scope |
| **System** | Scheduled jobs and automated agent processes with no interactive session | In scope |
| **Analyst** | Read-only access to analytics and portfolios shared with them | Post-MVP |
| **Viewer** | Limited read-only access to specific shared portfolios | Post-MVP |
| **Anonymous visitor** | Unauthenticated browser user | In scope (sign-in only) |

## 3. Role model

### 3.1 MVP roles

| Role | Source of truth | Capabilities |
|------|-----------------|--------------|
| Portfolio Owner | Any authenticated Supabase Auth user | Manage own accounts, portfolios, snapshots, recommendations, and settings |
| Admin | Supabase Auth user with `app_metadata.role = "admin"`, projected into the JWT by the Custom Access Token Hook | Everything a Portfolio Owner can do **plus** manage EVM chains, EVM tokens, and Solana tokens |
| System | Internal scheduler / job runner | Execute background jobs against all users' data; never granted interactive access |

### 3.2 Post-MVP roles (target model)

| Role | Scope | Capabilities |
|------|-------|--------------|
| Owner | Workspace/tenant | Full access including billing and user management |
| Admin | Workspace/tenant | Portfolio and user management, no billing |
| Analyst | Shared portfolios | Read-only access to analytics and portfolio data |
| Viewer | Specific portfolios | Limited read-only view |

> The post-MVP model introduces shared portfolios and tenant-level roles. Its data model must be designed before multi-user launch; the MVP must not encode assumptions that prevent it.

## 4. Permission matrix (MVP)

| Capability | Owner | Admin | System | Anonymous |
|------------|:-----:|:-----:|:------:|:---------:|
| Sign in / sign out | Yes | Yes | N/A | Sign-in only |
| View and edit own profile | Yes | Yes | No | No |
| Create / edit / delete own accounts | Yes | Yes | No | No |
| Sync own accounts | Yes | Yes | Yes (scheduled) | No |
| Create / edit / delete own portfolios | Yes | Yes | No | No |
| Link / unlink own accounts | Yes | Yes | No | No |
| Set target allocation and guardrails | Yes | Yes | No | No |
| View own allocation and snapshots | Yes | Yes | Yes (writes snapshots) | No |
| Create manual snapshots | Yes | Yes | Yes (EOD) | No |
| Generate / view / accept / reject own recommendations | Yes | Yes | Generates scheduled recommendations (future) | No |
| Manage EVM chains | No | Yes | No | No |
| Manage EVM / Solana token registry | No | Yes | No | No |
| Trigger background jobs manually | No | Yes | Yes (scheduled) | No |
| Access another user's data | No | No | Jobs process all users' data internally | No |

## 5. Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-USR-01 | The system must create an internal user record on first successful authentication, keyed by the identity provider's subject identifier. | Must |
| FR-USR-02 | Every account, portfolio, snapshot, allocation, and recommendation must be owned by exactly one internal user. | Must |
| FR-USR-03 | All read and write operations on user-owned resources must verify ownership against the authenticated user. | Must |
| FR-USR-04 | A request for a resource owned by another user must return "not found" or "forbidden" without disclosing its existence. | Must |
| FR-USR-05 | Users holding the `admin` role must be able to access administrative resource management; other users must be denied. | Must |
| FR-USR-06 | The system should record an audit event for security-sensitive actions: sign-in, account create/update/delete, credential change, recommendation decision, and admin configuration change. | Should |
| FR-USR-07 | The system should support a user profile with at least display name and email, derived from the identity provider and editable where allowed. | Should |
| FR-USR-08 | The system must prevent deletion of a user while owned portfolios exist, or define and communicate a cascade/anonymization policy. | Must |
| FR-USR-09 | The system must support disabling or deactivating a user without deleting historical data. | Should |
| FR-USR-10 | The post-MVP model must support sharing a portfolio with Analyst/Viewer users and per-portfolio role assignment. | Won't (MVP) |

## 6. Business rules

| ID | Rule |
|----|------|
| BR-USR-01 | Identity is federated: passwords and multi-factor authentication are managed by the identity provider, not by the application. |
| BR-USR-02 | The internal user identifier is stable and must not change if the identity provider rotates other claims. |
| BR-USR-03 | Authorisation is deny-by-default: a capability not explicitly granted to a role is denied. |
| BR-USR-04 | Administrative access is additive: an Admin is also a Portfolio Owner and can manage their own portfolios. |
| BR-USR-05 | System jobs act on behalf of no user and must never expose one user's data in another user's response. |
| BR-USR-06 | Deleting an account must not delete the underlying exchange or wallet; it only removes the connection record and its portfolio links. |
| BR-USR-07 | An account may be linked to multiple portfolios owned by the same user. |
| BR-USR-08 | An account may not be linked to a portfolio owned by a different user. |

## 7. Data requirements

| ID | Data element | Description | Sensitivity |
|----|--------------|-------------|-------------|
| DR-USR-01 | Internal user ID | Stable UUID used as the owner key across all domain data | Internal |
| DR-USR-02 | Identity-provider subject ID | External identifier mapped to the internal user ID | Internal |
| DR-USR-03 | Email and display name | Profile information derived from the identity provider | Personal data |
| DR-USR-04 | Role assignments | User-to-role mapping, including `admin` | Internal |
| DR-USR-05 | Audit events | Actor, action, target, timestamp, outcome | Sensitive |
| DR-USR-06 | User status | Active / disabled | Internal |

## 8. Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-USR-01 | Given a first-time authenticated user, when their first API request is processed, then exactly one internal user record exists and is mapped to their identity-provider subject. |
| AC-USR-02 | Given user A owns a portfolio, when user B requests it, then the response does not reveal its existence or contents. |
| AC-USR-03 | Given a user without the `admin` role, when they call an administrative endpoint, then access is denied. |
| AC-USR-04 | Given an Admin, when they manage EVM chains or tokens, then the change is recorded and takes effect for subsequent syncs. |
| AC-USR-05 | Given a user with two accounts, when they link both to two different portfolios, then both links are allowed and allocation calculations remain isolated per portfolio. |
| AC-USR-06 | Given a scheduled job, when it processes all portfolios, then each portfolio's results are persisted against its correct owner. |

## 9. Assumptions, dependencies, constraints

- Supabase Auth is the source of truth for authentication. The admin role is stored in Supabase `app_metadata.role` and projected into the JWT by the Custom Access Token Hook.
- Role claims are delivered in the access token and validated by the API.
- The MVP does not support email invitations, self-service registration, or shared portfolios.
- Audit logging may be implemented as an append-only store; its retention period is defined in BRD-012.

## 10. Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | What exact claim/role name designates an Admin, and is it realm-wide or client-scoped? | Security |
| OQ-002 | Should account/portfolio deletion be soft-delete with recovery? | Product |
| OQ-003 | What is the retention period for audit events? | Compliance |
| OQ-004 | What is the user-deletion and data-export policy (right to erasure)? | Compliance |
