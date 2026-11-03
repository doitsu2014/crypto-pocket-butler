// core/tools/lib/orchestrate.mjs — the workflow engine: next, report, park,
// and status directives. This is the only component that advances state.

import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import {
  DEFAULT_SPACE,
  activeSpace,
  artifactDir,
  configPath,
  detectHarness,
  harnessDir,
  intentDir,
  intentsDir,
  memoryDir,
  spaceScopesDir,
} from "./paths.mjs";
import {
  appendAudit,
  isStageComplete,
  loadState,
  newIntentId,
  labelFromDescription,
  newState,
  saveState,
  stageRecord,
} from "./state.mjs";
import { detectScope, scopeByName, workflowForScope } from "./graph.mjs";
import { resolveMode, resolveQuestionBudget, resolveReviewRequired } from "./config.mjs";
import { engineRoot } from "./version.mjs";

function recordDirRel(space, intentId) {
  return `aidlc/spaces/${space}/intents/${intentId}`;
}

/**
 * Initialization stages are led by the engine's own conductor rather than by an
 * agent persona, so this slug has no agent file.
 */
const RESERVED_LEAD_AGENTS = new Set(["orchestrator"]);

function scopeForState(methodology, state, config) {
  const name = state.activeIntent?.scope || config.defaultScope || "classic";
  return (
    scopeByName(methodology, name) ||
    scopeByName(methodology, "classic") ||
    methodology.scopes[0]
  );
}

function currentWorkflow(methodology, state, config) {
  return workflowForScope(methodology, scopeForState(methodology, state, config));
}

/** Effective execution mode (scope > project > default). */
function executionMode(scope, config) {
  return resolveMode({ scope: scope ? scope.mode : null, config: config ? config.mode : null });
}

/**
 * A stage that requires review always presents a human gate, even under YOLO.
 * Set `review: required` in the stage frontmatter.
 */
export function stageReviewRequired(stage) {
  return stage?.review === "required";
}

function artifactPath(relDir, stage, artifact) {
  return `${relDir}/${stage.phase}/${stage.slug}/${artifact}.md`;
}

function producePathsFor(relDir, stage) {
  return stage.produces.map((artifact) => artifactPath(relDir, stage, artifact));
}

/** Artifacts the stage writes only when its condition selects that output. */
function optionalProducePathsFor(relDir, stage) {
  return (stage.optionalProduces || []).map((artifact) => artifactPath(relDir, stage, artifact));
}

function firstIncomplete(state, workflow) {
  return workflow.find((stage) => !isStageComplete(state, stage.slug)) || null;
}

/** The global phase-review toggle, resolved from scope > project config. */
function globalPhaseReview(methodology, state, config) {
  const scope = scopeForState(methodology, state, config);
  return resolveReviewRequired({
    scope: scope ? scope.reviewRequired : null,
    config: config ? config.reviewRequired : null,
  }).required;
}

/**
 * A phase gates before the workflow leaves it when its frontmatter says
 * `review: required`, or when the global review toggle is on.
 */
export function phaseReviewRequired(methodology, phaseSlug, globalRequired = false) {
  const phase = methodology.phases.find((p) => p.slug === phaseSlug);
  if (phase?.review === "required") return true;
  return globalRequired === true;
}

function stagesInPhase(workflow, phaseSlug) {
  return workflow.filter((stage) => stage.phase === phaseSlug);
}

function phaseReviewApproved(state, phaseSlug) {
  return state.phaseReviews?.[phaseSlug]?.status === "approved";
}

/** True when every applicable stage of the phase is done and it still needs a review. */
function phaseNeedsReview(state, workflow, methodology, phaseSlug, globalRequired) {
  if (!phaseReviewRequired(methodology, phaseSlug, globalRequired)) return false;
  if (phaseReviewApproved(state, phaseSlug)) return false;
  const stages = stagesInPhase(workflow, phaseSlug);
  if (stages.length === 0) return false;
  return stages.every((stage) => isStageComplete(state, stage.slug));
}

/**
 * Move to the next incomplete stage. When the finished stage closes a phase
 * that requires review, pause on a phase-review gate instead of crossing the
 * phase boundary.
 */
function advance(state, workflow, methodology, globalRequired = false) {
  const index = workflow.findIndex((stage) => stage.slug === state.currentStage);
  const finished = index >= 0 ? workflow[index] : null;
  for (let i = index + 1; i < workflow.length; i += 1) {
    const stage = workflow[i];
    if (isStageComplete(state, stage.slug)) continue;
    if (
      finished &&
      finished.phase !== stage.phase &&
      phaseNeedsReview(state, workflow, methodology, finished.phase, globalRequired)
    ) {
      state.pendingPhaseReview = finished.phase;
      return state.currentStage;
    }
    state.currentStage = stage.slug;
    return state.currentStage;
  }
  if (finished && phaseNeedsReview(state, workflow, methodology, finished.phase, globalRequired)) {
    state.pendingPhaseReview = finished.phase;
    return state.currentStage;
  }
  state.currentStage = null;
  return null;
}

function scaffoldWorkspace(root, methodology) {
  const space = activeSpace(root);
  mkdirSync(memoryDir(root, space), { recursive: true });
  const coreRoot = engineRoot();
  const memorySource = join(coreRoot, "memory");
  if (existsSync(memorySource)) {
    for (const file of readdirSync(memorySource)) {
      if (!file.endsWith(".md")) continue;
      const target = join(memoryDir(root, space), file);
      if (!existsSync(target)) copyFileSync(join(memorySource, file), target);
    }
  }
  // Seed the team's workflow scopes into this space. They layer over the native
  // baseline, and an existing file is never overwritten.
  const scopeSource = join(coreRoot, "templates", "scopes");
  if (existsSync(scopeSource)) {
    const target = spaceScopesDir(root, space);
    mkdirSync(target, { recursive: true });
    for (const file of readdirSync(scopeSource)) {
      if (!file.endsWith(".md")) continue;
      const dest = join(target, file);
      if (!existsSync(dest)) copyFileSync(join(scopeSource, file), dest);
    }
  }
  if (!existsSync(configPath(root))) {
    mkdirSync(join(root, "aidlc"), { recursive: true });
    saveProjectConfig(root, {
      harness: detectHarness(root),
      defaultScope: "classic",
    });
  }
}

function saveProjectConfig(root, patch) {
  const path = configPath(root);
  let current = {};
  try {
    current = JSON.parse(readFileSync(path, "utf8"));
  } catch {
    current = {};
  }
  const next = { ...current, ...patch };
  mkdirSync(join(root, "aidlc"), { recursive: true });
  writeFileSync(path, `${JSON.stringify(next, null, 2)}\n`, "utf8");
}

export function buildRunDirective(root, methodology, config, state, stage) {
  const phase = methodology.phases.find((p) => p.slug === stage.phase);
  const scope = scopeForState(methodology, state, config);
  const workflow = currentWorkflow(methodology, state, config);
  const harness = detectHarness(root);
  const dir = harnessDir(root, harness);
  const space = state.activeIntent.space || DEFAULT_SPACE;
  const intentId = state.activeIntent.id;
  const relDir = recordDirRel(space, intentId);

  const producePaths = producePathsFor(relDir, stage);

  const consumes = [];
  const consumesAbsent = [];
  for (const entry of stage.consumes) {
    const producing = methodology.stages.find((s) => s.produces.includes(entry.artifact));
    let path = null;
    if (producing) {
      path = `${relDir}/${producing.phase}/${producing.slug}/${entry.artifact}.md`;
    }
    if (path && existsSync(join(root, path))) {
      consumes.push({ artifact: entry.artifact, path, required: entry.required });
    } else {
      consumesAbsent.push({
        artifact: entry.artifact,
        required: entry.required,
        expected: producing ? !workflow.some((s) => s.slug === producing.slug) : false,
      });
    }
  }

  const protocolModules = ["stage-protocol"];
  const mode = executionMode(scope, config);
  const yolo = mode.mode === "yolo";
  const reviewRequired = stageReviewRequired(stage);
  const questionBudget = yolo
    ? { min: 0, max: 0, source: "mode" }
    : resolveQuestionBudget({
        stage: stage.questionBudget,
        scope: scope.questionBudget,
        config: config ? config.questionBudget : null,
      });
  if (questionBudget.max > 0) protocolModules.push("question-flow");
  if (scope.learnings === "on") protocolModules.push("learnings");

  const stageIndex = workflow.findIndex((s) => s.slug === stage.slug) + 1;

  return {
    kind: "run-stage",
    stage: stage.slug,
    stage_name: stage.name,
    phase: stage.phase,
    phase_name: phase ? phase.name : stage.phase,
    scope: scope.name,
    stage_file: `${dir}/native/stages/${stage.phase}/${stage.slug}.md`,
    lead_agent: stage.leadAgent,
    lead_agent_file:
      stage.leadAgent && !RESERVED_LEAD_AGENTS.has(stage.leadAgent)
        ? `${dir}/native/agents/${stage.leadAgent}.md`
        : null,
    support_agents: stage.supportAgents,
    support_agent_files: stage.supportAgents.map((agent) => `${dir}/native/agents/${agent}.md`),
    mode: stage.mode,
    execution_mode: mode.mode,
    mode_source: mode.source,
    auto_approve: yolo && !reviewRequired,
    answer_policy: yolo ? "recommended" : "human",
    review: stage.review,
    review_required: reviewRequired,
    reviewer: stage.reviewer,
    reviewer_file: stage.reviewer ? `${dir}/native/agents/${stage.reviewer}.md` : null,
    gate: true,
    workspace_requires: stage.workspaceRequires,
    question_budget: questionBudget,
    produces: stage.produces,
    produce_paths: producePaths,
    optional_produces: stage.optionalProduces || [],
    optional_produce_paths: optionalProducePathsFor(relDir, stage),
    consumes,
    consumes_absent: consumesAbsent,
    memory_path: scope.learnings === "on"
      ? `${relDir}/${stage.phase}/${stage.slug}/memory.md`
      : null,
    protocol_modules: protocolModules,
    record_dir: relDir,
    narration: `Running ${phase ? phase.name : stage.phase} / ${stage.name}.`,
    workflow: { scope: scope.name, stage_index: stageIndex, stage_total: workflow.length },
  };
}

function gateDirective(root, methodology, config, state, stage) {
  const reviewRequired = stageReviewRequired(stage);
  const relDir = recordDirRel(
    state.activeIntent.space || DEFAULT_SPACE,
    state.activeIntent.id,
  );
  return {
    kind: "ask",
    ask_type: "stage-approval",
    stage: stage.slug,
    stage_name: stage.name,
    phase: stage.phase,
    scope: state.activeIntent.scope,
    question: reviewRequired
      ? `Review and approve ${stage.name}?`
      : `Approve ${stage.name}?`,
    options: ["Approve", "Request Changes"],
    route: "report",
    review: stage.review,
    review_required: reviewRequired,
    produces: stage.produces,
    produce_paths: producePathsFor(relDir, stage),
    record_dir: relDir,
  };
}

function phaseGateDirective(root, methodology, config, state, workflow, phaseSlug) {
  const phase = methodology.phases.find((p) => p.slug === phaseSlug);
  const space = state.activeIntent.space || DEFAULT_SPACE;
  const relDir = recordDirRel(space, state.activeIntent.id);
  const stages = stagesInPhase(workflow, phaseSlug);

  state.phaseReviews = state.phaseReviews || {};
  const entry = state.phaseReviews[phaseSlug] || (state.phaseReviews[phaseSlug] = { status: "pending" });
  if (!entry.requested) {
    entry.requested = true;
    entry.updatedAt = new Date().toISOString();
    appendAudit(root, {
      event: "PHASE_AWAITING_APPROVAL",
      space,
      intent: state.activeIntent.id,
      phase: phaseSlug,
    });
    saveState(root, state);
  }

  return {
    kind: "ask",
    ask_type: "phase-review",
    phase: phaseSlug,
    phase_name: phase ? phase.name : phaseSlug,
    scope: state.activeIntent.scope,
    question: `Review the ${phase ? phase.name : phaseSlug} phase?`,
    options: ["Approve phase", "Request Changes"],
    route: "report-phase",
    stages: stages.map((stage) => stage.slug),
    produces: stages.flatMap((stage) => stage.produces),
    produce_paths: stages.flatMap((stage) => producePathsFor(relDir, stage)),
    record_dir: relDir,
  };
}

export function currentDirective(root, methodology, config, state) {
  const workflow = currentWorkflow(methodology, state, config);
  const globalRequired = globalPhaseReview(methodology, state, config);
  if (state.pendingPhaseReview) {
    const phaseSlug = state.pendingPhaseReview;
    if (phaseNeedsReview(state, workflow, methodology, phaseSlug, globalRequired)) {
      return phaseGateDirective(root, methodology, config, state, workflow, phaseSlug);
    }
    state.pendingPhaseReview = null;
    advance(state, workflow, methodology, globalRequired);
    saveState(root, state);
  }
  if (!state.currentStage) {
    const next = firstIncomplete(state, workflow);
    if (!next) return doneDirective(state, workflow);
    state.currentStage = next.slug;
  }
  const stage = workflow.find((s) => s.slug === state.currentStage);
  if (!stage) {
    const next = firstIncomplete(state, workflow);
    if (!next) return doneDirective(state, workflow);
    state.currentStage = next.slug;
    return currentDirective(root, methodology, config, state);
  }
  const record = stageRecord(state, stage.slug);
  if (record.status === "awaiting-approval") {
    const mode = executionMode(scopeForState(methodology, state, config), config);
    if (mode.mode === "yolo" && !stageReviewRequired(stage)) {
      record.status = "complete";
      record.autoApproved = true;
      record.updatedAt = new Date().toISOString();
      appendAudit(root, {
        event: "STAGE_AUTO_APPROVED",
        space: state.activeIntent.space || DEFAULT_SPACE,
        intent: state.activeIntent.id,
        phase: stage.phase,
        stage: stage.slug,
        reason: "yolo mode: gate auto-satisfied with the recommended answer",
      });
      advance(state, workflow, methodology, globalRequired);
      saveState(root, state);
      return currentDirective(root, methodology, config, state);
    }
    return gateDirective(root, methodology, config, state, stage);
  }
  if (record.status === "pending") {
    record.status = "active";
    record.attempt = (record.attempt || 0) + 1;
    record.updatedAt = new Date().toISOString();
    appendAudit(root, {
      event: "STAGE_STARTED",
      space: state.activeIntent.space || DEFAULT_SPACE,
      intent: state.activeIntent.id,
      phase: stage.phase,
      stage: stage.slug,
    });
    saveState(root, state);
  }
  return buildRunDirective(root, methodology, config, state, stage);
}

function doneDirective(state, workflow) {
  return {
    kind: "done",
    message: `Workflow complete for intent "${state.activeIntent.id}" (${state.activeIntent.scope}).`,
    summary: {
      intent: state.activeIntent.id,
      scope: state.activeIntent.scope,
      stages: workflow.map((stage) => ({
        slug: stage.slug,
        name: stage.name,
        phase: stage.phase,
        status: state.stages[stage.slug]?.status || "pending",
      })),
    },
  };
}

export function parkDirective(root, state) {
  state.parked = true;
  saveState(root, state);
  appendAudit(root, {
    event: "WORKFLOW_PARKED",
    space: state.activeIntent.space || DEFAULT_SPACE,
    intent: state.activeIntent.id,
    stage: state.currentStage,
  });
  return {
    kind: "parked",
    stage: state.currentStage,
    message: `Workflow parked at ${state.currentStage}. Resume with "my-aidlc orchestrate next --resume".`,
  };
}

/** Handle `orchestrate next`. */
export function runNext(root, methodology, config, options = {}) {
  const state = loadState(root);
  const text = options.text || "";
  const forceNew = options.newIntent === true;

  if (!state || forceNew) {
    if (!text) {
      return {
        kind: "error",
        message:
          "No active workflow. Describe what you want to build, for example: my-aidlc orchestrate next \"Build a REST API for inventory\".",
      };
    }
    scaffoldWorkspace(root, methodology);
    const scopeName = options.scope || detectScope(methodology, text, config.defaultScope);
    const scope = scopeByName(methodology, scopeName) || scopeByName(methodology, "classic");
    const existing = existsSync(intentsDir(root, DEFAULT_SPACE))
      ? readdirSync(intentsDir(root, DEFAULT_SPACE))
      : [];
    const label = labelFromDescription(text);
    const id = newIntentId(label, existing);
    const intent = {
      id,
      label,
      description: text,
      scope: scope.name,
      space: DEFAULT_SPACE,
      createdAt: new Date().toISOString(),
    };
    mkdirSync(intentDir(root, intent.id, DEFAULT_SPACE), { recursive: true });
    const fresh = newState({ intent, scope: scope.name });
    appendAudit(root, {
      event: "INTENT_CREATED",
      space: DEFAULT_SPACE,
      intent: id,
      scope: scope.name,
    });
    appendAudit(root, {
      event: "SCOPE_SELECTED",
      space: DEFAULT_SPACE,
      intent: id,
      scope: scope.name,
    });
    saveState(root, fresh);
    return currentDirective(root, methodology, config, fresh);
  }

  if (state.parked && !options.resume) {
    return {
      kind: "parked",
      stage: state.currentStage,
      message: `Workflow parked at ${state.currentStage}. Resume with "my-aidlc orchestrate next --resume".`,
    };
  }
  if (state.parked && options.resume) {
    state.parked = false;
    saveState(root, state);
  }

  return currentDirective(root, methodology, config, state);
}

/** Handle a phase-level review decision (`orchestrate report --phase`). */
function runPhaseReport(root, methodology, config, state, options) {
  const phaseSlug = options.phase;
  const phase = methodology.phases.find((p) => p.slug === phaseSlug);
  if (!phase) {
    return { kind: "error", message: `Unknown phase "${phaseSlug}".` };
  }
  const workflow = currentWorkflow(methodology, state, config);
  const globalRequired = globalPhaseReview(methodology, state, config);
  const stages = stagesInPhase(workflow, phaseSlug);
  if (stages.length === 0) {
    return {
      kind: "error",
      message: `Phase "${phaseSlug}" is not in the "${state.activeIntent.scope}" workflow.`,
    };
  }
  if (state.pendingPhaseReview !== phaseSlug) {
    return { kind: "error", message: `Phase "${phaseSlug}" is not awaiting review.` };
  }

  const result = options.result;
  const space = state.activeIntent.space || DEFAULT_SPACE;
  const baseAudit = {
    space,
    intent: state.activeIntent.id,
    phase: phaseSlug,
    result,
    userInput: options.userInput || null,
    reason: options.reason || null,
  };

  state.phaseReviews = state.phaseReviews || {};
  const entry =
    state.phaseReviews[phaseSlug] ||
    (state.phaseReviews[phaseSlug] = { status: "pending" });

  if (result === "approved") {
    entry.status = "approved";
    entry.requested = false;
    entry.updatedAt = new Date().toISOString();
    state.pendingPhaseReview = null;
    appendAudit(root, { ...baseAudit, event: "PHASE_APPROVED" });
    advance(state, workflow, methodology, globalRequired);
    saveState(root, state);
    const directive = currentDirective(root, methodology, config, state);
    if (directive.kind === "done") {
      appendAudit(root, { event: "WORKFLOW_COMPLETED", space, intent: state.activeIntent.id });
    }
    return directive;
  }

  if (result === "rejected") {
    entry.status = "pending";
    entry.requested = false;
    entry.feedback = entry.feedback || [];
    entry.feedback.push({ at: new Date().toISOString(), reason: options.reason || "" });
    entry.updatedAt = new Date().toISOString();
    state.pendingPhaseReview = null;
    // Re-open the phase so the conductor redoes it with the feedback, then gates again.
    for (const stage of stages) {
      const record = stageRecord(state, stage.slug);
      record.status = "pending";
      record.autoApproved = false;
      record.updatedAt = new Date().toISOString();
    }
    state.currentStage = stages[0].slug;
    appendAudit(root, { ...baseAudit, event: "PHASE_REJECTED" });
    saveState(root, state);
    return currentDirective(root, methodology, config, state);
  }

  return {
    kind: "error",
    message: `Unknown phase report result "${result}". Valid: approved, rejected.`,
  };
}

/** Handle `orchestrate report`. */
export function runReport(root, methodology, config, options = {}) {
  const state = loadState(root);
  if (!state) {
    return { kind: "error", message: "No active workflow to report against." };
  }
  if (options.phase) {
    return runPhaseReport(root, methodology, config, state, options);
  }
  const slug = options.stage || state.currentStage;
  const stage = methodology.stages.find((s) => s.slug === slug);
  if (!stage) {
    return { kind: "error", message: `Unknown stage "${slug}".` };
  }
  const workflow = currentWorkflow(methodology, state, config);
  const globalRequired = globalPhaseReview(methodology, state, config);
  if (!workflow.some((s) => s.slug === slug)) {
    return { kind: "error", message: `Stage "${slug}" is not in the "${state.activeIntent.scope}" workflow.` };
  }
  const record = stageRecord(state, slug);
  const result = options.result;
  const space = state.activeIntent.space || DEFAULT_SPACE;
  const baseAudit = {
    space,
    intent: state.activeIntent.id,
    phase: stage.phase,
    stage: slug,
    result,
    userInput: options.userInput || null,
    reason: options.reason || null,
  };

  switch (result) {
    case "in-progress":
      record.status = "active";
      break;
    case "awaiting-approval":
      record.status = "awaiting-approval";
      appendAudit(root, { ...baseAudit, event: "STAGE_AWAITING_APPROVAL" });
      break;
    case "approved":
      record.status = "complete";
      appendAudit(root, { ...baseAudit, event: "STAGE_APPROVED" });
      advance(state, workflow, methodology, globalRequired);
      break;
    case "completed":
      record.status = "complete";
      appendAudit(root, { ...baseAudit, event: "STAGE_COMPLETED" });
      advance(state, workflow, methodology, globalRequired);
      break;
    case "rejected":
      record.status = "active";
      record.feedback = record.feedback || [];
      record.feedback.push({ at: new Date().toISOString(), reason: options.reason || "" });
      appendAudit(root, { ...baseAudit, event: "STAGE_REJECTED" });
      break;
    case "revised":
      record.status = "awaiting-approval";
      appendAudit(root, { ...baseAudit, event: "STAGE_REVISED" });
      break;
    case "skipped":
      if (!options.reason) {
        return { kind: "error", message: `Reporting "skipped" for ${slug} requires a --reason.` };
      }
      record.status = "skipped";
      appendAudit(root, { ...baseAudit, event: "STAGE_SKIPPED" });
      advance(state, workflow, methodology, globalRequired);
      break;
    default:
      return {
        kind: "error",
        message: `Unknown report result "${result}". Valid: in-progress, awaiting-approval, approved, rejected, revised, completed, skipped.`,
      };
  }

  record.updatedAt = new Date().toISOString();
  saveState(root, state);

  const directive = currentDirective(root, methodology, config, state);
  if (directive.kind === "done") {
    appendAudit(root, { event: "WORKFLOW_COMPLETED", space, intent: state.activeIntent.id });
  }
  return directive;
}

export function statusReport(root, methodology, config) {
  const state = loadState(root);
  if (!state) {
    return { active: false, message: "No active workflow." };
  }
  const workflow = currentWorkflow(methodology, state, config);
  const globalRequired = globalPhaseReview(methodology, state, config);
  return {
    active: true,
    parked: state.parked,
    mode: executionMode(scopeForState(methodology, state, config), config).mode,
    intent: state.activeIntent,
    currentStage: state.currentStage,
    pendingPhaseReview: state.pendingPhaseReview || null,
    scope: state.activeIntent.scope,
    phaseReviews: [...new Set(workflow.map((stage) => stage.phase))].map((phaseSlug) => ({
      phase: phaseSlug,
      reviewRequired: phaseReviewRequired(methodology, phaseSlug, globalRequired),
      status: state.phaseReviews?.[phaseSlug]?.status || "pending",
    })),
    stages: workflow.map((stage) => ({
      slug: stage.slug,
      name: stage.name,
      phase: stage.phase,
      status: state.stages[stage.slug]?.status || "pending",
      autoApproved: state.stages[stage.slug]?.autoApproved === true,
      reviewRequired: stageReviewRequired(stage),
    })),
  };
}
