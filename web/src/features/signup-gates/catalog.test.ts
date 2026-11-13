import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import en from "../../i18n/en.json";
import vi from "../../i18n/vi.json";

/**
 * The `gates.*` catalogue contract (plan step 7.1/7.2; functional-spec §2.2,
 * §2.6, §4; NFR26).
 *
 * Every string the feature renders exists in both English and Vietnamese with
 * a real translation, and the two catalogues carry the same key set so a
 * missing Vietnamese key can never fall back silently.
 */

type Catalogue = Record<string, string>;

const enKeys = en as Catalogue;
const viKeys = vi as Catalogue;

/** Every account key the feature uses, including the dynamically built ones. */
const REQUIRED = [
  "gates.title",
  "gates.intro",
  "gates.loading",
  "gates.submitting",
  "gates.continue",
  "gates.retry",
  "gates.accepted",
  "gates.form.legend",
  "gates.summary.title",
  "gates.country.label",
  "gates.country.placeholder",
  "gates.country.required",
  "gates.terms.label",
  "gates.terms.required",
  "gates.terms.document",
  "gates.terms.documentHref",
  "gates.risk.label",
  "gates.risk.required",
  "gates.risk.document",
  "gates.risk.documentHref",
  "gates.age.label",
  "gates.age.required",
  "gates.blocked.title",
  "gates.blocked.signupBody",
  "gates.blocked.continuedUseBody",
  "gates.blocked.notStored",
  "gates.blocked.support",
  "gates.blocked.supportHref",
  "gates.blocked.signOut",
  "gates.error.nothingSaved",
  "gates.error.incomplete",
  "gates.error.correlation",
  "gates.error.retry",
  "gates.entryError.title",
  "gates.entryError.body",
  "gates.unavailable.title",
  "gates.unavailable.body",
];

const FEATURE_DIR = join(process.cwd(), "src/features/signup-gates");

/** The `gates.*` keys written literally in the feature's production modules. */
function literalKeys(): string[] {
  const found = new Set<string>();
  for (const file of readdirSync(FEATURE_DIR)) {
    if (!file.endsWith(".ts") && !file.endsWith(".tsx")) {
      continue;
    }
    if (file.endsWith(".test.ts") || file.endsWith(".test.tsx")) {
      continue;
    }
    const source = readFileSync(join(FEATURE_DIR, file), "utf8");
    for (const match of source.matchAll(/["'`](gates\.[a-zA-Z0-9_.]+)["'`]/g)) {
      found.add(match[1]);
    }
  }
  return [...found];
}

function isRealTranslation(catalogue: Catalogue, key: string): boolean {
  const value = catalogue[key];
  return typeof value === "string" && value.trim().length > 0 && value !== key;
}

describe("gates.* catalogue", () => {
  it("defines every required key in English", () => {
    for (const key of REQUIRED) {
      expect(isRealTranslation(enKeys, key), key).toBe(true);
    }
  });

  it("defines every required key in Vietnamese", () => {
    for (const key of REQUIRED) {
      expect(isRealTranslation(viKeys, key), key).toBe(true);
    }
  });

  it("keeps the English and Vietnamese gates key sets equal", () => {
    const gates = (catalogue: Catalogue) =>
      Object.keys(catalogue)
        .filter((key) => key.startsWith("gates."))
        .sort();
    expect(gates(viKeys)).toEqual(gates(enKeys));
  });

  it("covers every literal gates.* key the feature sources reference", () => {
    const keys = literalKeys();
    expect(keys.length).toBeGreaterThan(10);
    for (const key of keys) {
      expect(isRealTranslation(enKeys, key), key).toBe(true);
      expect(isRealTranslation(viKeys, key), key).toBe(true);
    }
  });

  it("links the terms and risk documents per locale", () => {
    for (const key of ["gates.terms.documentHref", "gates.risk.documentHref"]) {
      expect(enKeys[key]).toMatch(/(^\/|^https?:\/\/)/);
      expect(viKeys[key]).toMatch(/(^\/|^https?:\/\/)/);
      expect(enKeys[key]).not.toBe(viKeys[key]);
      expect(enKeys[key]).toContain("lang=en");
      expect(viKeys[key]).toContain("lang=vi");
    }
  });

  it("names all four missing items and distinguishes the two error variants", () => {
    for (const key of [
      "gates.country.required",
      "gates.terms.required",
      "gates.risk.required",
      "gates.age.required",
    ]) {
      expect(enKeys[key]).not.toBe(viKeys[key]);
    }
    expect(enKeys["gates.error.nothingSaved"]).not.toBe(enKeys["gates.error.incomplete"]);
    expect(viKeys["gates.error.nothingSaved"]).not.toBe(viKeys["gates.error.incomplete"]);
    expect(enKeys["gates.blocked.notStored"]).not.toBe(enKeys["gates.blocked.continuedUseBody"]);
  });
});
