import { axe } from "jest-axe";
import { expect } from "vitest";

/**
 * Runs the automated accessibility checks on a rendered surface. A violation
 * fails the test with the rule id and the offending node, so a regression is
 * actionable rather than a bare count.
 */
export async function expectNoAxeViolations(container: HTMLElement): Promise<void> {
  const results = await axe(container);
  const summary = results.violations.map(
    (violation) =>
      `${violation.id}: ${violation.help} (${violation.nodes
        .map((node) => node.target.join(" "))
        .join(", ")})`,
  );
  expect(summary).toEqual([]);
}
