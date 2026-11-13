# Domain Design — Questions

The domain design models the **logical building blocks** for BRD-002: code you write, with its own business logic and owned entities. Deployment topology and tech stack are decided later. The existing system already has an Identity service, an API Gateway, and an Audit service, so the questions are about how BRD-002's responsibilities border inside them.

Answer each question with a letter after `[Answer]:`, or pick `X` and say what you want instead.

## Q1. [architect] How should the BRD-002 responsibilities be split into components?

The work spans user records, authorisation decisions, support grants, preferences and consent, the status/deletion lifecycle, and the jurisdiction gate.

- A. Six cohesive components — User Registry, Authorisation, Support Access, Preferences & Consent, Account Lifecycle, Jurisdiction Gate — inside the Identity bounded context
- B. One Identity component with internal modules for each responsibility
- C. Three components — User Registry (records, preferences, consent), Access Control (authorisation, support grants, jurisdiction), Account Lifecycle (status and deletion)
- X. Other (please specify)

[Answer]: A — Six cohesive components inside the Identity bounded context.

## Q2. [architect] Where should authorisation decisions live?

The gateway today validates the JWT and applies a deny-by-default route table with an admin gate; each service re-validates the token and applies ownership checks.

- A. Gateway keeps coarse route and admin gating; a dedicated Authorisation component owns role, status, and ownership decisions that services call
- B. Gateway keeps the route/admin gate; each service keeps its own ownership and status checks, with role mapping shared as a small domain library (current pattern)
- C. Move all authorisation into the gateway
- X. Other (please specify)

[Answer]: A — Gateway keeps coarse route and admin gating; a dedicated Authorisation component owns role, status, and ownership decisions that services call.

## Q3. [architect] Should support access be its own component?

Support grants have their own lifecycle (request, distinct-admin approval, 60-minute window, revocation) and a deliberately narrow metadata projection.

- A. A separate Support Access component owning grants and the metadata projection, used by the support console and admin
- B. Part of the User Registry component, sharing its entity ownership
- C. Part of the Authorisation component, since a grant is an authorisation artefact
- X. Other (please specify)

[Answer]: A — A separate Support Access component owning grants and the metadata projection.

## Q4. [architect] How should the UI be structured into components?

The mockups define four surfaces: account settings, sign-up gates, support console, and admin user management, plus three shared additions (RoleBadge, StatusPill, GrantBanner).

- A. One feature module per surface, sharing the three components and the existing design-system primitives
- B. A single Account component rendering all four surfaces
- C. Two modules: self-service (settings and gates) and operations (support console and admin)
- X. Other (please specify)

[Answer]: A — One feature module per surface, sharing the three components and the existing design-system primitives.
