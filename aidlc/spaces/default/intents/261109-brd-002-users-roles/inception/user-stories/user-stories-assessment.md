# User Stories — Assessment

**Stage:** inception/user-stories · **Decision:** Execute

## Rationale

This work is user-facing, has several distinct personas with very different privileges, and encodes security-sensitive business rules (ownership, support grants, deletion, jurisdiction). Stories add value here because they turn the requirements into independently testable user journeys that Developers and QA can build against, and they expose privilege boundaries that a requirements table states but does not walk through.

## Factors Considered

- **Project type:** brownfield product with an existing Identity service and support console.
- **User-facing scope:** yes — account settings, support console, admin user management, sign-up gates.
- **Complexity signals:** four actor roles, deny-by-default authorisation, time-bound support grants, a delete/recover state machine, legal-hold overrides, jurisdiction gating.
- **Traceability need:** requirements FR14–FR20 (38 sub-requirements) must be covered by stories before design starts.

## Key Areas Where Stories Add the Most Value

- The four-role permission boundaries (Owner, Admin, Support, Anonymous), where a single missed check is a security defect.
- The support-access journey end to end: request, approval, time window, metadata boundary, expiry, audit.
- The deletion and recovery state machine, including the 30-day window and legal-hold override.
- The sign-up gates (terms, age, jurisdiction) that must run before any personal data is stored.

## Coverage Approach

The decision from the plan questions was taken as recorded:

- **Personas:** four — Portfolio Owner, Admin, Support, Anonymous visitor; System is a non-user actor.
- **Breakdown:** by requirement group, one story group per FR14–FR20.
- **Granularity:** one story per functional sub-requirement (38 stories).
- **Priorities:** every BRD Must is Must Have; shared portfolios (FR-USR-10) is Won't Have; no Should/Could in this set.
- **Persona cards:** full — role, goals, pain points, context, tech comfort, frequency.

## Alternatives Considered

- **Coarser stories (15–20):** rejected because one-story-per-sub-requirement keeps the requirement-to-story trace exact for the security rules.
- **Journey-based breakdown:** rejected because it would cut across the requirement groups and make coverage harder to verify.
- **Skipping stories:** rejected — the privilege differences and the multi-step support/deletion journeys are exactly where vague acceptance criteria cause defects.
