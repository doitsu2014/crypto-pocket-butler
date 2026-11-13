import { render, screen } from "@testing-library/react";
import { axe } from "jest-axe";
import { describe, expect, it } from "vitest";
import { RoleBadge } from "./RoleBadge";

describe("RoleBadge", () => {
  it.each([
    ["owner", "Owner", "ring"],
    ["admin", "Admin", "dot"],
    ["support", "Support", "diamond"],
  ] as const)("renders %s by label and shape", (role, label, shape) => {
    render(<RoleBadge role={role} />);

    const badge = screen.getByTestId("role-badge");
    expect(badge).toHaveTextContent(label);
    expect(badge.querySelector("[data-shape]")).toHaveAttribute("data-shape", shape);
    expect(badge).toHaveAccessibleName(new RegExp(label, "i"));
  });

  it("never carries the role by colour alone", () => {
    const { container } = render(<RoleBadge role="admin" />);

    const badge = screen.getByTestId("role-badge");
    expect(badge).toHaveTextContent(/admin/i);
    expect(badge.querySelector("[data-shape]")).not.toBeNull();
    expect(container.textContent).not.toBe("");
  });

  it("has no accessibility violations", async () => {
    const { container } = render(<RoleBadge role="support" />);
    expect((await axe(container)).violations).toEqual([]);
  });
});
