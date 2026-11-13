# Entities — u2-authorisation

**Upstream:** `unit-of-work.md` (U2 `u2-authorisation`), `components.md` (Authorisation), `stories.md` (US2.1–US2.9, US5.1–US5.3, US7.1–US7.4), `requirements.md` (FR15, FR18, FR20).

The Authorisation component owns **no persistent entity**. The caller's role lives in the
identity provider's `app_metadata.role` claim, and every other entity it reasons about — the
user record, the resource owner id — is owned by another unit and reached through that unit's
contract. What U2 does own is the *decision*: the value objects that turn a validated token, a
resource's owner, and the requested operation into an allow or a deny.

## Source of truth

```yaml
entities: []

value_objects:
  - name: CallerIdentity
    description: The caller a decision is made for — a user, or the system acting on its own behalf.
    variants:
      - name: user
        attributes:
          - name: subject
            type: string
            required: true
            constraints: "the provider subject from the validated token; never taken from a request body, header, or query"
          - name: userId
            type: uuid
            required: true
            constraints: "the internal user id resolved for that subject (u1-user-registry)"
          - name: role
            type: Role
            required: true
            constraints: "mapped from app_metadata.role only"
      - name: system
        attributes:
          - name: service
            type: string
            required: true
            constraints: "the job or service name; never a user id"
          - name: role
            type: Role
            required: true
            allowed: [system]
        constraints:
          - "A system caller is never an owner and never an admin: it may act only on rows it reads for a stated owner, and never on an administrative operation."
    constraints:
      - "A user caller identity is only ever built from a token that passed signature, issuer, audience, and expiry validation."
      - "Any role-shaped claim outside app_metadata.role is ignored, never merged."

  - name: Role
    description: The roles the release recognises for a user caller; a system caller carries its own role.
    allowed: [owner, admin, support, system]
    constraints:
      - "app_metadata.role value 'admin' maps to admin; 'support' maps to support; absent, empty, or any other value maps to owner."
      - "owner is the default and the floor: it is never denied a capability that the role model grants to everyone."
      - "system exists only for jobs; a caller-supplied value can never produce it."

  - name: Operation
    description: The named thing the caller is trying to do, chosen from the operation catalogue.
    attributes:
      - name: name
        type: string
        required: true
        constraints: "one of the catalogue's names; an unknown name is treated as undeclared"
      - name: class
        type: string
        required: true
        allowed: [owner-scoped, collection, administrative, confirmation, undeclared]
    constraints:
      - "The class is fixed by the catalogue, never by the caller: a request cannot promote itself to an administrative operation."

  - name: ResourceOwnership
    description: The ownership assertion a caller supplies for what it is about to touch.
    attributes:
      - name: resourceKind
        type: string
        required: true
        constraints: "the resource's domain name (account, portfolio, transaction, snapshot, allocation, order, preference, export, deletion request); it selects the catalogue entry"
      - name: scope
        type: string
        required: true
        allowed: [resource, collection]
        constraints: "resource for one row; collection for a list, search, or export"
      - name: resourceOwnerId
        type: uuid
        required: false
        constraints: "required when scope is resource; absent when scope is collection"
    constraints:
      - "A single row without exactly one owner is not authorisable: a missing or ambiguous owner is a deny, not an allow."
      - "A collection carries no owner id; the owning service must apply the owner filter to every row it returns, and the decision records that obligation."
      - "The decision compares resourceOwnerId to CallerIdentity.userId; the caller cannot satisfy the check by asserting a role."

  - name: AuthorisationDecision
    description: The outcome of one evaluation.
    attributes:
      - name: allowed
        type: boolean
        required: true
      - name: reason
        type: DenialReason
        required: true
      - name: httpStatus
        type: integer
        required: true
        allowed: [200, 401, 403, 404, 503]
      - name: filterOwnerId
        type: uuid
        required: false
        constraints: "present on an allowed collection decision: the owner every returned row must carry"
    constraints:
      - "A decision is total for every request that reaches the library; a token that fails validation never reaches it and is a 401 produced by the caller's middleware."
      - "The reason is for the audit record and the logs; the caller sees only the httpStatus and a problem body that does not disclose existence."

  - name: DenialReason
    description: Why a request was refused.
    allowed: [allowed, no_role, status_denied, not_owner, not_declared, audit_unavailable]
    constraints:
      - "not_owner and the nonexistent-resource case both answer 404 to the caller; only the audit record distinguishes them."
      - "audit_unavailable answers 503: the refusal could not be recorded, so the request fails rather than passing unaudited."

references:
  - name: UserStatus
    owner: u1-user-registry
    attributes: [pending, active, disabled, pending_deletion, deleted]
    used_for: "the status denial rule; read from u1's record, never duplicated here"
  - name: ProviderRoleClaim
    owner: Supabase Auth
    attribute: app_metadata.role
    used_for: "the only source of the caller's role; U2 writes it on a role change and never stores a second copy"
```

## Summary

| Item | Kind | Purpose |
|---|---|---|
| CallerIdentity | value object | Who is asking — a user, or the system for a job |
| Role | value object | owner / admin / support for users, system for jobs |
| Operation | value object | The named operation and its catalogue class |
| ResourceOwnership | value object | The owner assertion, or the collection obligation |
| AuthorisationDecision | value object | Allow or deny, its reason, the status, and the owner filter |
| DenialReason | value object | Why, for the audit record |
| UserStatus | reference | Owned by u1; read, never copied |
| ProviderRoleClaim | reference | Owned by the provider; read on every request, written on a role change |

## Decision recorded here

U2 stores no role, no permission set, and no ownership table. The role is a claim on every
request (so a role change takes effect at the caller's next request, NFR24), ownership is read
from the row the caller already loaded, and the only thing U2 writes is the provider's role
claim on an admin-initiated change and the audit entries around it. This keeps the unit
stateless and removes the "two sources of truth for a role" failure mode entirely.
