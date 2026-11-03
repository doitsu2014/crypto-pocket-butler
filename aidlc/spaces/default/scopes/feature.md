---
name: feature
depth: Standard
keywords:
  - feature
  - add
  - enhance
  - capability
  - story
description: "Full design pass from intent through a validated release"
skeleton: off
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
  # Ideation
  - intent-capture
  - market-research
  - feasibility
  - scope-definition
  - team-formation
  - rough-mockups
  - approval-handoff
  # Inception
  - reverse-engineering
  - requirements-analysis
  - user-stories
  - refined-mockups
  - domain-design
  - contract-design
  - units-generation
  - delivery-planning
  - practices-discovery
  # Construction
  - functional-design
  - nfr-requirements
  - nfr-design
  - infrastructure-design
  - code-generation
  - build-and-test
  - ci-pipeline
  # Operation, up to a validated release
  - deployment-pipeline
  - environment-provisioning
  - deployment-execution
  - observability-setup
  - incident-response
  - performance-validation
skip:
  - feedback-optimization
---

# feature scope

`feature` is the profile for adding a capability to an existing system. It runs
the full design pass and stops at a validated release.

## Why these stages

Everything runs except `feedback-optimization`. The design pass is complete:
requirements, stories, refined mockups, domain and contract design, units, and
the non-functional and infrastructure design stages all participate, which is
what the original scope meant by "a full design pass".

`feedback-optimization` is the continuous-curation stage — SLO reports, cost
analysis, drift reports, and the feedback loop. A feature hand-off ends before
that: the delivered capability is validated, deployed, and observed, and
ongoing optimisation is a separate intent. `incident-response` and
`performance-validation` stay because both are CONDITIONAL readiness and
verification work that a shipped feature may owe its operators.

This is deliberately **wider than upstream's `classic`** and almost identical to
upstream's `feature`, differing only in the one skipped stage.

## Membership

32 of 33 stages. `feedback-optimization` is SKIP.
