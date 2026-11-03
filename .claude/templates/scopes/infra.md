---
name: infra
depth: Standard
keywords:
  - infra
  - infrastructure
  - terraform
  - kubernetes
  - pipeline
  - platform
  - observability
description: "Design, deploy, and operate infrastructure and platforms"
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
  - feasibility
  - scope-definition
  # Inception
  - reverse-engineering
  - requirements-analysis
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
skip:
  - market-research
  - team-formation
  - rough-mockups
  - approval-handoff
  - user-stories
  - refined-mockups
---

# infra scope

`infra` is for platform and infrastructure work: environments, pipelines,
deployment automation, and observability. The target is a topology and its
operation, so the user-facing design stages give way to non-functional and
infrastructure design, and the full Operation phase runs.

## Why these stages

`nfr-requirements`, `nfr-design`, and `infrastructure-design` define the
performance, security, scalability, reliability, and observability posture of
the target environment. `practices-discovery` captures the operational rules
already in force. Construction still generates and tests the code that
expresses the topology.

**All seven Operation stages run**, including `feedback-optimization`: the
original scope covered the Curate phase — now Operation — in full, and for
infrastructure that
is where cost analysis, drift reports, and the feedback loop belong.
`deployment-pipeline`, `environment-provisioning`, and
`deployment-execution` are the point of the scope.

The skipped stages are the product-facing ones: market research, team
formation, rough mockups, the approval handoff, user stories, and refined
mockups. None of them change a network boundary or a pipeline.

## Membership

27 of 33 stages. Everything listed under `skip` is SKIP.
