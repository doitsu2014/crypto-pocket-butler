import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import HelpPage from "./HelpPage";
import { ApiError } from "../../api/client";
import { renderWithProviders } from "../../test/render";
import type { Incident, SupportArticle } from "../../api/types";

const articles: SupportArticle[] = [
  {
    id: "a1",
    title: "Why is my balance missing a token?",
    summary: "Unpriced tokens are excluded from totals.",
    body: "An unpriced token is listed with an em dash and never counted as zero.",
  },
];

const incidents: Incident[] = [
  {
    id: "i1",
    title: "Exchange sync delayed",
    severity: "sev2",
    status: "investigating",
    declaredAt: "2026-11-08T08:00:00Z",
  },
];

describe("help centre", () => {
  it("lists articles and active incidents, then sends a support request", async () => {
    const createRequest = vi.fn().mockResolvedValue({ id: "req-1" });
    const user = userEvent.setup();
    renderWithProviders(
      <HelpPage
        articlesOverride={articles}
        incidentsOverride={incidents}
        createRequest={createRequest}
      />,
      { route: "/help" },
    );

    expect(screen.getByTestId("help-article")).toHaveTextContent(
      "Why is my balance missing a token?",
    );
    expect(screen.getByTestId("help-incident")).toHaveTextContent("Exchange sync delayed");

    await user.type(screen.getByTestId("help-subject"), "Missing DOGE");
    await user.type(screen.getByTestId("help-body"), "My DOGE balance is shown with a dash.");
    await user.click(screen.getByTestId("help-submit"));

    expect(createRequest).toHaveBeenCalledWith({
      subject: "Missing DOGE",
      body: "My DOGE balance is shown with a dash.",
    });
    expect(await screen.findByTestId("help-sent")).toBeInTheDocument();
  });

  it("blocks a message that looks like a credential and never sends it", async () => {
    const createRequest = vi.fn();
    const user = userEvent.setup();
    renderWithProviders(
      <HelpPage articlesOverride={[]} incidentsOverride={[]} createRequest={createRequest} />,
      { route: "/help" },
    );

    await user.type(screen.getByTestId("help-subject"), "Key");
    await user.type(screen.getByTestId("help-body"), `here is my key 0x${"a".repeat(64)}`);
    await user.click(screen.getByTestId("help-submit"));

    expect(screen.getByTestId("help-errors")).toHaveTextContent(/secret/i);
    expect(createRequest).not.toHaveBeenCalled();
  });

  it("shows the no-incident state and requires both fields", async () => {
    const createRequest = vi.fn();
    const user = userEvent.setup();
    renderWithProviders(
      <HelpPage articlesOverride={[]} incidentsOverride={[]} createRequest={createRequest} />,
      { route: "/help" },
    );

    expect(screen.getByTestId("help-no-incidents")).toBeInTheDocument();

    await user.click(screen.getByTestId("help-submit"));
    expect(screen.getByTestId("help-errors")).toHaveTextContent(/required/i);
    expect(createRequest).not.toHaveBeenCalled();
  });

  it("names the field when the gateway refuses the request", async () => {
    const createRequest = vi.fn().mockRejectedValue(
      new ApiError({
        type: "about:blank",
        title: "Validation failed",
        status: 422,
        correlationId: "corr-422",
        errors: [{ field: "body", reason: "is too long" }],
      }),
    );
    const user = userEvent.setup();
    renderWithProviders(
      <HelpPage articlesOverride={[]} incidentsOverride={[]} createRequest={createRequest} />,
      { route: "/help" },
    );

    await user.type(screen.getByTestId("help-subject"), "Question");
    await user.type(screen.getByTestId("help-body"), "How do I export?");
    await user.click(screen.getByTestId("help-submit"));

    expect(await screen.findByTestId("help-errors")).toHaveTextContent("body: is too long");
  });
});
