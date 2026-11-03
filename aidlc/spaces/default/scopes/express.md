---
name: express
depth: Minimal
keywords:
  - express
  - quick
  - lightweight
  - small
  - tiny
description: "Lightest run: requirements to deploy, no design pass, no reviewers"
skeleton: off
review_cap: none
guard_policy: off
sensors: off
learnings: off
summary_confirmation: off
question_budget:
  min: 0
  max: 3
include:
  # Initialization
  - workspace-scaffold
  - workspace-detection
  - state-init
  # Brownfield understanding, when there is existing code
  - reverse-engineering
  # The contract
  - requirements-analysis
  # Implement and verify
  - code-generation
  - build-and-test
  # Ship, then watch it
  - deployment-pipeline
  - deployment-execution
  - observability-setup
skip:
  # Ideation ceremony
  - intent-capture
  - market-research
  - feasibility
  - scope-definition
  - team-formation
  - rough-mockups
  - approval-handoff
  # Design pass
  - user-stories
  - refined-mockups
  - domain-design
  - contract-design
  - units-generation
  - delivery-planning
  - practices-discovery
  # Design and pipeline detail
  - functional-design
  - nfr-requirements
  - nfr-design
  - infrastructure-design
  - ci-pipeline
  # Operations beyond deployment
  - environment-provisioning
  - incident-response
  - performance-validation
  - feedback-optimization
---

# express scope

`express` is the lightest useful run: a straight line from requirements to
code, test, deploy, and a first look at production, with no design pass and no
reviewers.

## Why these stages

`requirements-analysis` establishes the contract. `code-generation` implements
it and `build-and-test` verifies it. The Operation tail deploys and observes
the result. `reverse-engineering` stays in the grid because it is CONDITIONAL:
on a brownfield project it supplies the understanding the other stages need,
and on a greenfield one it self-skips with a recorded reason. The deploy tail
is CONDITIONAL too and self-skips when there is nothing to deploy.

`review_cap: none` disables reviewer passes, so `code-generation` runs without
its architecture reviewer. Guard Policy is off: changed inputs are recorded and
announced rather than reopening approval. Sensors, learnings, and summary
confirmation are off as well. Human approval gates still fire — this scope
removes ceremony, not the human.

## Membership

10 of 33 stages: the three Initialization stages, `reverse-engineering`,
`requirements-analysis`, `code-generation`, `build-and-test`,
`deployment-pipeline`, `deployment-execution`, and `observability-setup`.
