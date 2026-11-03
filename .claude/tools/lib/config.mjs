// core/tools/lib/config.mjs — project configuration layering.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import {
  configLocalPath,
  configPath,
  detectHarness,
  HARNESS_DIRS,
} from "./paths.mjs";
import { readVersion } from "./version.mjs";

/** Default number of clarifying questions a stage may ask before its gate. */
export const DEFAULT_QUESTION_BUDGET = Object.freeze({ min: 0, max: 5 });

/** Execution modes. `normal` keeps human questions and approval gates. */
export const MODES = Object.freeze(["normal", "yolo"]);
export const DEFAULT_MODE = "normal";

export function normalizeMode(value, fallback = DEFAULT_MODE) {
  if (typeof value !== "string") return fallback;
  const mode = value.trim().toLowerCase();
  return MODES.includes(mode) ? mode : fallback;
}

/**
 * Resolve the execution mode with precedence scope > project > default.
 * Returns { mode, source }.
 */
export function resolveMode({ scope, config } = {}) {
  if (typeof scope === "string" && MODES.includes(scope.trim().toLowerCase())) {
    return { mode: scope.trim().toLowerCase(), source: "scope" };
  }
  if (typeof config === "string" && MODES.includes(config.trim().toLowerCase())) {
    return { mode: config.trim().toLowerCase(), source: "project" };
  }
  return { mode: DEFAULT_MODE, source: "default" };
}

/** Global phase-review toggle. `reviewRequired: true` gates every phase. */
export const DEFAULT_REVIEW_REQUIRED = false;

/** Coerce a review toggle from config, CLI, or frontmatter into a boolean. */
export function normalizeReviewRequired(value, fallback = DEFAULT_REVIEW_REQUIRED) {
  if (value === undefined || value === null) return fallback;
  if (value === true || value === 1) return true;
  if (value === false || value === 0) return false;
  if (typeof value === "string") {
    const text = value.trim().toLowerCase();
    if (["true", "on", "yes", "1"].includes(text)) return true;
    if (["false", "off", "no", "0", ""].includes(text)) return false;
  }
  return fallback;
}

/** Resolve the global phase-review toggle with precedence scope > project > default. */
export function resolveReviewRequired({ scope, config } = {}) {
  if (scope !== undefined && scope !== null) {
    return { required: normalizeReviewRequired(scope), source: "scope" };
  }
  if (config !== undefined && config !== null) {
    return { required: normalizeReviewRequired(config), source: "project" };
  }
  return { required: DEFAULT_REVIEW_REQUIRED, source: "default" };
}

/**
 * Normalise a question-budget value from anywhere it can be authored:
 *   - an object: { min, max }
 *   - a number:  max only (min 0)
 *   - "off" / "none" / false / 0: no questions
 * Invalid input falls back to the supplied default.
 */
export function normalizeBudget(value, fallback = DEFAULT_QUESTION_BUDGET) {
  if (value === undefined || value === null) return { ...fallback };
  if (value === false || value === "off" || value === "none") return { min: 0, max: 0 };
  if (typeof value === "number" && Number.isFinite(value)) {
    return clampBudget(0, Math.trunc(value));
  }
  if (typeof value === "string" && /^\d+$/.test(value.trim())) {
    return clampBudget(0, Number.parseInt(value, 10));
  }
  if (typeof value === "object") {
    const hasMin = Number.isFinite(value.min);
    const hasMax = Number.isFinite(value.max);
    if (!hasMin && !hasMax) return { ...fallback };
    const min = hasMin ? Math.trunc(value.min) : fallback.min;
    const max = hasMax ? Math.trunc(value.max) : fallback.max;
    return clampBudget(min, max);
  }
  return { ...fallback };
}

function clampBudget(min, max) {
  const lo = Math.max(0, min);
  let hi = Math.max(0, max);
  if (hi < lo) hi = lo;
  return { min: lo, max: hi };
}

/** True when a value carries at least one explicit min/max. */
export function hasBudget(value) {
  if (value === undefined || value === null) return false;
  if (value === false || value === "off" || value === "none") return true;
  if (typeof value === "number") return Number.isFinite(value);
  if (typeof value === "string") return /^\d+$/.test(value.trim());
  if (typeof value === "object") return Number.isFinite(value.min) || Number.isFinite(value.max);
  return false;
}

/**
 * Resolve the effective budget with precedence stage > scope > project > default.
 * Returns { min, max, source }.
 */
export function resolveQuestionBudget({ stage, scope, config } = {}) {
  if (hasBudget(stage)) return { ...normalizeBudget(stage), source: "stage" };
  if (hasBudget(scope)) return { ...normalizeBudget(scope), source: "scope" };
  if (hasBudget(config)) return { ...normalizeBudget(config), source: "project" };
  return { ...DEFAULT_QUESTION_BUDGET, source: "default" };
}

export function loadConfig(root) {
  const base = readJsonOrNull(configPath(root)) || {};
  const local = readJsonOrNull(configLocalPath(root)) || {};
  const merged = { ...base, ...local };
  return {
    ...merged,
    harness: merged.harness || null,
    defaultScope: merged.defaultScope || "classic",
    version: merged.version || readVersion(),
    mode: normalizeMode(merged.mode),
    reviewRequired: normalizeReviewRequired(merged.reviewRequired),
    questionBudget: normalizeBudget(merged.questionBudget),
  };
}

export function saveConfig(root, patch, { local = false } = {}) {
  const path = local ? configLocalPath(root) : configPath(root);
  const current = readJsonOrNull(path) || {};
  const next = { ...current, ...patch, version: readVersion() };
  if (patch && "questionBudget" in patch) {
    next.questionBudget = normalizeBudget(patch.questionBudget);
  }
  if (patch && "mode" in patch) {
    next.mode = normalizeMode(patch.mode, current.mode || DEFAULT_MODE);
  }
  if (patch && "reviewRequired" in patch) {
    next.reviewRequired = normalizeReviewRequired(patch.reviewRequired);
  }
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(next, null, 2)}\n`, "utf8");
  return next;
}

export function configExists(root) {
  return existsSync(configPath(root));
}

export function resolvedHarness(root) {
  const config = loadConfig(root);
  if (config.harness && HARNESS_DIRS[config.harness]) return config.harness;
  return detectHarness(root);
}

function readJsonOrNull(path) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch {
    return null;
  }
}
