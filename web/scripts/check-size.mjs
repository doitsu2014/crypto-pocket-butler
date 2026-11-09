#!/usr/bin/env node
// Bundle budget gate for the web app (NFR2.3).
//
// `size-limit` checks the entry chunk and stylesheet. This script checks the
// two budgets size-limit cannot express with a glob:
//   * the full initial chunk set (the entry script plus every chunk Vite
//     eagerly preloads from index.html) at <= 300 kB gzip;
//   * every route chunk at <= 150 kB gzip, measured one file at a time.
//
// Run `yarn build` first: the script reads the built `dist/` tree.

import { readFile, readdir } from "node:fs/promises";
import { gzipSync } from "node:zlib";
import { join } from "node:path";

const INITIAL_LIMIT = 300 * 1024;
const ROUTE_LIMIT = 150 * 1024;

const dist = new URL("../dist/", import.meta.url).pathname;
const html = await readFile(join(dist, "index.html"), "utf8");

/** Collect the initial script and modulepreload URLs referenced by index.html. */
function initialUrls(markup) {
  const urls = new Set();
  const script = /<script[^>]+src="([^"]+)"/g;
  const preload = /<link[^>]+rel="modulepreload"[^>]+href="([^"]+)"/g;
  for (const match of markup.matchAll(script)) {
    urls.add(match[1]);
  }
  for (const match of markup.matchAll(preload)) {
    urls.add(match[1]);
  }
  return [...urls]
    .filter((url) => url.startsWith("/") && url.endsWith(".js"))
    .map((url) => url.replace(/^\//, ""));
}

function gzipBytes(buffer) {
  return gzipSync(buffer, { level: 9 }).length;
}

function kib(bytes) {
  return `${(bytes / 1024).toFixed(1)} kB`;
}

const failures = [];
const initial = initialUrls(html);
let initialTotal = 0;

if (initial.length === 0) {
  failures.push("index.html references no initial JavaScript chunk");
}

for (const file of initial) {
  const buffer = await readFile(join(dist, file));
  initialTotal += gzipBytes(buffer);
}

if (initialTotal > INITIAL_LIMIT) {
  failures.push(
    `initial chunk set is ${kib(initialTotal)} gzip, over the ${kib(INITIAL_LIMIT)} budget`,
  );
}

const assets = await readdir(join(dist, "assets"));
const routeChunks = assets.filter(
  (file) => file.endsWith(".js") && !initial.some((entry) => entry.endsWith(file)),
);

for (const file of routeChunks) {
  const buffer = await readFile(join(dist, "assets", file));
  const size = gzipBytes(buffer);
  if (size > ROUTE_LIMIT) {
    failures.push(`route chunk ${file} is ${kib(size)} gzip, over the ${kib(ROUTE_LIMIT)} budget`);
  }
}

console.log(`initial chunk set: ${kib(initialTotal)} gzip across ${initial.length} file(s)`);
console.log(`route chunks checked: ${routeChunks.length} (limit ${kib(ROUTE_LIMIT)} each)`);

if (failures.length > 0) {
  for (const failure of failures) {
    console.error(`size budget failed: ${failure}`);
  }
  process.exit(1);
}

console.log("size budgets met");
