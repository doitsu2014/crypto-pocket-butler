# Architecture Decision Records — Users, Roles & Permissions

**Status:** Proposed · **Date:** 2026-11-10

## ADR-001: Six domain components inside the Identity bounded context

### Context

BRD-002 covers user records, authorisation decisions, support grants, preferences and consent, the status/deletion lifecycle, and jurisdiction gating. The existing system already has an Identity service that owns the `identity` schema, and the team's code style requires four-crate Clean Architecture per service. The choice is how to carve the business logic into components, not into services.

### Decision

Model six domain components — UserRegistry, Authorisation, SupportAccess, PreferencesConsent, AccountLifecycle, JurisdictionGate — as cohesive building blocks inside the Identity bounded context.

### Consequences

- **Positive:** Each component has one nameable responsibility and one lifecycle; jurisdiction gating is explicit and ordered before any data write; support access can be tested with its own narrow projection.
- **Negative:** More crates/modules to wire and more ports between components; a cross-component call is required where a single object would do.
- **Neutral:** Deployment stays one Identity service; the split is logical and can be promoted to separate services later if change rates diverge.

### Alternatives Rejected

- **One Identity component with internal modules** — simpler, but the six responsibilities share a change surface and the gate ordering becomes implicit.
- **Three components (User Registry / Access Control / Account Lifecycle)** — merges authorisation, support access, and jurisdiction, which have different lifecycles and failure modes.

## ADR-002: A dedicated Authorisation component with coarse gateway gating

### Context

The gateway validates JWTs and applies a deny-by-default route table with an admin gate; each service re-validates the token and applies ownership checks. BRD-002 adds status-based denial, a strict role claim, and a single not-found/forbidden response rule. Duplicating those rules per service would drift.

### Decision

Keep the gateway's coarse route and admin gating, and give Authorisation a dedicated component that owns the role mapping, status denial, ownership contract, and denial evidence. Services consult it for decisions; the gateway remains a fast edge filter.

### Consequences

- **Positive:** One place to change a rule; denials are consistent and auditable; status denial cannot be forgotten in a new endpoint.
- **Negative:** A call per decision (or a shared library) where a local check used to be; the gateway and Authorisation must agree on role mapping.
- **Neutral:** The gateway still refuses undeclared routes before any data work, which keeps blast radius small.

### Alternatives Rejected

- **Gateway-only authorisation** — the gateway would need account status and ownership data it must not hold.
- **Per-service inline checks (current pattern)** — no single decision point; the status rule would be re-implemented in every service.

## ADR-003: SupportAccess is its own component

### Context

Support grants have a distinct lifecycle (request, different-admin approval, a 60-minute window, revocation), a distinct audit obligation, and a deliberately narrow metadata projection. They also must never expose credentials, balances, or portfolio contents.

### Decision

Model SupportAccess as a separate component that owns `SupportGrant` and the metadata projection, used by the support console and admin surfaces.

### Consequences

- **Positive:** The projection boundary is explicit and testable; the grant lifecycle is isolated; approval distinctness is enforced in one place.
- **Negative:** It needs UserRegistry and Authorisation to resolve people and roles; one more component to wire.
- **Neutral:** The grant audit events are emitted by the component and written by the Audit service.

### Alternatives Rejected

- **Part of UserRegistry** — conflates a read-only, time-bound access artefact with the user record itself.
- **Part of Authorisation** — a grant is an authorisation artefact, but its lifecycle and projection deserve their own tests; folding it in would make Authorisation stateful.

## ADR-004: One UI feature module per surface

### Context

The mockups define four surfaces with different actors and states: account settings, sign-up gates, the support console, and admin user management. They share three presentation components and the Parchment Ledger primitives.

### Decision

Build one feature module per surface, sharing RoleBadge, StatusPill, GrantBanner, and the existing design-system primitives.

### Consequences

- **Positive:** Each surface can evolve and be tested independently; actor-specific states stay local.
- **Negative:** Some duplication of layout and state handling; shared components need a clear contract.
- **Neutral:** The SPA and support console are separate packages already, so the split follows the existing repository topology.

### Alternatives Rejected

- **A single Account component** — collapses four actors and two privilege levels into one change surface.
- **Two modules (self-service / operations)** — still mixes the settings and gate journeys, which have different entry conditions.

## ADR-005: Jurisdiction gate runs before any personal data is written

### Context

BRD-002 requires a restricted sign-up to be denied before any personal data is stored (FR19.4, BR-USR-12, AC-USR-11). A gate that runs after provisioning would already have stored the subject hash and profile.

### Decision

UserRegistry depends on JurisdictionGate: no user record, profile row, or shredding key is created until the gate clears the provider's self-declared country and IP signal.

### Consequences

- **Positive:** The "nothing stored" promise is structural, not procedural; the gate has one caller order.
- **Negative:** Provisioning now has a synchronous dependency on the gate and the geolocation provider; a gate outage blocks first sign-in.
- **Neutral:** A failed check stores nothing and is audited without personal data; a gate outage must return a retry, not a pass.

### Alternatives Rejected

- **Check after provisioning and delete on failure** — violates "before any personal data is stored" and creates an erasure path.
- **Gateway-only gate** — the gateway has no user-provisioning knowledge and would still let a service create the record.

## ADR-006: `User.status` is owned by UserRegistry and mutated only through AccountLifecycle

### Context

Four statuses (`active`, `disabled`, `pending_deletion`, `deleted`) drive denial, and the delete/recover state machine has a 30-day window plus legal-hold rules. The status attribute naturally sits on the user record, but its transition rules belong to the lifecycle.

### Decision

UserRegistry owns the `User` entity and the `status` attribute; AccountLifecycle owns the transition rules and is the only component that may request a status change. Admin commands and user deletion both go through AccountLifecycle.

### Consequences

- **Positive:** One owner per entity; every transition passes the lifecycle rules; audit events have a single source.
- **Negative:** A status write is a two-step internal flow (lifecycle → registry); a direct registry write must be prohibited by the port design.
- **Neutral:** Reads of status by Authorisation stay cheap.

### Alternatives Rejected

- **AccountLifecycle owns the status attribute** — splits the user record across two owners and forces Authorisation to read two places.
- **Status transitions inside UserRegistry** — mixes record storage with lifecycle policy and makes legal-hold rules a storage concern.
