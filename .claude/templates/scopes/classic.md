---
name: classic
depth: Standard
keywords: []
description: "Full lifecycle through all five phases with one human approval per stage"
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
  # Operation
  - deployment-pipeline
  - environment-provisioning
  - deployment-execution
  - observability-setup
  - incident-response
  - performance-validation
  - feedback-optimization
skip: []
---

# classic scope

`classic` is the implicit default scope — used when neither the user nor
`defaultScope` names one. It is the original my-aidlc scope: the whole
lifecycle, one human approval per stage, advisory reviews, the learnings ritual
on.

## Why these stages

Every phase participates, so all 33 native stages are in the grid.

This is deliberately **wider than upstream's `classic`**, which is a v1-style
18-of-33 grid that drops Ideation entirely and ends at Build and Test. The
scope you authored at the start of this project described a full lifecycle
across all five phases, so that intent is what this file states: the space
layer wins over the native baseline, and `include` is the authoritative list.

Conditional stages (`reverse-engineering`, `market-research`, `feasibility`,
`team-formation`, `rough-mockups`, `refined-mockups`, `domain-design`,
`contract-design`, `practices-discovery`, and most of Operation) still
self-select at runtime and record a reason when they skip.

## Membership

All five phases, all 33 stages. Every stage still ends at a human approval
gate.
