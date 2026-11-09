import { render, screen } from "@testing-library/react";
import { axe } from "jest-axe";
import { describe, expect, it } from "vitest";
import { StatusFooter } from "./StatusFooter";

describe("StatusFooter", () => {
  it("links to the public status page owned by the customer app", () => {
    render(<StatusFooter />);
    expect(screen.getByTestId("status-footer-link")).toHaveAttribute("href", "/status");
  });

  it("states that the console is metadata only", () => {
    render(<StatusFooter />);
    expect(screen.getByTestId("status-footer")).toHaveTextContent(/metadata only/i);
  });

  it("has no accessibility violations", async () => {
    const { container } = render(<StatusFooter />);
    expect((await axe(container)).violations).toEqual([]);
  });
});
