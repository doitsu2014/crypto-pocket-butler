# Artifact Vocabulary

This is the artifact vocabulary of the native AWS AI-DLC methodology that
my-aidlc runs. Every name below comes from the release pinned in
`aidlc-version.json` - AI-DLC 2.11.0, tag `v2.11.0`, commit
`6a378b53c0a4fe0641ed7d8de8dfff94264d5b6a` - snapshotted under `core/native/`.
Each name is the literal `produces:` or `consumes:` entry of a stage file at
`core/native/stages/<phase>/<slug>.md`. This document invents no names.

The engine writes artifact files; stages and agents never choose paths. A
produced artifact lands at:

    aidlc/spaces/<space>/intents/<intent>/<phase>/<stage>/

Names are lowercase-kebab. A stage's `produces:` list is what it writes and its
`consumes:` list is what it reads.

The 33 native stages declare 128 `produces:` entries covering 121 slugs, plus
one `optional_produces:` slug (`frontend-components`) - 122 distinct artifacts.
`traceability` is the only slug declared by more than one stage.

## How to read this reference

- `produces:` - artifacts the stage writes, verbatim from its frontmatter and
  in declaration order.
- `produces (optional):` - declared under `optional_produces:` and written only
  when the stage's condition selects them.
- `reads:` - artifacts the stage consumes. `(optional)` marks a non-required
  input; `[brownfield]` marks an input gated by `conditional_on`. A required
  input that is absent is a gap the engine reports as `consumes_absent`.
- Stages appear in the order the engine resolves for a scope that includes all
  33 slugs (topological by `requires_stage`).

## Where files land

The default for every stage is
`aidlc/spaces/<space>/intents/<intent>/<phase>/<stage>/`. Three exceptions:

- Per-unit stages. The `for_each: unit-of-work` stages - functional-design,
  nfr-requirements, nfr-design, infrastructure-design, and code-generation -
  run once per unit and write under a per-unit record directory inside their
  stage directory.
- reverse-engineering writes its nine artifacts to the space knowledge store
  `aidlc/spaces/<space>/codekb/<repo>/` instead of an intent record. Its
  `reverse-engineering-timestamp` is the freshness marker the rerun guard
  reads.
- practices-discovery writes its stage artifacts plus one contribution file
  per support agent; on affirmation the content is promoted to
  `aidlc/spaces/<space>/memory/team.md` and `project.md`.

The three Initialization stages declare no produced artifacts. They create
`<record>/aidlc-state.md`, the per-intent record tree, and the space-level
knowledge directory, all engine-owned. `-questions` artifacts are per-stage
discussion records; they are listed with their stage like any other artifact.

## Initialization

### workspace-scaffold

- execution: ALWAYS
- produces: none - creates the per-intent record tree (one directory per
  in-scope phase, plus the verification directory) and the space knowledge
  directory
- reads: none

### workspace-detection

- execution: ALWAYS
- produces: none - records the workspace classification (greenfield or
  brownfield) and technology stack detection
- reads: none

### state-init

- execution: ALWAYS
- produces: none - writes the fully populated `<record>/aidlc-state.md` and
  determines routing
- reads: none

## Ideation

### intent-capture

- execution: ALWAYS
- produces: `intent-statement`, `stakeholder-map`, `intent-capture-questions`
- reads: none

### market-research

- execution: CONDITIONAL
- produces: `competitive-analysis`, `market-trends`, `build-vs-buy`,
  `market-research-questions`
- reads: `intent-statement`

### feasibility

- execution: CONDITIONAL
- produces: `feasibility-assessment`, `constraint-register`, `raid-log`,
  `feasibility-questions`
- reads: `intent-statement`, `competitive-analysis` (optional),
  `market-trends` (optional), `build-vs-buy` (optional)

### scope-definition

- execution: ALWAYS
- produces: `scope-document`, `intent-backlog`, `scope-definition-questions`
- reads: `intent-statement`, `feasibility-assessment` (optional),
  `constraint-register` (optional)

### team-formation

- execution: CONDITIONAL
- produces: `team-assessment`, `skill-matrix`, `mob-composition`,
  `team-formation-questions`
- reads: `scope-document`, `intent-backlog`, `feasibility-assessment`
  (optional)

### rough-mockups

- execution: CONDITIONAL
- produces: `wireframes`, `user-flow`, `rough-mockups-questions`
- reads: `intent-statement`, `scope-document`, `intent-backlog`

### approval-handoff

- execution: ALWAYS
- produces: `initiative-brief`, `decision-log`, `approval-handoff-questions`
- reads: `intent-statement`, `stakeholder-map`, `scope-document`,
  `intent-backlog`, `competitive-analysis` (optional),
  `feasibility-assessment` (optional), `constraint-register` (optional),
  `team-assessment` (optional), `wireframes` (optional)

## Inception

### reverse-engineering

- execution: CONDITIONAL (brownfield only)
- produces: `business-overview`, `architecture`, `code-structure`,
  `api-documentation`, `component-inventory`, `technology-stack`,
  `dependencies`, `code-quality-assessment`, `reverse-engineering-timestamp`
- reads: none
- path: `aidlc/spaces/<space>/codekb/<repo>/`, not the intent record

### practices-discovery

- execution: CONDITIONAL
- produces: `team-practices`, `discovered-rules`, `evidence`,
  `practices-discovery-timestamp`
- reads: `code-structure` (optional) [brownfield], `technology-stack`
  (optional) [brownfield], `dependencies` (optional) [brownfield],
  `code-quality-assessment` (optional) [brownfield], `architecture`
  (optional) [brownfield], `business-overview` (optional) [brownfield]

### requirements-analysis

- execution: ALWAYS
- produces: `requirements`, `requirements-analysis-questions`
- reads: `intent-statement` (optional), `scope-document` (optional),
  `business-overview` (optional) [brownfield], `architecture` (optional)
  [brownfield], `code-structure` (optional) [brownfield], `team-practices`
  (optional)

### user-stories

- execution: CONDITIONAL
- produces: `stories`, `personas`, `user-stories-assessment`, `traceability`
- reads: `requirements`, `business-overview` (optional) [brownfield],
  `component-inventory` (optional) [brownfield], `team-practices` (optional)

### refined-mockups

- execution: CONDITIONAL
- produces: `mockups`, `interaction-spec`, `design-system-mapping`,
  `accessibility-checklist`, `refined-mockups-questions`
- reads: `wireframes`, `user-flow`, `stories` (optional), `requirements`,
  `team-practices` (optional)

### domain-design

- execution: CONDITIONAL
- produces: `components`, `decisions`, `traceability`
- reads: `requirements`, `stories` (optional), `architecture` (optional)
  [brownfield], `component-inventory` (optional) [brownfield],
  `team-practices` (optional)

### units-generation

- execution: ALWAYS
- produces: `unit-of-work`, `unit-of-work-dependency`,
  `unit-of-work-story-map`, `traceability`
- reads: `components`, `decisions` (optional), `requirements`, `stories`
  (optional)

### contract-design

- execution: CONDITIONAL
- produces: `contract-summary`
- reads: `unit-of-work`, `unit-of-work-dependency`, `components` (optional),
  `requirements` (optional)

### delivery-planning

- execution: ALWAYS
- produces: `bolt-plan`, `team-allocation`, `risk-and-sequencing-rationale`,
  `external-dependency-map`, `delivery-planning-questions`
- reads: `requirements`, `stories` (optional), `mockups` (optional),
  `components`, `unit-of-work`, `unit-of-work-dependency`,
  `unit-of-work-story-map` (optional), `contract-summary` (optional),
  `team-practices` (optional)

## Construction

### functional-design

- execution: CONDITIONAL
- for_each: unit-of-work
- produces: `entities`, `rules`, `functional-spec`, `traceability`
- produces (optional): `frontend-components`
- reads: `unit-of-work`, `unit-of-work-story-map` (optional), `requirements`,
  `components`, `contract-summary` (optional)

### nfr-requirements

- execution: CONDITIONAL
- for_each: unit-of-work
- produces: `performance-requirements`, `security-requirements`,
  `scalability-requirements`, `reliability-requirements`,
  `observability-requirements`, `tech-stack-decisions`, `traceability`
- reads: `functional-spec`, `rules`, `requirements`, `contract-summary`
  (optional), `technology-stack` (optional) [brownfield]

### nfr-design

- execution: CONDITIONAL
- for_each: unit-of-work
- produces: `performance-design`, `security-design`, `scalability-design`,
  `reliability-design`, `observability-design`, `logical-components`,
  `traceability`
- reads: `performance-requirements`, `security-requirements`,
  `scalability-requirements`, `reliability-requirements`,
  `observability-requirements`, `tech-stack-decisions`, `functional-spec`,
  `contract-summary` (optional)

### infrastructure-design

- execution: CONDITIONAL
- for_each: unit-of-work
- produces: `infrastructure-specification`, `monitoring-design`,
  `cicd-pipeline`, `traceability`
- reads: `performance-design`, `security-design`, `scalability-design`,
  `reliability-design`, `observability-design`, `logical-components`,
  `components`, `functional-spec`, `contract-summary` (optional)

### code-generation

- execution: ALWAYS
- for_each: unit-of-work
- produces: `code-generation-plan`, `unit-test-instructions`, `code-summary`,
  `traceability`
- reads: `functional-spec` (optional), `rules` (optional), `entities`
  (optional), `contract-summary` (optional), `performance-design` (optional),
  `security-design` (optional), `infrastructure-specification` (optional),
  `unit-of-work`, `requirements`

### build-and-test

- execution: ALWAYS
- produces: `build-instructions`, `integration-test-instructions`,
  `performance-test-instructions`, `security-test-instructions`,
  `build-and-test-summary`, `build-test-results`, `cross-unit-traceability`
- reads: `code-generation-plan`, `unit-test-instructions`, `code-summary`

### ci-pipeline

- execution: CONDITIONAL
- produces: `ci-config`, `quality-gates`, `ci-pipeline-questions`
- reads: `code-summary`, `build-and-test-summary`, `build-test-results`

## Operation

### deployment-pipeline

- execution: CONDITIONAL
- produces: `cd-config`, `deployment-strategy`, `rollback-runbook`,
  `deployment-pipeline-questions`
- reads: `ci-config`, `quality-gates`, `infrastructure-specification`,
  `cicd-pipeline`

### environment-provisioning

- execution: CONDITIONAL
- produces: `environment-inventory`, `validation-report`,
  `environment-provisioning-questions`
- reads: `infrastructure-specification`, `cd-config`

### deployment-execution

- execution: CONDITIONAL
- produces: `deployment-log`, `smoke-test-results`, `health-check-report`,
  `deployment-execution-questions`
- reads: `cd-config`, `deployment-strategy`, `environment-inventory`,
  `build-test-results`

### observability-setup

- execution: CONDITIONAL
- produces: `dashboards`, `alarms`, `slo-config`, `log-queries`,
  `tracing-config`, `anomaly-config`, `observability-setup-questions`
- reads: `performance-design`, `security-design`, `reliability-design`,
  `monitoring-design`, `infrastructure-specification`

### incident-response

- execution: CONDITIONAL
- produces: `runbooks`, `incident-plan`, `escalation-matrix`,
  `incident-response-questions`
- reads: `dashboards`, `alarms`, `reliability-design`, `security-design`,
  `infrastructure-specification`

### performance-validation

- execution: CONDITIONAL
- produces: `load-test-plan`, `load-test-results`, `nfr-validation-matrix`,
  `performance-validation-questions`
- reads: `performance-requirements`, `scalability-requirements`,
  `performance-design`, `scalability-design`, `dashboards`

### feedback-optimization

- execution: CONDITIONAL
- produces: `slo-report`, `cost-analysis`, `drift-report`, `feedback-loop`,
  `feedback-optimization-questions`
- reads: `dashboards`, `alarms`, `slo-config`, `deployment-log`,
  `load-test-results` (optional), `incident-plan` (optional)

## Cross-phase handoffs

Artifacts a stage reads in a phase other than the one that produced them. The
producing stage runs first whatever a scope's `include:` order says.

- Ideation to Inception: `intent-statement`, `scope-document`, `wireframes`,
  and `user-flow` feed requirements-analysis, refined-mockups, and the
  inception design stages.
- Inception to Construction: `requirements`, `components`, `technology-stack`,
  `contract-summary`, `unit-of-work`, and `unit-of-work-story-map` feed the
  per-unit design and code-generation stages.
- Construction to Operation: `ci-config`, `cicd-pipeline`, `quality-gates`,
  `monitoring-design`, `infrastructure-specification`, `performance-design`,
  `performance-requirements`, `scalability-design`, `scalability-requirements`,
  `security-design`, and `reliability-design` feed the pipeline, deploy,
  observability, and validation stages.
- Multi-producer: `traceability` is written by eight stages - domain-design,
  user-stories, units-generation, functional-design, nfr-requirements,
  nfr-design, infrastructure-design, and code-generation - one file per stage
  record directory.
- No artifact is consumed by a stage unless some stage produces it; lint
  enforces that closure.

## Naming notes

- `-questions` artifacts are stage discussion records. Every gated stage
  declares one and the engine also writes some of them directly.
- A few stage bodies name their output file differently from the slug:
  build-and-test writes `test-results.md` for `build-test-results`, and
  performance-validation writes `test-results.md` for `load-test-results`. The
  slug in `produces:` is authoritative.
- `frontend-components` is the only artifact declared as an optional produce,
  and only for UI units.

## Artifact index

Alphabetical. Each entry names the phase whose stage produces the artifact.

- `accessibility-checklist` - produced in inception
- `alarms` - produced in operation
- `anomaly-config` - produced in operation
- `api-documentation` - produced in inception
- `approval-handoff-questions` - produced in ideation
- `architecture` - produced in inception
- `bolt-plan` - produced in inception
- `build-and-test-summary` - produced in construction
- `build-instructions` - produced in construction
- `build-test-results` - produced in construction
- `build-vs-buy` - produced in ideation
- `business-overview` - produced in inception
- `cd-config` - produced in operation
- `ci-config` - produced in construction
- `ci-pipeline-questions` - produced in construction
- `cicd-pipeline` - produced in construction
- `code-generation-plan` - produced in construction
- `code-quality-assessment` - produced in inception
- `code-structure` - produced in inception
- `code-summary` - produced in construction
- `competitive-analysis` - produced in ideation
- `component-inventory` - produced in inception
- `components` - produced in inception
- `constraint-register` - produced in ideation
- `contract-summary` - produced in inception
- `cost-analysis` - produced in operation
- `cross-unit-traceability` - produced in construction
- `dashboards` - produced in operation
- `decision-log` - produced in ideation
- `decisions` - produced in inception
- `delivery-planning-questions` - produced in inception
- `dependencies` - produced in inception
- `deployment-execution-questions` - produced in operation
- `deployment-log` - produced in operation
- `deployment-pipeline-questions` - produced in operation
- `deployment-strategy` - produced in operation
- `design-system-mapping` - produced in inception
- `discovered-rules` - produced in inception
- `drift-report` - produced in operation
- `entities` - produced in construction
- `environment-inventory` - produced in operation
- `environment-provisioning-questions` - produced in operation
- `escalation-matrix` - produced in operation
- `evidence` - produced in inception
- `external-dependency-map` - produced in inception
- `feasibility-assessment` - produced in ideation
- `feasibility-questions` - produced in ideation
- `feedback-loop` - produced in operation
- `feedback-optimization-questions` - produced in operation
- `frontend-components` - produced in construction (optional)
- `functional-spec` - produced in construction
- `health-check-report` - produced in operation
- `incident-plan` - produced in operation
- `incident-response-questions` - produced in operation
- `infrastructure-specification` - produced in construction
- `initiative-brief` - produced in ideation
- `integration-test-instructions` - produced in construction
- `intent-backlog` - produced in ideation
- `intent-capture-questions` - produced in ideation
- `intent-statement` - produced in ideation
- `interaction-spec` - produced in inception
- `load-test-plan` - produced in operation
- `load-test-results` - produced in operation
- `log-queries` - produced in operation
- `logical-components` - produced in construction
- `market-research-questions` - produced in ideation
- `market-trends` - produced in ideation
- `mob-composition` - produced in ideation
- `mockups` - produced in inception
- `monitoring-design` - produced in construction
- `nfr-validation-matrix` - produced in operation
- `observability-design` - produced in construction
- `observability-requirements` - produced in construction
- `observability-setup-questions` - produced in operation
- `performance-design` - produced in construction
- `performance-requirements` - produced in construction
- `performance-test-instructions` - produced in construction
- `performance-validation-questions` - produced in operation
- `personas` - produced in inception
- `practices-discovery-timestamp` - produced in inception
- `quality-gates` - produced in construction
- `raid-log` - produced in ideation
- `refined-mockups-questions` - produced in inception
- `reliability-design` - produced in construction
- `reliability-requirements` - produced in construction
- `requirements` - produced in inception
- `requirements-analysis-questions` - produced in inception
- `reverse-engineering-timestamp` - produced in inception
- `risk-and-sequencing-rationale` - produced in inception
- `rollback-runbook` - produced in operation
- `rough-mockups-questions` - produced in ideation
- `rules` - produced in construction
- `runbooks` - produced in operation
- `scalability-design` - produced in construction
- `scalability-requirements` - produced in construction
- `scope-definition-questions` - produced in ideation
- `scope-document` - produced in ideation
- `security-design` - produced in construction
- `security-requirements` - produced in construction
- `security-test-instructions` - produced in construction
- `skill-matrix` - produced in ideation
- `slo-config` - produced in operation
- `slo-report` - produced in operation
- `smoke-test-results` - produced in operation
- `stakeholder-map` - produced in ideation
- `stories` - produced in inception
- `team-allocation` - produced in inception
- `team-assessment` - produced in ideation
- `team-formation-questions` - produced in ideation
- `team-practices` - produced in inception
- `tech-stack-decisions` - produced in construction
- `technology-stack` - produced in inception
- `traceability` - produced in inception, construction
- `tracing-config` - produced in operation
- `unit-of-work` - produced in inception
- `unit-of-work-dependency` - produced in inception
- `unit-of-work-story-map` - produced in inception
- `unit-test-instructions` - produced in construction
- `user-flow` - produced in ideation
- `user-stories-assessment` - produced in inception
- `validation-report` - produced in operation
- `wireframes` - produced in ideation
