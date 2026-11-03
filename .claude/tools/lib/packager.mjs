// core/tools/lib/packager.mjs — projects the harness-neutral core/ tree into
// one harness distribution, and applies it to a project via `config`.

import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, relative } from "node:path";
import { pathToFileURL } from "node:url";
import { readVersion } from "./version.mjs";

const TEXT_EXTENSIONS = new Set([".md", ".json", ".toml", ".mjs", ".js", ".txt", ".yaml", ".yml", ".sh", ".ps1"]);

function isTextFile(path) {
  const dot = path.lastIndexOf(".");
  return dot >= 0 && TEXT_EXTENSIONS.has(path.slice(dot));
}

export function substituteTokens(text, manifest) {
  return text
    .replaceAll(".claude", manifest.harnessDir)
    .replaceAll("/aidlc", manifest.invoke)
    .replaceAll("Claude Code", manifest.productName);
}

function copyFileWithSubstitution(src, dst, manifest) {
  mkdirSync(dirname(dst), { recursive: true });
  if (isTextFile(src) || isTextFile(dst)) {
    const text = readFileSync(src, "utf8");
    writeFileSync(dst, substituteTokens(text, manifest), "utf8");
  } else {
    writeFileSync(dst, readFileSync(src));
  }
}

function copyDirWithSubstitution(src, dst, manifest) {
  if (!existsSync(src)) return;
  const stats = statSync(src);
  if (stats.isFile()) {
    copyFileWithSubstitution(src, dst, manifest);
    return;
  }
  mkdirSync(dst, { recursive: true });
  for (const entry of readdirSync(src, { withFileTypes: true })) {
    const from = join(src, entry.name);
    const to = join(dst, entry.name);
    if (entry.isDirectory()) copyDirWithSubstitution(from, to, manifest);
    else if (entry.isFile()) copyFileWithSubstitution(from, to, manifest);
  }
}

export async function loadManifest(repoRoot, harnessName) {
  const path = join(repoRoot, "harness", harnessName, "manifest.mjs");
  if (!existsSync(path)) throw new Error(`Unknown harness "${harnessName}" (no ${path}).`);
  const module = await import(pathToFileURL(path).href);
  return module.default;
}

export function listHarnesses(repoRoot) {
  const root = join(repoRoot, "harness");
  if (!existsSync(root)) return [];
  return readdirSync(root, { withFileTypes: true })
    .filter((entry) => entry.isDirectory() && existsSync(join(root, entry.name, "manifest.mjs")))
    .map((entry) => entry.name)
    .sort();
}

/** Build one harness distribution into outDir. */
export async function buildHarness({ repoRoot, harnessName, outDir }) {
  const manifest = await loadManifest(repoRoot, harnessName);
  const coreRoot = join(repoRoot, "core");
  const harnessRoot = join(repoRoot, "harness", harnessName);
  const harnessTree = join(outDir, manifest.harnessDir);

  rmSync(outDir, { recursive: true, force: true });
  mkdirSync(harnessTree, { recursive: true });

  for (const dir of manifest.coreDirs || []) {
    copyDirWithSubstitution(join(coreRoot, dir.src), join(harnessTree, dir.dst), manifest);
  }
  for (const file of manifest.coreFiles || []) {
    copyDirWithSubstitution(join(coreRoot, file.src), join(harnessTree, file.dst), manifest);
  }
  for (const file of manifest.coreProjectFiles || []) {
    copyDirWithSubstitution(join(coreRoot, file.src), join(outDir, file.dst), manifest);
  }
  for (const file of manifest.harnessFiles || []) {
    copyDirWithSubstitution(join(harnessRoot, file.src), join(harnessTree, file.dst), manifest);
  }
  for (const file of manifest.projectFiles || []) {
    copyDirWithSubstitution(join(harnessRoot, file.src), join(outDir, file.dst), manifest);
  }

  if (manifest.onboarding) {
    const target = manifest.onboarding.projectRoot
      ? join(outDir, manifest.onboarding.dst)
      : join(harnessTree, manifest.onboarding.dst);
    if (manifest.onboarding.content) {
      writeFileSync(target, substituteTokens(manifest.onboarding.content, manifest), "utf8");
    } else if (manifest.onboarding.src) {
      copyFileWithSubstitution(join(harnessRoot, manifest.onboarding.src), target, manifest);
    }
  }

  // Stamp the projected engine with the release version so it reports
  // correctly without a repository package.json.
  writeFileSync(
    join(harnessTree, "version.json"),
    `${JSON.stringify({ version: readVersion() }, null, 2)}\n`,
    "utf8",
  );

  return { manifest, outDir };
}

/** Copy a built distribution into a live project root (used by `config`). */
export function applyDistribution({ distributionDir, projectRoot, manifest }) {
  const written = [];
  const tree = join(distributionDir, manifest.harnessDir);
  const harnessTarget = join(projectRoot, manifest.harnessDir);
  copyDirWithSubstitution(tree, harnessTarget, manifest);
  written.push(manifest.harnessDir);

  for (const file of manifest.projectFiles || []) {
    const from = join(distributionDir, file.dst);
    if (!existsSync(from)) continue;
    // A managed .gitignore is merged rather than overwritten.
    if (file.dst === ".gitignore") {
      mergeGitignoreBlock(projectRoot, from, manifest.name);
    } else {
      copyDirWithSubstitution(from, join(projectRoot, file.dst), manifest);
    }
    written.push(file.dst);
  }
  for (const file of manifest.coreProjectFiles || []) {
    const from = join(distributionDir, file.dst);
    if (!existsSync(from)) continue;
    copyDirWithSubstitution(from, join(projectRoot, file.dst), manifest);
    written.push(file.dst);
  }
  if (manifest.onboarding && manifest.onboarding.projectRoot) {
    const from = join(distributionDir, manifest.onboarding.dst);
    if (existsSync(from)) {
      mergeManagedBlock(join(projectRoot, manifest.onboarding.dst), from, `my-aidlc:${manifest.name}`);
      written.push(manifest.onboarding.dst);
    }
  }
  return written;
}

/** Merge a managed block into a root text file, preserving other content. */
export function mergeManagedBlock(target, source, marker) {
  const begin = `# BEGIN ${marker}`;
  const end = `# END ${marker}`;
  const block = `${begin}\n${readFileSync(source, "utf8").trimEnd()}\n${end}\n`;
  let existing = existsSync(target) ? readFileSync(target, "utf8") : "";
  const pattern = new RegExp(`${escapeRegExp(begin)}[\\s\\S]*?${escapeRegExp(end)}\\n?`, "m");
  if (pattern.test(existing)) {
    existing = existing.replace(pattern, block);
  } else {
    existing = existing.trimEnd();
    existing = existing ? `${existing}\n\n${block}` : block;
  }
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, existing, "utf8");
}

function mergeGitignoreBlock(projectRoot, source, harnessName) {
  const target = join(projectRoot, ".gitignore");
  const lines = readFileSync(source, "utf8")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  const marker = `my-aidlc:${harnessName}`;
  const begin = `# BEGIN ${marker}`;
  const end = `# END ${marker}`;
  const block = `${begin}\n${lines.join("\n")}\n${end}\n`;
  let existing = existsSync(target) ? readFileSync(target, "utf8") : "";
  const pattern = new RegExp(`${escapeRegExp(begin)}[\\s\\S]*?${escapeRegExp(end)}\\n?`, "m");
  if (pattern.test(existing)) {
    existing = existing.replace(pattern, block);
  } else {
    existing = existing.trimEnd();
    existing = existing ? `${existing}\n\n${block}` : block;
  }
  writeFileSync(target, existing, "utf8");
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/** Compare two directory trees; returns a list of differences. */
export function diffTrees(a, b) {
  const differences = [];
  const walk = (dirA, dirB, prefix) => {
    const entriesA = existsSync(dirA) ? readdirSync(dirA).sort() : [];
    const entriesB = existsSync(dirB) ? readdirSync(dirB).sort() : [];
    for (const name of new Set([...entriesA, ...entriesB])) {
      const pathA = join(dirA, name);
      const pathB = join(dirB, name);
      const rel = prefix ? `${prefix}/${name}` : name;
      const inA = existsSync(pathA);
      const inB = existsSync(pathB);
      if (!inA) differences.push(`only in generated: ${rel}`);
      else if (!inB) differences.push(`only in dist: ${rel}`);
      else if (statSync(pathA).isDirectory() && statSync(pathB).isDirectory()) walk(pathA, pathB, rel);
      else if (readFileSync(pathA).compare(readFileSync(pathB)) !== 0) differences.push(`differs: ${rel}`);
    }
  };
  walk(a, b, "");
  return differences;
}

export { copyDirWithSubstitution, relative };
