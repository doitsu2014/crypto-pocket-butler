---
name: bugfix
depth: Minimal
keywords:
  - bug
  - fix
  - regression
  - hotfix
  - defect
  - broken
description: "Fast path from a reproduced defect to a verified fix"
skeleton: off
review_cap: advisory
guard_policy: relaxed
sensors: off
learnings: on
summary_confirmation: off
include:
  # Initialization
  - workspace-scaffold
  - workspace-detection
  - state-init
  # Frame the defect
  - intent-capture
  # Understand the code the defect lives in
  - reverse-engineering
  # The defect statement
  - requirements-analysis
  # Fix and verify
  - code-generation
  - build-and-test
  # Ship the fix
  - deployment-pipeline
  - deployment-execution
skip:
  # Ideation ceremony
  - market-research
  - feasibility
  - scope-definition
  - team-formation
  - rough-mockups
  - approval-handoff
  # Design ceremony
  - user-stories
  - refined-mockups
  - domain-design
  - contract-design
  - units-generation
  - delivery-planning
  - practices-discovery
  - functional-design
  - nfr-requirements
  - nfr-design
  - infrastructure-design
  - ci-pipeline
  # Operations beyond shipping the fix
  - environment-provisioning
  - observability-setup
  - incident-response
  - performance-validation
  - feedback-optimization
---

# bugfix scope

`bugfix` is a fast, evidence-driven path: reproduce, fix, verify, ship. The
design and planning ceremony is skipped; the review and validation ceremony
stays.

## Why these stages

`intent-capture` becomes a defect statement, and `reverse-engineering` — which
is CONDITIONAL and self-skips when the defect is unambiguous and the code is
already understood — grounds it in the actual implementation.
`requirements-analysis` turns the defect into a testable statement of the
correct behaviour. `code-generation` applies the fix with its advisory review
intact, and `build-and-test` proves it builds and passes. The two deployment
stages carry the verified fix to the environment where it has to hold.

`code-generation` keeps its architecture reviewer; this is a real fix, not a
prototype, and the original scope kept the review and validation ceremony.
Sensors are off, matching the original scope; the learnings ritual stays on so
the next defect starts from what this one taught.

## Membership

10 of 33 stages. Everything listed under `skip` is SKIP.
