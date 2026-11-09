import { render, screen } from "@testing-library/react";
import { axe } from "jest-axe";
import { describe, expect, it } from "vitest";
import { grantStatusPresentation } from "../domain/grants";
import { StatusBadge } from "./StatusBadge";

describe("StatusBadge", () => {
  it("always renders a text label and a shape, not colour alone", () => {
    render(<StatusBadge presentation={grantStatusPresentation("active")} testId="badge" />);

    const badge = screen.getByTestId("badge");
    expect(badge).toHaveTextContent("Active");
    const shape = badge.querySelector("[data-shape]");
    expect(shape).not.toBeNull();
    expect(shape).toHaveAttribute("data-shape", "dot");
    expect(shape).toHaveAttribute("aria-hidden", "true");
  });

  it("maps the tone to the matching token class", () => {
    const { rerender } = render(
      <StatusBadge presentation={grantStatusPresentation("expired")} testId="badge" />,
    );
    expect(screen.getByTestId("badge").className).toContain("text-danger");

    rerender(<StatusBadge presentation={grantStatusPresentation("active")} testId="badge" />);
    expect(screen.getByTestId("badge").className).toContain("text-success");
  });

  it("has no accessibility violations", async () => {
    const { container } = render(
      <StatusBadge presentation={grantStatusPresentation("pending_approval")} />,
    );
    expect((await axe(container)).violations).toEqual([]);
  });
});
