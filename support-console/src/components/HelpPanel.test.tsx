import { render, screen } from "@testing-library/react";
import { axe } from "jest-axe";
import { describe, expect, it } from "vitest";
import { HelpPanel } from "./HelpPanel";

describe("HelpPanel", () => {
  it("renders the shared help articles read-only", () => {
    render(<HelpPanel />);

    expect(screen.getByRole("heading", { name: /help/i })).toBeInTheDocument();
    expect(screen.getByText(/connect an exchange/i)).toBeInTheDocument();
    expect(screen.getByText(/where is my portfolio/i)).toBeInTheDocument();
  });

  it("links to the customer help channel instead of forking content", () => {
    render(<HelpPanel />);

    expect(screen.getByTestId("help-panel-contact")).toHaveAttribute(
      "href",
      "https://cryptopocketbutler.example/help",
    );
  });

  it("offers no editing controls", () => {
    render(<HelpPanel />);
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("has no accessibility violations", async () => {
    const { container } = render(<HelpPanel />);
    expect((await axe(container)).violations).toEqual([]);
  });
});
