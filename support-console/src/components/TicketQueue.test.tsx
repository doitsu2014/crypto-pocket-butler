import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { axe } from "jest-axe";
import { describe, expect, it } from "vitest";
import { problemDetails, renderWithQuery, ticketMetadata } from "../test/fixtures";
import { server } from "../test/server";
import { TicketQueue } from "./TicketQueue";

function renderQueue(grantId = "grant-1") {
  return renderWithQuery(<TicketQueue grantId={grantId} token="token-1" />);
}

describe("TicketQueue", () => {
  it("lists grant-scoped ticket metadata and never renders conversation or requester labels", async () => {
    let seenGrantId: string | null = null;
    server.use(
      http.get("*/v1/support/tickets", ({ request }) => {
        seenGrantId = new URL(request.url).searchParams.get("grantId");
        return HttpResponse.json({
          items: [
            {
              ...ticketMetadata,
              requesterLabel: "Customer A",
              conversation: [{ body: "the customer's private message" }],
            },
          ],
        });
      }),
    );
    renderQueue();

    expect(await screen.findByText("Sync status question")).toBeInTheDocument();
    expect(seenGrantId).toBe("grant-1");
    expect(screen.queryByText(/private message/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/Customer A/i)).not.toBeInTheDocument();
  });

  it("requires an active grant before reading the queue", () => {
    renderQueue("");

    expect(screen.getByTestId("ticket-queue-grant-required")).toHaveTextContent(
      /active support grant/i,
    );
    expect(screen.getByTestId("ticket-status-filter")).toBeDisabled();
  });

  it("filters the queue by status", async () => {
    const seen: Array<string | null> = [];
    server.use(
      http.get("*/v1/support/tickets", ({ request }) => {
        const url = new URL(request.url);
        seen.push(url.searchParams.get("status"));
        expect(url.searchParams.get("grantId")).toBe("grant-1");
        return HttpResponse.json({ items: [ticketMetadata] });
      }),
    );
    renderQueue();

    await userEvent.setup().selectOptions(screen.getByTestId("ticket-status-filter"), "open");

    await waitFor(() => expect(seen).toContain("open"));
  });

  it("shows an empty state", async () => {
    server.use(http.get("*/v1/support/tickets", () => HttpResponse.json({ items: [] })));
    renderQueue();

    expect(await screen.findByTestId("ticket-queue-empty")).toHaveTextContent(/no tickets/i);
  });

  it("clears the cache and shows access ended on a 403", async () => {
    server.use(
      http.get("*/v1/support/tickets", () =>
        HttpResponse.json(problemDetails(403, "Access ended"), {
          status: 403,
          headers: { "Content-Type": "application/problem+json" },
        }),
      ),
    );
    const { client } = renderQueue();
    client.setQueryData(["support", "tickets", "grant-1", ""], { items: [] });

    expect(await screen.findByRole("alert")).toHaveTextContent(/access ended/i);
    expect(client.getQueryData(["support", "tickets", "grant-1", ""])).toBeUndefined();
  });

  it("has no accessibility violations", async () => {
    server.use(http.get("*/v1/support/tickets", () => HttpResponse.json({ items: [] })));
    const { container } = renderQueue();
    await screen.findByTestId("ticket-queue-empty");
    expect((await axe(container)).violations).toEqual([]);
  });
});
