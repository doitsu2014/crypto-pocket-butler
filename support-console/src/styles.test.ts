import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const css = readFileSync(join(process.cwd(), "src", "styles.css"), "utf8");

describe("accessibility styles", () => {
  it("gives every control a visible keyboard focus ring", () => {
    expect(css).toContain(":focus-visible");
    expect(css).toContain("--shadow-focus");
    expect(css).toContain("box-shadow: var(--shadow-focus)");
  });

  it("draws a non-colour shape for every status cue", () => {
    expect(css).toContain(".status-shape-dot");
    expect(css).toContain(".status-shape-diamond");
    expect(css).toContain(".status-shape-square");
    expect(css).toContain(".status-shape-ring");
  });

  it("respects prefers-reduced-motion", () => {
    expect(css).toContain("prefers-reduced-motion: reduce");
    expect(css).toContain("transition: none !important");
  });
});
