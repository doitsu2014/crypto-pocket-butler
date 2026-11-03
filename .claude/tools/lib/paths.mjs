// core/tools/lib/paths.mjs — project + workspace path resolution.

import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

export const DEFAULT_SPACE = "default";

/** Known harness directory names, keyed by manifest name. */
export const HARNESS_DIRS = {
  pi: ".pi",
  claude: ".claude",
  codex: ".codex",
  deepseek: ".dsh",
};

/** Known harness invocation strings. */
export const HARNESS_INVOKE = {
  pi: "/aidlc",
  claude: "/aidlc",
  codex: "$aidlc",
  deepseek: "/aidlc",
};

export function projectRoot(cwd = process.cwd()) {
  return resolve(cwd);
}

export function workspaceRoot(root) {
  return join(root, "aidlc");
}

export function configPath(root) {
  return join(workspaceRoot(root), "config.json");
}

export function configLocalPath(root) {
  return join(workspaceRoot(root), "config.local.json");
}

export function statePath(root) {
  return join(workspaceRoot(root), "state.json");
}

export function auditPath(root) {
  return join(workspaceRoot(root), "audit.log");
}

export function spaceDir(root, space = DEFAULT_SPACE) {
  return join(workspaceRoot(root), "spaces", space);
}

export function memoryDir(root, space = DEFAULT_SPACE) {
  return join(spaceDir(root, space), "memory");
}

/**
 * The active space. `aidlc/active-space` is the cursor (mirroring native
 * AI-DLC); when it is absent or empty, the space is `default`.
 */
export function activeSpace(root) {
  const cursor = join(workspaceRoot(root), "active-space");
  try {
    const name = readFileSync(cursor, "utf8").trim().split(/\r?\n/)[0].trim();
    if (name) return name;
  } catch {
    // No cursor yet: fall through to the default space.
  }
  return DEFAULT_SPACE;
}

/**
 * Where a space's workflow scopes live: `aidlc/spaces/<space>/scopes/`.
 * These layer over the native baseline in `core/native/scopes/`.
 */
export function spaceScopesDir(root, space = activeSpace(root)) {
  return join(spaceDir(root, space), "scopes");
}

export function intentsDir(root, space = DEFAULT_SPACE) {
  return join(spaceDir(root, space), "intents");
}

export function intentDir(root, intentId, space = DEFAULT_SPACE) {
  return join(intentsDir(root, space), intentId);
}

export function artifactDir(root, intentId, phase, stage, space = DEFAULT_SPACE) {
  return join(intentDir(root, intentId, space), phase, stage);
}

export function stageMemoryPath(root, intentId, phase, stage, space = DEFAULT_SPACE) {
  return join(artifactDir(root, intentId, phase, stage, space), "memory.md");
}

export function readJson(path, fallback = null) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch {
    return fallback;
  }
}

/** Resolve the active harness from project config, then by directory presence. */
export function detectHarness(root) {
  const config = readJson(configPath(root), null);
  if (config && typeof config.harness === "string" && HARNESS_DIRS[config.harness]) {
    return config.harness;
  }
  for (const name of Object.keys(HARNESS_DIRS)) {
    if (existsSync(join(root, HARNESS_DIRS[name]))) return name;
  }
  return "pi";
}

export function harnessDir(root, name = detectHarness(root)) {
  return HARNESS_DIRS[name] || HARNESS_DIRS.pi;
}

export function harnessInvoke(root, name = detectHarness(root)) {
  return HARNESS_INVOKE[name] || HARNESS_INVOKE.pi;
}
