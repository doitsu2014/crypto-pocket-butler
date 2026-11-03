// core/tools/lib/graph.mjs — loads the methodology and derives the workflow
// order for a scope.
//
// The methodology is not authored here. It is the native AWS AI-DLC snapshot
// that `my-aidlc update` writes into `core/native/` (see core/tools/lib/native.mjs
// and aidlc-version.json):
//
//   native/phases/<phase>.md            my-aidlc's display + gate metadata
//   native/stages/<phase>/<slug>.md     the native stage definitions
//   native/agents/<slug>.md             the native agent personas
//   native/scopes/<name>.md             the native baseline workflow scopes
//
// Workflow scopes the team owns live in the workspace, one file per scope:
//
//   aidlc/spaces/<space>/scopes/<name>.md
//
// A space scope with the same name replaces the native baseline of that name;
// a new name adds a profile. Space scopes are where `include`/`skip` are
// authored, and those lists win over a stage's own `scopes:` declaration.

import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { parseFrontmatter } from "./frontmatter.mjs";

function readFrontmatterFile(path) {
  const text = readFileSync(path, "utf8");
  const { data, body, hasFrontmatter } = parseFrontmatter(text);
  return { data, body, hasFrontmatter };
}

function listDirs(path) {
  if (!existsSync(path)) return [];
  return readdirSync(path, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort();
}

function listMarkdown(path) {
  if (!existsSync(path)) return [];
  return readdirSync(path, { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith(".md"))
    .map((entry) => entry.name)
    .sort();
}

export function titleCase(slug) {
  return slug
    .split("-")
    .map((part) => (part.length ? part[0].toUpperCase() + part.slice(1) : part))
    .join(" ");
}

function asArray(value) {
  if (value === null || value === undefined) return [];
  return Array.isArray(value) ? value : [value];
}

/** Root of the fetched native methodology inside the engine directory. */
export function nativeRoot(coreRoot) {
  return join(coreRoot, "native");
}

function readScopeFile(path, source) {
  const { data, body } = readFrontmatterFile(path);
  const fileName = path.split("/").pop().replace(/\.md$/, "");
  return {
    name: data.name || fileName.replace(/^aidlc-/, ""),
    depth: data.depth || "Standard",
    keywords: asArray(data.keywords).map((k) => String(k).toLowerCase()),
    description: data.description || "",
    skeleton: data.skeleton || "off",
    reviewCap: data.review_cap || "advisory",
    guardPolicy: data.guard_policy || "relaxed",
    sensors: data.sensors || "on",
    learnings: data.learnings || "on",
    summaryConfirmation: data.summary_confirmation || "off",
    mode: typeof data.mode === "string" ? data.mode.trim().toLowerCase() : null,
    reviewRequired: data.review_required ?? null,
    questionBudget: data.question_budget ?? null,
    phases: asArray(data.phases),
    include: asArray(data.include),
    skip: asArray(data.skip),
    source,
    file: path,
    body,
  };
}

/**
 * Load phases, stages, agents, and scopes.
 *
 * @param {string} coreRoot Engine directory (e.g. `<repo>/core` or `<project>/.pi`).
 * @param {{ spaceScopesDir?: string|null }} [options]
 *   Absolute path to the active space's `scopes/` directory, if any.
 */
export function loadMethodology(coreRoot, options = {}) {
  const native = nativeRoot(coreRoot);
  const phasesRoot = join(native, "phases");
  const stagesRoot = join(native, "stages");
  const agentsRoot = join(native, "agents");
  const scopesRoot = join(native, "scopes");

  // The methodology is fetched, so a missing tree is a real, fixable state
  // rather than an empty workflow. Fail loudly with the way out.
  if (!existsSync(stagesRoot) || !existsSync(phasesRoot)) {
    throw new Error(
      `No native AI-DLC methodology at ${native}. ` +
        'From a my-aidlc checkout run "my-aidlc update", then re-project this project with ' +
        '"my-aidlc config --harness <name>".',
    );
  }

  const phases = [];
  for (const file of listMarkdown(phasesRoot)) {
    const path = join(phasesRoot, file);
    const { data } = readFrontmatterFile(path);
    const slug = data.slug || file.replace(/\.md$/, "");
    phases.push({
      slug,
      name: data.name || titleCase(slug),
      order: typeof data.order === "number" ? data.order : phases.length + 1,
      focus: data.focus || "",
      aiRole: data.ai_role || "",
      output: data.output || "",
      description: data.description || "",
      keyActivities: asArray(data.key_activities),
      examplePrompts: asArray(data.example_prompts),
      review:
        data.review === undefined || data.review === null
          ? "auto"
          : String(data.review).trim().toLowerCase(),
      file: path,
    });
  }
  phases.sort((a, b) => a.order - b.order || a.slug.localeCompare(b.slug));

  const stages = [];
  for (const phaseSlug of listDirs(stagesRoot)) {
    const dir = join(stagesRoot, phaseSlug);
    for (const file of listMarkdown(dir)) {
      const path = join(dir, file);
      const { data, body } = readFrontmatterFile(path);
      const slug = data.slug || file.replace(/\.md$/, "");
      if (data.phase && data.phase !== phaseSlug) {
        throw new Error(
          `Stage ${path} declares phase "${data.phase}" but lives under "${phaseSlug}".`,
        );
      }
      stages.push({
        slug,
        name: data.name || titleCase(slug),
        phase: data.phase || phaseSlug,
        execution: (data.execution || "ALWAYS").toUpperCase(),
        condition: data.condition || "",
        leadAgent: data.lead_agent || null,
        supportAgents: asArray(data.support_agents),
        mode: data.mode || "inline",
        reviewer: data.reviewer || null,
        reviewClass: data.review_class || (data.reviewer ? "adversarial" : null),
        review:
          data.review === undefined || data.review === null
            ? "auto"
            : String(data.review).trim().toLowerCase(),
        reviewArtifact: data.review_artifact || null,
        forEach: data.for_each || null,
        workspaceRequires: data.workspace_requires === true,
        questionBudget: data.question_budget ?? null,
        produces: asArray(data.produces),
        // `optional_produces` is native's way of naming an artifact a stage
        // writes only when its condition selects that output (for example
        // `frontend-components`, written only for a `ui` unit).
        optionalProduces: asArray(data.optional_produces),
        consumes: asArray(data.consumes).map((entry) => ({
          artifact: entry.artifact,
          required: entry.required !== false,
          conditionalOn: entry.conditional_on || null,
        })),
        requiresStage: asArray(data.requires_stage),
        scopes: asArray(data.scopes),
        inputs: data.inputs || "",
        outputs: data.outputs || "",
        file: path,
        body,
      });
    }
  }
  stages.sort((a, b) => a.slug.localeCompare(b.slug));

  // Baseline scopes come from the pinned release; space scopes layer over them.
  const byName = new Map();
  for (const file of listMarkdown(scopesRoot)) {
    const scope = readScopeFile(join(scopesRoot, file), "native");
    byName.set(scope.name, scope);
  }
  if (options.spaceScopesDir && existsSync(options.spaceScopesDir)) {
    for (const file of listMarkdown(options.spaceScopesDir)) {
      const scope = readScopeFile(join(options.spaceScopesDir, file), "space");
      byName.set(scope.name, scope);
    }
  }
  const scopes = [...byName.values()].sort((a, b) => a.name.localeCompare(b.name));

  const agents = [];
  for (const file of listMarkdown(agentsRoot)) {
    const path = join(agentsRoot, file);
    const { data, body } = readFrontmatterFile(path);
    agents.push({
      slug: file.replace(/\.md$/, ""),
      name: data.name || file.replace(/\.md$/, ""),
      displayName: data.display_name || titleCase(file.replace(/\.md$/, "")),
      description: (data.description || "").toString().trim(),
      tier: data.tier || "balanced",
      file: path,
      body,
    });
  }
  agents.sort((a, b) => a.slug.localeCompare(b.slug));

  return { phases, stages, scopes, agents };
}

export function scopeByName(methodology, name) {
  return methodology.scopes.find((scope) => scope.name === name) || null;
}

/** Scopes the team authored in a space, as opposed to the native baseline. */
export function spaceScopes(methodology) {
  return methodology.scopes.filter((scope) => scope.source === "space");
}

/**
 * Does a stage execute under a scope?
 *
 * Precedence, most specific first:
 *   1. a non-empty `include` list is authoritative: the stage runs if and only
 *      if its slug is listed. This is how a space scope states its complete
 *      workflow, and it is why a space scope can re-map the native grid
 *      without touching a single stage file.
 *   2. the scope's `skip` list (an explicit SKIP).
 *   3. the stage's own `scopes:` declaration — the native baseline grid.
 *   4. the scope's `phases` list.
 *
 * The native baseline scopes under `core/native/scopes/` declare no `include`,
 * so they resolve through rule 3 and reproduce the published AI-DLC grids.
 */
export function stageRunsInScope(stage, scope) {
  if (scope.include.length > 0) return scope.include.includes(stage.slug);
  if (scope.skip.includes(stage.slug)) return false;
  if (stage.scopes.length > 0) return stage.scopes.includes(scope.name);
  return scope.phases.includes(stage.phase);
}

/** The ordered, applicable stage list for a scope (topological by requires_stage). */
export function workflowForScope(methodology, scope) {
  const phaseOrder = new Map(methodology.phases.map((phase) => [phase.slug, phase.order]));
  const applicable = methodology.stages.filter((stage) => stageRunsInScope(stage, scope));
  const bySlug = new Map(applicable.map((stage) => [stage.slug, stage]));

  const dependencies = new Map();
  for (const stage of applicable) {
    const deps = stage.requiresStage.filter((slug) => bySlug.has(slug));
    dependencies.set(stage.slug, deps);
  }

  const indegree = new Map();
  for (const stage of applicable) indegree.set(stage.slug, dependencies.get(stage.slug).length);
  const dependents = new Map();
  for (const stage of applicable) {
    for (const dep of dependencies.get(stage.slug)) {
      if (!dependents.has(dep)) dependents.set(dep, []);
      dependents.get(dep).push(stage.slug);
    }
  }

  const ready = applicable
    .filter((stage) => indegree.get(stage.slug) === 0)
    .map((stage) => stage.slug);
  const sortReady = (slugs) =>
    slugs.sort((a, b) => {
      const sa = bySlug.get(a);
      const sb = bySlug.get(b);
      const pa = phaseOrder.get(sa.phase) ?? 0;
      const pb = phaseOrder.get(sb.phase) ?? 0;
      if (pa !== pb) return pa - pb;
      return a.localeCompare(b);
    });

  const ordered = [];
  while (ready.length > 0) {
    sortReady(ready);
    const slug = ready.shift();
    ordered.push(bySlug.get(slug));
    for (const dependent of dependents.get(slug) || []) {
      indegree.set(dependent, indegree.get(dependent) - 1);
      if (indegree.get(dependent) === 0) ready.push(dependent);
    }
  }

  if (ordered.length !== applicable.length) {
    const missing = applicable
      .map((stage) => stage.slug)
      .filter((slug) => !ordered.some((stage) => stage.slug === slug));
    throw new Error(`Cycle detected in workflow for scope "${scope.name}": ${missing.join(", ")}`);
  }
  return ordered;
}

/** Auto-detect a scope from a freeform description. Longest keyword wins. */
export function detectScope(methodology, description, fallback = "classic") {
  if (!description) return fallback;
  const text = ` ${description.toLowerCase()} `;
  let best = null;
  let bestLength = 0;
  for (const scope of methodology.scopes) {
    for (const keyword of scope.keywords) {
      if (!keyword) continue;
      const pattern = new RegExp(`(^|[^a-z0-9])${escapeRegExp(keyword)}([^a-z0-9]|$)`, "i");
      if (pattern.test(text) && keyword.length > bestLength) {
        best = scope.name;
        bestLength = keyword.length;
      }
    }
  }
  return best || fallback;
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/** The pin and provenance recorded by the last `my-aidlc update`. */
export function readNativeLock(coreRoot) {
  const path = join(nativeRoot(coreRoot), "lock.json");
  if (!existsSync(path)) return null;
  try {
    const lock = JSON.parse(readFileSync(path, "utf8"));
    return {
      repository: lock.repository,
      version: lock.version,
      tag: lock.tag,
      commit: lock.commit || null,
      archiveVerified: lock.archiveVerified === true,
      stages: lock.stages,
      agents: lock.agents,
      scopes: lock.scopes,
      protocols: lock.protocols,
    };
  } catch {
    return null;
  }
}

/** Compile the method into a serializable graph for `package` output. */
export function compileGraph(coreRoot, options = {}) {
  const methodology = loadMethodology(coreRoot, options);
  const scopeGrid = {};
  for (const scope of methodology.scopes) {
    scopeGrid[scope.name] = workflowForScope(methodology, scope).map((stage) => stage.slug);
  }
  return {
    native: readNativeLock(coreRoot),
    phases: methodology.phases.map((phase) => ({
      slug: phase.slug,
      name: phase.name,
      order: phase.order,
      focus: phase.focus,
      aiRole: phase.aiRole,
      output: phase.output,
      review: phase.review,
    })),
    stages: methodology.stages.map((stage) => ({
      slug: stage.slug,
      name: stage.name,
      phase: stage.phase,
      execution: stage.execution,
      leadAgent: stage.leadAgent,
      supportAgents: stage.supportAgents,
      mode: stage.mode,
      reviewer: stage.reviewer,
      review: stage.review,
      produces: stage.produces,
      optionalProduces: stage.optionalProduces,
      consumes: stage.consumes,
      requiresStage: stage.requiresStage,
    })),
    scopes: methodology.scopes.map((scope) => ({
      name: scope.name,
      source: scope.source,
      depth: scope.depth,
      keywords: scope.keywords,
      phases: scope.phases,
      include: scope.include,
      skip: scope.skip,
    })),
    scopeGrid,
  };
}
