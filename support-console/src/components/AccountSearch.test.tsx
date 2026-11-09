import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { axe } from "jest-axe";
import { describe, expect, it, vi } from "vitest";
import { accountMetadata, problemDetails, renderWithQuery } from "../test/fixtures";
import { server } from "../test/server";
import { AccountSearch } from "./AccountSearch";

function renderSearch(grantId = "grant-1") {
  const onSelect = vi.fn();
  const rendered = renderWithQuery(
    <AccountSearch grantId={grantId} token="token-1" onSelect={onSelect} />,
  );
  return { ...rendered, onSelect };
}

describe("AccountSearch", () => {
  it("searches within the grant and opens a selected account", async () => {
    const user = userEvent.setup();
    let seenGrantId: string | null = null;
    let seenQuery: string | null = null;
    server.use(
      http.get("*/v1/support/accounts", ({ request }) => {
        const url = new URL(request.url);
        seenGrantId = url.searchParams.get("grantId");
        seenQuery = url.searchParams.get("query");
        return HttpResponse.json({ items: [accountMetadata], nextCursor: null });
      }),
    );
    const { onSelect } = renderSearch();

    await user.type(screen.getByTestId("account-search-query"), "binance");
    await user.click(screen.getByTestId("account-search-submit"));

    expect(await screen.findByText("Main Binance")).toBeInTheDocument();
    expect(seenGrantId).toBe("grant-1");
    expect(seenQuery).toBe("binance");

    await user.click(screen.getByTestId(`account-search-open-${accountMetadata.id}`));
    expect(onSelect).toHaveBeenCalledWith(accountMetadata.id);
  });

  it("refuses to search without an active grant and makes no request", async () => {
    renderSearch("");

    expect(screen.getByTestId("account-search-grant-required")).toHaveTextContent(
      /active support grant/i,
    );
    expect(screen.getByTestId("account-search-query")).toBeDisabled();
    expect(screen.getByTestId("account-search-submit")).toBeDisabled();
  });

  it("clears the cache and shows access ended on a 403", async () => {
    server.use(
      http.get("*/v1/support/accounts", () =>
        HttpResponse.json(problemDetails(403, "Access ended"), {
          status: 403,
          headers: { "Content-Type": "application/problem+json" },
        }),
      ),
    );
    const { client } = renderSearch();
    client.setQueryData(["support", "accounts"], { items: [] });

    await userEvent.setup().click(screen.getByTestId("account-search-submit"));

    expect(await screen.findByRole("alert")).toHaveTextContent(/access ended/i);
    expect(client.getQueryData(["support", "accounts"])).toBeUndefined();
  });

  it("shows a sign-in message on a 401", async () => {
    server.use(
      http.get("*/v1/support/accounts", () =>
        HttpResponse.json(problemDetails(401, "Unauthorized"), {
          status: 401,
          headers: { "Content-Type": "application/problem+json" },
        }),
      ),
    );
    renderSearch();

    await userEvent.setup().click(screen.getByTestId("account-search-submit"));

    expect(await screen.findByRole("alert")).toHaveTextContent(/sign in again/i);
  });

  it("shows an empty state when the grant has no matching accounts", async () => {
    server.use(
      http.get("*/v1/support/accounts", () =>
        HttpResponse.json({ items: [], nextCursor: null }),
      ),
    );
    renderSearch();

    await userEvent.setup().click(screen.getByTestId("account-search-submit"));

    expect(await screen.findByTestId("account-search-empty")).toHaveTextContent(
      /no accounts found/i,
    );
  });

  it("has no accessibility violations", async () => {
    const { container } = renderWithQuery(
      <AccountSearch grantId="grant-1" token="token-1" onSelect={vi.fn()} />,
    );
    expect((await axe(container)).violations).toEqual([]);
  });
});
