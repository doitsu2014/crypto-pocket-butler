// core/tools/lib/version.mjs — framework version and layout-aware roots.
//
// The engine runs from two layouts:
//   repo:      <repo>/core/tools/lib/version.mjs   -> engineRoot = <repo>/core
//   projected: <project>/.pi/tools/lib/version.mjs -> engineRoot = <project>/.pi
// In both cases engineRoot is two levels up from this file and repoRoot is
// three levels up. MY_AIDLC_HOME overrides for a global install.

import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export const FALLBACK_VERSION = "0.1.0";

function moduleHere() {
  return dirname(fileURLToPath(import.meta.url));
}

export function engineRoot() {
  if (process.env.MY_AIDLC_HOME) {
    const core = join(process.env.MY_AIDLC_HOME, "core");
    if (existsSync(core)) return core;
    if (existsSync(join(process.env.MY_AIDLC_HOME, "native"))) return process.env.MY_AIDLC_HOME;
  }
  return join(moduleHere(), "..", "..");
}

export function repoRoot() {
  if (process.env.MY_AIDLC_HOME && existsSync(process.env.MY_AIDLC_HOME)) {
    return process.env.MY_AIDLC_HOME;
  }
  return join(moduleHere(), "..", "..", "..");
}

export function readVersion() {
  const projected = join(engineRoot(), "version.json");
  if (existsSync(projected)) {
    try {
      return JSON.parse(readFileSync(projected, "utf8")).version || FALLBACK_VERSION;
    } catch {
      /* fall through */
    }
  }
  const pkg = join(repoRoot(), "package.json");
  if (existsSync(pkg)) {
    try {
      return JSON.parse(readFileSync(pkg, "utf8")).version || FALLBACK_VERSION;
    } catch {
      /* fall through */
    }
  }
  return FALLBACK_VERSION;
}
