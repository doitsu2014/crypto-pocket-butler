// core/tools/lib/doctor.mjs — validates the engine, workspace, and config.

import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { loadMethodology, workflowForScope, scopeByName, readNativeLock, nativeRoot } from "./graph.mjs";
import { loadConfig } from "./config.mjs";
import { loadState } from "./state.mjs";
import { detectHarness, harnessDir, memoryDir, spaceScopesDir, workspaceRoot } from "./paths.mjs";
import { engineRoot, readVersion } from "./version.mjs";

const REQUIRED_MEMORY = ["org.md", "team.md", "project.md"];

export function runDoctor(root) {
  const checks = [];
  const add = (status, message) => checks.push({ status, message });

  const major = Number.parseInt(process.versions.node.split(".")[0], 10);
  if (major >= 20) add("pass", `Node.js ${process.versions.node}`);
  else add("fail", `Node.js ${process.versions.node} found; version 20 or newer is required.`);

  add("pass", `my-aidlc ${readVersion()} engine at ${engineRoot()}`);

  const native = readNativeLock(engineRoot());
  if (native) {
    add(
      native.archiveVerified ? "pass" : "warn",
      `Native AI-DLC ${native.version} (${native.repository} ${native.tag}${native.commit ? `, ${native.commit.slice(0, 12)}` : ""}) — ` +
        `${native.stages} stages, ${native.agents} agents, ${native.scopes} baseline scopes` +
        `${native.archiveVerified ? "" : "; snapshot content-verified, archive bytes differ from the pin"}`,
    );
  } else {
    add(
      "fail",
      `No native methodology at ${nativeRoot(engineRoot())}. Run "my-aidlc update" (in a checkout) or re-run "my-aidlc config --harness <name>".`,
    );
  }

  let methodology = null;
  try {
    methodology = loadMethodology(engineRoot(), { spaceScopesDir: spaceScopesDir(root) });
    const ok =
      methodology.phases.length > 0 &&
      methodology.stages.length > 0 &&
      methodology.scopes.length > 0 &&
      methodology.agents.length > 0;
    add(
      ok ? "pass" : "fail",
      `Methodology: ${methodology.phases.length} phases, ${methodology.stages.length} stages, ` +
        `${methodology.scopes.length} scopes, ${methodology.agents.length} agents`,
    );
    const space = methodology.scopes.filter((scope) => scope.source === "space");
    const scopesDir = spaceScopesDir(root);
    if (space.length === 0) {
      add(
        "warn",
        `No space scopes in ${scopesDir}; running on the native baseline. Run "my-aidlc init" to seed yours.`,
      );
    } else {
      add("pass", `Space scopes: ${space.map((scope) => scope.name).join(", ")}`);
    }
    for (const scope of methodology.scopes) {
      if (scope.include.length === 0) continue;
      const unknown = scope.include.filter(
        (slug) => !methodology.stages.some((stage) => stage.slug === slug),
      );
      if (unknown.length) {
        add("fail", `Scope ${scope.name} includes unknown stage(s): ${unknown.join(", ")}`);
      }
    }
  } catch (error) {
    add("fail", `Methodology failed to load: ${error.message}`);
  }

  if (methodology) {
    // Initialization stages are led by the engine's own conductor, not by an
    // agent persona, so `orchestrator` is a reserved lead rather than a file.
    const reservedLeads = new Set(["orchestrator"]);
    const missingAgents = [];
    for (const stage of methodology.stages) {
      const refs = [stage.leadAgent, ...stage.supportAgents, stage.reviewer].filter(Boolean);
      for (const ref of refs) {
        if (reservedLeads.has(ref)) continue;
        if (!methodology.agents.some((agent) => agent.slug === ref)) {
          missingAgents.push(`${stage.slug}->${ref}`);
        }
      }
    }
    add(
      missingAgents.length === 0 ? "pass" : "fail",
      missingAgents.length === 0
        ? "Every stage references a known agent"
        : `Unknown agents referenced: ${missingAgents.join(", ")}`,
    );
  }

  if (!existsSync(workspaceRoot(root))) {
    add("warn", "No aidlc/ workspace yet. It is created on first workflow.");
  } else {
    add("pass", `Workspace at ${workspaceRoot(root)}`);
    const memory = memoryDir(root);
    const missing = REQUIRED_MEMORY.filter((file) => !existsSync(join(memory, file)));
    add(
      missing.length === 0 ? "pass" : "warn",
      missing.length === 0
        ? "Memory files present (org, team, project)"
        : `Missing memory files: ${missing.join(", ")}`,
    );
  }

  const harness = detectHarness(root);
  const dir = harnessDir(root, harness);
  add(
    existsSync(join(root, dir)) ? "pass" : "warn",
    existsSync(join(root, dir))
      ? `Harness ${harness} configured at ${dir}/`
      : `Harness directory ${dir}/ not found; run "my-aidlc config --harness ${harness}".`,
  );

  const config = loadConfig(root);
  add("pass", `Default scope: ${config.defaultScope}`);
  add(
    config.mode === "yolo" ? "warn" : "pass",
    config.mode === "yolo"
      ? "Execution mode: yolo (questions skipped and gates auto-approved; every auto-approval is recorded in the audit log)"
      : `Execution mode: ${config.mode}`,
  );

  const state = loadState(root);
  if (state) {
    add("pass", `Active intent: ${state.activeIntent.id} (${state.activeIntent.scope})`);
    const scope = methodology ? scopeByName(methodology, state.activeIntent.scope) : null;
    if (scope && methodology) {
      const workflow = workflowForScope(methodology, scope);
      const known = new Set(workflow.map((stage) => stage.slug));
      if (state.currentStage && !known.has(state.currentStage)) {
        add("fail", `Current stage "${state.currentStage}" is not in scope "${scope.name}".`);
      } else {
        add("pass", `Current stage resolves: ${state.currentStage || "(complete)"}`);
      }
      const incomplete = workflow.filter(
        (stage) => state.stages[stage.slug]?.status === "complete",
      );
      add("pass", `${incomplete.length}/${workflow.length} stages complete`);
    }
  } else {
    add("warn", "No active workflow state.");
  }

  const failures = checks.filter((check) => check.status === "fail").length;
  return { checks, ok: failures === 0, harness, scope: config.defaultScope };
}
