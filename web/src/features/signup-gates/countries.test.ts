import { describe, expect, it } from "vitest";
import { COUNTRIES } from "./countries";

/**
 * The generated country module (functional-spec §6 open item 5; performance-design §4).
 * It ships inside the lazy gate route chunk and is the only country dataset the
 * browser has — there is no runtime fetch and no restricted-list copy.
 */

function fold(value: string): string {
  return value
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .replace(/đ/g, "d")
    .replace(/Đ/g, "d")
    .toLowerCase();
}

describe("generated country module", () => {
  it("exports a frozen, non-empty array covering the full ISO 3166-1 alpha-2 list", () => {
    expect(Array.isArray(COUNTRIES)).toBe(true);
    expect(COUNTRIES.length).toBeGreaterThan(240);
    expect(COUNTRIES.length).toBeLessThan(270);
    expect(Object.isFrozen(COUNTRIES)).toBe(true);
  });

  it("uses unique uppercase alpha-2 codes", () => {
    const codes = COUNTRIES.map((country) => country.code);
    expect(new Set(codes).size).toBe(codes.length);
    for (const code of codes) {
      expect(code).toMatch(/^[A-Z]{2}$/);
    }
  });

  it("gives every entry a non-empty English and Vietnamese name", () => {
    for (const country of COUNTRIES) {
      expect(country.en.trim().length).toBeGreaterThan(0);
      expect(country.vi.trim().length).toBeGreaterThan(0);
    }
  });

  it("precomputes a lowercase, diacritic-free search key covering both names", () => {
    for (const country of COUNTRIES) {
      expect(country.search).toBe(country.search.toLowerCase());
      expect(country.search).not.toMatch(/\p{M}/u);
      expect(country.search).toContain(fold(country.en));
      expect(country.search).toContain(fold(country.vi));
    }
  });

  it("carries a known probe country with both locale names", () => {
    const vn = COUNTRIES.find((country) => country.code === "VN");
    expect(vn).toBeDefined();
    expect(vn?.en).toBe("Vietnam");
    expect(vn?.vi).toBe("Việt Nam");
    expect(vn?.search).toContain("vietnam");
    expect(vn?.search).toContain("viet nam");
  });

  it("folds diacritics in the search key rather than carrying them", () => {
    const vn = COUNTRIES.find((country) => country.code === "VN");
    expect(vn?.search).not.toContain("ệ");
    expect(fold("Việt Nam")).toBe("viet nam");
  });
});
