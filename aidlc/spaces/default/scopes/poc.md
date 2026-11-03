---
name: poc
depth: Minimal
keywords:
  - poc
  - proof of concept
  - spike
  - experiment
  - prototype
  - validate idea
description: "Prove or disprove a technical assumption, then stop"
skeleton: on
review_cap: none
guard_policy: off
sensors: off
learnings: on
summary_confirmation: off
question_budget:
  min: 1
  max: 3
include:
  # Initialization
  - workspace-scaffold
  - workspace-detection
  - state-init
  # Frame the assumption
  - intent-capture
  - feasibility
  # The success signal
  - requirements-analysis
  # The smallest thing that tests it
  - code-generation
skip:
  # Ideation ceremony
  - market-research
  - scope-definition
  - team-formation
  - rough-mockups
  - approval-handoff
  # Design ceremony
  - reverse-engineering
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
  - build-and-test
  - ci-pipeline
  # Everything operational
  - deployment-pipeline
  - environment-provisioning
  - deployment-execution
  - observability-setup
  - incident-response
  - performance-validation
  - feedback-optimization
---

# poc scope

`poc` proves or disproves a single technical assumption fast. The output is a
finding, not a product. Do not carry proof-of-concept code to production.

## Why these stages

`intent-capture` frames the assumption, `feasibility` records what is already
known about it, and `requirements-analysis` captures the success signal that
decides the question. `code-generation` builds the smallest thing that tests
it. Then the workflow stops.

This is leaner than upstream's `poc` (8 stages), which keeps
`build-and-test`. The original scope said "everything else is SKIP", and a
spike's evidence usually comes from running the experiment by hand rather than
from a generated suite — so `build-and-test` is out here. If a particular spike
does owe a repeatable check, add `build-and-test` to `include` in this file for
that intent; the space layer is the place to make that call.

`review_cap: none` keeps the reviewer out of a spike. Guard Policy is off.
Learnings stay on: a disproved assumption is exactly the kind of thing the next
intent needs to know.

## Membership

7 of 33 stages. Everything listed under `skip` is SKIP.
