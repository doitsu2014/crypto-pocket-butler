// core/tools/lib/native.mjs — pinned acquisition of the native AWS AI-DLC.
//
// my-aidlc does not author the methodology. It pins exactly one AI-DLC release
// (aidlc-version.json), fetches that tag's source from AWS, and normalises the
// methodology it ships into `core/native/`.
//
// This module owns the whole snapshot: the download, the verification, the
// normalisation, and the projection-layer files (phase metadata, the README,
// and the adaptation bridge). `scripts/fetch-native.mjs` and `my-aidlc update`
// are both thin front ends over `updateSnapshot`, so the two can never drift.
//
// The committed snapshot exists so builds, lint, and tests work offline. It is
// derived, never hand-edited.
//
// Integrity is checked twice, because the two checks fail for different
// reasons:
//   1. the archive's SHA-256, against the pin (catches a substituted download);
//   2. every extracted methodology file's SHA-256, against `core/native/lock.json`
//      (catches a re-compressed or re-tagged archive, and drift in a file we
//      already shipped).
// An archive that fails (1) is still accepted when (2) passes; an archive that
// fails both is refused.

import { createHash } from "node:crypto";
import { gunzipSync } from "node:zlib";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, join } from "node:path";
import { NATIVE_PHASES, renderPhaseFile } from "./native-phases.mjs";

/** Read the pinned AI-DLC release descriptor. */
export function readPin(repoRoot) {
  const path = join(repoRoot, "aidlc-version.json");
  const pin = JSON.parse(readFileSync(path, "utf8"));
  for (const key of ["repository", "version", "tag", "asset", "assetUrl", "sha256", "archiveRoot", "sourcePath"]) {
    if (!pin[key]) throw new Error(`aidlc-version.json is missing "${key}".`);
  }
  return pin;
}

export function sha256(buffer) {
  return createHash("sha256").update(buffer).digest("hex");
}

export function sha256File(path) {
  return sha256(readFileSync(path));
}

/** Where the downloaded release archive is cached between runs. */
export function cachePath(repoRoot, pin) {
  return join(repoRoot, pin.cacheDir || ".cache/aidlc", pin.asset);
}

// ---------------------------------------------------------------------------
// Minimal, dependency-free tar reader.
//
// The repo has no runtime dependencies, so the release archive is unpacked
// here rather than by shelling out to `tar`. Handles the POSIX ustar layout
// plus the GNU long-name ('L') and pax ('x') extensions that archives use for
// paths longer than 100 bytes.
// ---------------------------------------------------------------------------

function cString(buffer, start, length) {
  const slice = buffer.subarray(start, start + length);
  const end = slice.indexOf(0);
  return slice.subarray(0, end === -1 ? slice.length : end).toString("utf8");
}

function parseNumeric(buffer, start, length) {
  // GNU base-256 encoding sets the high bit of the first byte.
  if (buffer[start] & 0x80) {
    let value = 0;
    for (let i = start + 1; i < start + length; i += 1) value = value * 256 + buffer[i];
    return value;
  }
  const text = cString(buffer, start, length).trim();
  if (text === "") return 0;
  const value = Number.parseInt(text, 8);
  return Number.isNaN(value) ? 0 : value;
}

function parsePax(data) {
  const fields = {};
  const text = data.toString("utf8");
  let offset = 0;
  while (offset < text.length) {
    const space = text.indexOf(" ", offset);
    if (space === -1) break;
    const length = Number.parseInt(text.slice(offset, space), 10);
    if (!Number.isFinite(length) || length <= 0) break;
    const record = text.slice(space + 1, offset + length);
    const eq = record.indexOf("=");
    if (eq !== -1) fields[record.slice(0, eq)] = record.slice(eq + 1).replace(/\n$/, "");
    offset += length;
  }
  return fields;
}

/**
 * Read a `.tar.gz` into `{ path, type, data }` records.
 * `type` is "file" or "directory"; symlinks and other node types are skipped.
 */
export function readTarGz(path) {
  const tar = gunzipSync(readFileSync(path));
  const entries = [];
  let offset = 0;
  let longName = null;
  let pax = null;

  while (offset + 512 <= tar.length) {
    const header = tar.subarray(offset, offset + 512);
    if (header.every((byte) => byte === 0)) break;

    const size = parseNumeric(header, 124, 12);
    const typeflag = String.fromCharCode(header[156]);
    const prefix = cString(header, 345, 155);
    const rawName = longName || cString(header, 0, 100);
    const name = prefix ? `${prefix}/${rawName}` : rawName;
    const bodyStart = offset + 512;
    const body = tar.subarray(bodyStart, bodyStart + size);
    const padded = Math.ceil(size / 512) * 512;

    if (typeflag === "L") {
      longName = body.toString("utf8").replace(/\0+$/, "");
    } else if (typeflag === "x" || typeflag === "g") {
      pax = parsePax(body);
    } else if (typeflag === "5") {
      entries.push({ path: name.replace(/\/+$/, ""), type: "directory", data: null });
      longName = null;
      pax = null;
    } else if (typeflag === "0" || typeflag === "\0" || typeflag === "") {
      entries.push({ path: pax && pax.path ? pax.path : name, type: "file", data: Buffer.from(body) });
      longName = null;
      pax = null;
    } else {
      longName = null;
      pax = null;
    }

    offset = bodyStart + padded;
  }
  return entries;
}

// ---------------------------------------------------------------------------
// Methodology selection + normalisation
// ---------------------------------------------------------------------------

/**
 * Pick the methodology files out of the tagged source archive and map them onto
 * the snapshot layout:
 *
 *   <root>/core/aidlc-common/stages/<phase>/<slug>.md -> stages/<phase>/<slug>.md
 *   <root>/core/aidlc-common/protocols/<slug>.md      -> protocols/<slug>.md
 *   <root>/core/agents/<slug>.md                      -> agents/<slug>.md
 *   <root>/core/scopes/aidlc-<name>.md                -> scopes/<name>.md
 *
 * Files are copied byte-for-byte, so `/aidlc` and `.claude` stay
 * intact and my-aidlc's packager can bind them per harness.
 */
export function selectMethodology(entries, pin) {
  const root = `${pin.archiveRoot.replace(/\/+$/, "")}/${pin.sourcePath.replace(/^\/+|\/+$/g, "")}/`;
  const files = new Map();
  const unrecognised = [];

  for (const entry of entries) {
    if (entry.type !== "file") continue;
    if (!entry.path.startsWith(root)) continue;
    const rel = entry.path.slice(root.length);

    let target = null;
    let match = rel.match(/^aidlc-common\/stages\/([a-z0-9-]+)\/([a-z0-9-]+)\.md$/);
    if (match) target = `stages/${match[1]}/${match[2]}.md`;

    if (!target) {
      match = rel.match(/^aidlc-common\/protocols\/([a-z0-9-]+)\.md$/);
      if (match) target = `protocols/${match[1]}.md`;
    }
    if (!target) {
      match = rel.match(/^agents\/([a-z0-9-]+)\.md$/);
      if (match) target = `agents/${match[1]}.md`;
    }
    if (!target) {
      match = rel.match(/^scopes\/aidlc-([a-z0-9-]+)\.md$/);
      if (match) target = `scopes/${match[1]}.md`;
    }

    if (target) files.set(target, entry.data);
    else if (rel.startsWith("aidlc-common/stages/")) unrecognised.push(rel);
  }

  return { files, unrecognised };
}

/**
 * Native engine invocations found in stage bodies, with the my-aidlc reading.
 * Deterministic: both maps are emitted in a stable order.
 */
function scanInvocations(files) {
  const engineCalls = new Map();
  const toolCalls = new Map();
  const bump = (map, key) => map.set(key, (map.get(key) || 0) + 1);

  for (const [path, data] of files) {
    if (!path.startsWith("stages/")) continue;
    const text = data.toString("utf8");
    for (const match of text.matchAll(/\{\{INVOKE\}\}\s+([a-z0-9-]+(?:\s+[a-z0-9-]+)?)/gi)) {
      bump(engineCalls, match[1].trim().replace(/\s+/g, " "));
    }
    for (const match of text.matchAll(/(?:bun|node)\s+\{\{HARNESS_DIR\}\}\/tools\/([a-z0-9-]+)/gi)) {
      bump(toolCalls, match[1].trim());
    }
  }

  const render = (map) =>
    [...map.entries()]
      .sort((a, b) => (a[1] === b[1] ? a[0].localeCompare(b[0]) : b[1] - a[1]))
      .map(([key, count]) => ({ key, count }));
  return { engineCalls: render(engineCalls), toolCalls: render(toolCalls) };
}

/** Render the bridge between the native runtime's calls and my-aidlc's engine. */
function renderAdaptation(files, pin) {
  const { engineCalls, toolCalls } = scanInvocations(files);
  const lines = [];
  lines.push("# Native stage adaptation");
  lines.push("");
  lines.push(
    `The stage bodies under \`stages/\` are copied byte-for-byte from **${pin.product} ${pin.version}** ` +
      `(\`${pin.repository}\`, tag \`${pin.tag}\`, commit \`${pin.commit}\`). They were written for the native ` +
      "runtime, whose tools live in the harness directory. my-aidlc runs its own Node engine, so this file is " +
      "the bridge the conductor reads when a stage body names a native call.",
  );
  lines.push("");
  lines.push("## Engine calls");
  lines.push("");
  lines.push(
    "`/aidlc` expands to the harness invocation (`/aidlc` on PI Agent, Claude Code, and DeepSeek; " +
      "`$aidlc` on Codex). my-aidlc keeps the same subcommand surface under its own binary, so read every " +
      "native engine call as:",
  );
  lines.push("");
  lines.push("| Native call in stage body | Occurs | Read as |");
  lines.push("| --- | --- | --- |");
  for (const { key, count } of engineCalls) {
    lines.push(`| \`/aidlc ${key}\` | ${count} | \`my-aidlc ${key}\` |`);
  }
  lines.push("");
  lines.push("## Runtime tools");
  lines.push("");
  if (toolCalls.length === 0) {
    lines.push("No stage body calls a native runtime tool.");
  } else {
    lines.push(
      "These helpers belong to the native runtime and are **not** shipped by my-aidlc. Treat each as " +
        "reference-only context: take the equivalent fact from the stage's `inputs`, from the directive, " +
        "or by asking the human. `my-aidlc doctor` reports the same list.",
    );
    lines.push("");
    lines.push("| Native helper | Occurs | Status |");
    lines.push("| --- | --- | --- |");
    for (const { key, count } of toolCalls) {
      lines.push(`| \`bun .claude/tools/${key}.ts\` | ${count} | reference only |`);
    }
  }
  lines.push("");
  lines.push("## Not carried over");
  lines.push("");
  lines.push(
    "Native ceremony that my-aidlc's engine does not implement is still named inside the bodies: sensors " +
      "(`## Sensors`), the learnings ritual (`## Learn`), summary confirmation, plan approval, collaborators, " +
      "plugins, and Bolt/worktree swarms. my-aidlc's own `core/protocols/stage-protocol.md` governs those " +
      "concerns; the native sections stay so the method remains faithful and auditable.",
  );
  lines.push("");
  return `${lines.join("\n")}\n`;
}

function renderSnapshotReadme(pin, counts) {
  return [
    "# core/native — generated from AWS AI-DLC",
    "",
    `This directory is a normalised snapshot of **${pin.product} ${pin.version}** (tag \`${pin.tag}\`,`,
    `commit \`${pin.commit}\`) from [\`${pin.repository}\`](https://github.com/${pin.repository}), pinned in`,
    "[`aidlc-version.json`](../../aidlc-version.json).",
    "",
    "**Do not edit anything here by hand.** Every file is owned by `my-aidlc update`",
    "(`node scripts/fetch-native.mjs`), which re-downloads the pinned source, verifies it, and rewrites",
    "this tree.",
    "",
    "| Directory | Contents |",
    "| --- | --- |",
    `| \`stages/<phase>/\` | The ${counts.stages} native stage definitions, byte-for-byte |`,
    `| \`agents/\` | The ${counts.agents} native agent personas |`,
    `| \`scopes/\` | The ${counts.scopes} native baseline workflow scopes |`,
    `| \`protocols/\` | The ${counts.protocols} native stage protocols (reference) |`,
    "| `phases/` | my-aidlc's display and gate metadata for the five native phases |",
    "| `lock.json` | The pin plus a SHA-256 for every file in this tree |",
    "| `ADAPTATION.md` | How native engine calls map onto the my-aidlc engine |",
    "",
    "Stage bodies keep the upstream `/aidlc` and `.claude` placeholders, so the packager",
    "binds them per harness instead of freezing one harness's paths.",
    "",
    "Workflow scopes the team authors live in the workspace, not here:",
    "`aidlc/spaces/<space>/scopes/<name>.md`. They layer over the baseline in `scopes/`.",
    "",
  ].join("\n");
}

function countByPrefix(files, prefix) {
  return [...files.keys()].filter((path) => path.startsWith(prefix)).length;
}

/**
 * Add the projection layer my-aidlc owns on top of the upstream files: the
 * five phase metadata files, the README, and the adaptation bridge. Returns a
 * new Map so callers keep the pristine methodology separate.
 */
export function projectSnapshotFiles(methodologyFiles, pin) {
  const files = new Map(methodologyFiles);
  const counts = {
    stages: countByPrefix(files, "stages/"),
    agents: countByPrefix(files, "agents/"),
    scopes: countByPrefix(files, "scopes/"),
    protocols: countByPrefix(files, "protocols/"),
  };
  for (const phase of NATIVE_PHASES) {
    files.set(`phases/${phase.slug}.md`, Buffer.from(renderPhaseFile(phase), "utf8"));
  }
  files.set("ADAPTATION.md", Buffer.from(renderAdaptation(methodologyFiles, pin), "utf8"));
  files.set("README.md", Buffer.from(renderSnapshotReadme(pin, counts), "utf8"));
  return { files, counts };
}

/** Files that make up the snapshot, sorted, with digests. */
export function snapshotIndex(files) {
  return [...files.keys()]
    .sort()
    .map((path) => ({ path, sha256: sha256(files.get(path)), bytes: files.get(path).length }));
}

/** Directories in the snapshot that come straight from the AWS release. */
const UPSTREAM_DIRS = ["stages/", "agents/", "scopes/", "protocols/"];

export function isUpstreamPath(path) {
  return UPSTREAM_DIRS.some((prefix) => path.startsWith(prefix));
}

/**
 * Compare extracted upstream files against a previous lock.
 * Returns `{ ok, compared, mismatches }`.
 */
export function verifyAgainstLock(files, lock) {
  const mismatches = [];
  let compared = 0;
  const expected = new Map(
    (lock.files || []).filter((entry) => isUpstreamPath(entry.path)).map((entry) => [entry.path, entry.sha256]),
  );
  for (const [path, data] of files) {
    if (!isUpstreamPath(path)) continue;
    const want = expected.get(path);
    if (want === undefined) continue;
    compared += 1;
    const got = sha256(data);
    if (got !== want) mismatches.push({ path, expected: want, actual: got });
  }
  return { ok: mismatches.length === 0, compared, mismatches };
}

function writeTree(outDir, files) {
  const wanted = new Set(files.keys());
  if (existsSync(outDir)) {
    // Remove snapshot files that upstream no longer ships, so a refresh cannot
    // leave a stale stage behind.
    const walk = (dir, prefix) => {
      for (const entry of readdirSync(dir, { withFileTypes: true })) {
        const rel = prefix ? `${prefix}/${entry.name}` : entry.name;
        const abs = join(dir, entry.name);
        if (entry.isDirectory()) {
          walk(abs, rel);
          if (readdirSync(abs).length === 0) rmdirSync(abs);
        } else if (!wanted.has(rel)) {
          rmSync(abs);
        }
      }
    };
    walk(outDir, "");
  }

  for (const [rel, data] of files) {
    const abs = join(outDir, rel);
    mkdirSync(dirname(abs), { recursive: true });
    writeFileSync(abs, data);
  }
}

/** Write the normalised methodology snapshot plus its lock file. */
export function writeSnapshot(outDir, files, pin, options = {}) {
  const index = snapshotIndex(files);
  const count = (prefix) => index.filter((entry) => entry.path.startsWith(prefix)).length;
  const lock = {
    repository: pin.repository,
    version: pin.version,
    tag: pin.tag,
    commit: pin.commit || null,
    asset: pin.asset,
    assetUrl: pin.assetUrl,
    archiveSha256: pin.sha256,
    archiveVerified: options.archiveVerified === true,
    sourcePath: pin.sourcePath,
    stages: count("stages/"),
    agents: count("agents/"),
    scopes: count("scopes/"),
    protocols: count("protocols/"),
    files: index,
  };

  mkdirSync(outDir, { recursive: true });
  writeTree(outDir, files);
  writeFileSync(join(outDir, "lock.json"), `${JSON.stringify(lock, null, 2)}\n`);
  return lock;
}

/** Read the committed lock, or null when no snapshot has been generated yet. */
export function readLock(repoRoot, pin) {
  const path = join(repoRoot, pin.snapshotDir || "core/native", "lock.json");
  if (!existsSync(path)) return null;
  return JSON.parse(readFileSync(path, "utf8"));
}

/** Is the committed snapshot still the pinned release? */
export function lockMatchesPin(lock, pin) {
  return (
    !!lock &&
    lock.version === pin.version &&
    lock.repository === pin.repository &&
    lock.archiveSha256 === pin.sha256
  );
}

/**
 * Verify a committed snapshot against its own lock: every listed file is
 * present and unmodified.
 * Returns `{ ok, reason, lock, drift }`.
 */
export function verifySnapshot(repoRoot, pin) {
  const lock = readLock(repoRoot, pin);
  if (!lock) return { ok: false, reason: "missing", lock: null, drift: [] };
  if (lock.version !== pin.version || lock.repository !== pin.repository) {
    return { ok: false, reason: "pin", lock, drift: [] };
  }
  const dir = join(repoRoot, pin.snapshotDir || "core/native");
  const drift = [];
  for (const entry of lock.files || []) {
    const abs = join(dir, entry.path);
    if (!existsSync(abs)) drift.push({ path: entry.path, kind: "missing" });
    else if (sha256File(abs) !== entry.sha256) drift.push({ path: entry.path, kind: "modified" });
  }
  return { ok: drift.length === 0, reason: drift.length ? "drift" : null, lock, drift };
}

// ---------------------------------------------------------------------------
// Download
// ---------------------------------------------------------------------------

/**
 * Fetch the pinned release archive into the cache. Never fails on a checksum
 * mismatch — the caller decides whether the content check is enough.
 * Returns `{ path, digest, bytes, archiveVerified, fromCache }`.
 */
export async function downloadSource(repoRoot, pin, options = {}) {
  const target = cachePath(repoRoot, pin);
  const log = options.log || (() => {});

  if (existsSync(target)) {
    const digest = sha256File(target);
    if (digest === pin.sha256) {
      log(`cached ${pin.asset} (archive sha256 verified)`);
      return { path: target, digest, bytes: statSync(target).size, archiveVerified: true, fromCache: true };
    }
    log(`cached ${pin.asset} does not match the pinned archive; re-downloading`);
    rmSync(target);
  }

  log(`downloading ${pin.asset} from ${pin.repository} ${pin.tag}`);
  const response = await fetch(pin.assetUrl, { redirect: "follow" });
  if (!response.ok) {
    throw new Error(`Could not download ${pin.assetUrl}: HTTP ${response.status} ${response.statusText}`);
  }
  const buffer = Buffer.from(await response.arrayBuffer());
  const digest = sha256(buffer);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, buffer);
  if (digest === pin.sha256) log(`archive sha256 verified (${digest})`);
  else log(`archive sha256 differs from the pin (${digest}); falling back to per-file verification`);
  return { path: target, digest, bytes: buffer.length, archiveVerified: digest === pin.sha256, fromCache: false };
}

/**
 * Full update path: fetch (or reuse) the pinned archive, verify it, normalise
 * the methodology, add the projection layer, and write `core/native/`.
 *
 * @param {object} [options]
 * @param {(message: string) => void} [options.log]
 * @param {string} [options.from]  Path to a local archive instead of downloading.
 * @param {boolean} [options.dryRun]
 */
export async function updateSnapshot(repoRoot, pin, options = {}) {
  const log = options.log || (() => {});
  const outDir = join(repoRoot, pin.snapshotDir || "core/native");

  let archivePath;
  let archiveVerified = false;
  if (options.from) {
    if (!existsSync(options.from)) throw new Error(`No such archive: ${options.from}`);
    archivePath = options.from;
    const digest = sha256File(archivePath);
    archiveVerified = digest === pin.sha256;
    log(
      archiveVerified
        ? `using local archive ${archivePath} (sha256 verified)`
        : `using local archive ${archivePath} (sha256 ${digest} differs from the pin; verifying per file)`,
    );
  } else {
    const asset = await downloadSource(repoRoot, pin, { log });
    archivePath = asset.path;
    archiveVerified = asset.archiveVerified;
  }

  log(`reading ${pin.asset}`);
  const entries = readTarGz(archivePath);
  const { files: methodology, unrecognised } = selectMethodology(entries, pin);
  if (unrecognised.length) {
    throw new Error(`Unrecognised stage paths in ${pin.asset}: ${unrecognised.join(", ")}`);
  }
  if (methodology.size === 0) {
    throw new Error(
      `No methodology found under "${pin.archiveRoot}/${pin.sourcePath}" in ${pin.asset}. ` +
        "The pinned release layout may have changed.",
    );
  }

  const previous = readLock(repoRoot, pin);
  if (!archiveVerified) {
    const check =
      previous && previous.version === pin.version
        ? verifyAgainstLock(methodology, previous)
        : { ok: false, compared: 0 };
    if (!check.ok) {
      throw new Error(
        `Refusing to install ${pin.asset}: the archive checksum does not match the pin ` +
          `(expected ${pin.sha256}) and ${check.compared} file(s) could not be verified against ` +
          `${pin.snapshotDir}/lock.json.`,
      );
    }
    log(`archive bytes changed upstream; ${check.compared} methodology file(s) verified against the committed lock`);
    archiveVerified = true;
  }

  const { files, counts } = projectSnapshotFiles(methodology, pin);
  if (!counts.stages || !counts.agents || !counts.scopes) {
    throw new Error(`The pinned release yielded an empty methodology (${JSON.stringify(counts)}).`);
  }

  const before = new Set(previous ? previous.files.map((entry) => entry.path) : []);
  if (options.dryRun) {
    log(
      `dry run: would write ${files.size} files (${counts.stages} stages, ${counts.agents} agents, ` +
        `${counts.scopes} scopes, ${counts.protocols} protocols); current snapshot v${previous ? previous.version : "none"}`,
    );
    return { lock: previous, outDir, added: 0, removed: 0, dryRun: true };
  }

  const lock = writeSnapshot(outDir, files, pin, { archiveVerified });
  const added = lock.files.filter((entry) => !before.has(entry.path)).length;
  const removed = [...before].filter((path) => !lock.files.some((entry) => entry.path === path)).length;
  log(
    `wrote ${pin.snapshotDir}/: ${lock.stages} stages, ${lock.agents} agents, ${lock.scopes} scopes, ` +
      `${lock.protocols} protocols (+${added} / -${removed}) from ${pin.repository} ${pin.tag}`,
  );
  return { lock, outDir, added, removed, dryRun: false };
}
