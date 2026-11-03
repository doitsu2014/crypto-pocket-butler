---
name: mvp
depth: Standard
keywords:
  - mvp
  - prototype
  - launch fast
  - ship fast
  - first version
description: "Thin design pass from intent to a shippable first version"
skeleton: on
review_cap: advisory
guard_policy: relaxed
sensors: on
learnings: on
summary_confirmation: off
include:
  # Initialization
  - workspace-scaffold
  - workspace-detection
  - state-init
  # Ideation, kept deliberately thin
  - intent-capture
  - feasibility
  - scope-definition
  - rough-mockups
  # Inception: just enough design to build against
  - reverse-engineering
  - requirements-analysis
  - domain-design
  - units-generation
  - delivery-planning
  # Construction
  - functional-design
  - code-generation
  - build-and-test
  - ci-pipeline
skip:
  # Ideation we can defer
  - market-research
  - team-formation
  - approval-handoff
  # Design we can defer
  - user-stories
  - refined-mockups
  - contract-design
  - practices-discovery
  - nfr-requirements
  - nfr-design
  - infrastructure-design
  # All of Operation
  - deployment-pipeline
  - environment-provisioning
  - deployment-execution
  - observability-setup
  - incident-response
  - performance-validation
  - feedback-optimization
---

# mvp scope

`mvp` gets a first version in front of users with the minimum design needed to
ship safely. It favours a walking skeleton (`skeleton: on`) and a thin design
pass over a complete specification.

## Why these stages

`intent-capture`, `feasibility`, `scope-definition`, and `rough-mockups` frame
the first version and its boundary. `domain-design` and `units-generation`
stay because even a first version needs boundaries and independently
verifiable units; `delivery-planning` sequences them. `functional-design`
fixes the behaviour, `code-generation` builds it, `build-and-test` proves it,
and `ci-pipeline` makes the build repeatable.

Formal API contracts, pattern selection, and non-functional design are
deferred — a first version's non-functional posture is provisional by
definition. **All seven Operation stages are skipped**: the original scope
shipped the first version manually or on a managed platform, and deployment
automation belongs to the production intent that follows.

This matches upstream's `mvp` shape (which is also Operation-free) while
keeping the thinner inception the original scope described.

## Membership

16 of 33 stages. Everything listed under `skip` is SKIP.
