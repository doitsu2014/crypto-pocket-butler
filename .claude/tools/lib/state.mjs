// core/tools/lib/state.mjs — workflow state and the append-only audit log.

import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { auditPath, statePath } from "./paths.mjs";
import { readVersion } from "./version.mjs";

export const WORKFLOW_VERSION = "1.0.0";

export function loadState(root) {
  if (!existsSync(statePath(root))) return null;
  try {
    return JSON.parse(readFileSync(statePath(root), "utf8"));
  } catch {
    return null;
  }
}

export function saveState(root, state) {
  state.updatedAt = new Date().toISOString();
  const path = statePath(root);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(state, null, 2)}\n`, "utf8");
  return state;
}

export function newState({ intent, scope }) {
  const now = new Date().toISOString();
  return {
    workflowVersion: WORKFLOW_VERSION,
    engineVersion: readVersion(),
    activeIntent: intent,
    currentStage: null,
    parked: false,
    stages: {},
    startedAt: now,
    updatedAt: now,
  };
}

export function stageRecord(state, slug) {
  if (!state.stages[slug]) {
    state.stages[slug] = { status: "pending", attempt: 0, feedback: [], updatedAt: null };
  }
  return state.stages[slug];
}

export function isStageComplete(state, slug) {
  const record = state.stages[slug];
  return record && (record.status === "complete" || record.status === "skipped");
}

export function appendAudit(root, row) {
  const full = { ts: new Date().toISOString(), engineVersion: readVersion(), ...row };
  const path = auditPath(root);
  mkdirSync(dirname(path), { recursive: true });
  appendFileSync(path, `${JSON.stringify(full)}\n`, "utf8");
  return full;
}

export function readAudit(root, limit = Infinity) {
  if (!existsSync(auditPath(root))) return [];
  const lines = readFileSync(auditPath(root), "utf8").split(/\r?\n/).filter(Boolean);
  const rows = [];
  for (const line of lines) {
    try {
      rows.push(JSON.parse(line));
    } catch {
      // ignore malformed rows
    }
  }
  return Number.isFinite(limit) ? rows.slice(-limit) : rows;
}

/** Build a kebab-case intent label from a description (max three words). */
export function labelFromDescription(description) {
  const words = String(description || "")
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, " ")
    .split(/[\s-]+/)
    .filter(Boolean)
    .slice(0, 3);
  return words.length ? words.join("-") : "intent";
}

/** Build a unique intent id: YYMMDD-<label>[-n]. */
export function newIntentId(label, existing = []) {
  const now = new Date();
  const stamp = [
    String(now.getFullYear()).slice(2),
    String(now.getMonth() + 1).padStart(2, "0"),
    String(now.getDate()).padStart(2, "0"),
  ].join("");
  const base = `${stamp}-${label}`;
  if (!existing.includes(base)) return base;
  let counter = 2;
  while (existing.includes(`${base}-${counter}`)) counter += 1;
  return `${base}-${counter}`;
}
