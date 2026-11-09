import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { axe } from "jest-axe";
import { describe, expect, it, vi } from "vitest";
import { accountMetadata, problemDetails, renderWithQuery } from "../test/fixtures";
import { server } from "../test/server";
import { AccountMetadataPanel } from "./AccountMetadataPanel";

const REASON = "Investigating a reported sync failure";

function renderPanel(overrides: Partial<Parameters<typeof AccountMetadataPanel>[0]> = {}) {
  const onAccessEnded = vi.fn();
  const rendered = renderWithQuery(
    <AccountMetadataPanel
      accountId="acct-1"
      grantId="grant-1"
      token="token-1"
      defaultReason={REASON}
      onAccessEnded={onAccessEnded}
      {...overrides}
    />,
  );
  return { ...rendered, onAccessEnded };
}

describe("AccountMetadataPanel", () => {
  it("loads metadata under an active grant and never shows credentials or holdings", async () => {
    const user = userEvent.setup();
    server.use(
      http.get("*/v1/support/accounts/*/metadata", ({ request }) => {
        const url = new URL(request.url);
        expect(url.searchParams.get("reason")).toBeNull();
        expect(request.headers.get("X-Support-Reason")).toBe(REASON);
        return HttpResponse.json({
          ...accountMetadata,
          credentials: { apiKey: "never-render" },
          holdings: [{ asset: "BTC", quantity: "1" }],
          valuation: "1000.00",
        });
      }),
    );
    renderPanel();

    await user.click(screen.getByTestId("account-metadata-load"));

    const panel = await screen.findByTestId("account-metadata-panel");
    expect(panel).toHaveTextContent("Main Binance");
    expect(panel).toHaveTextContent("binance");
    expect(panel).toHaveTextContent("active");
    expect(screen.queryByText("never-render")).not.toBeInTheDocument();
    expect(panel).not.toHaveTextContent(/BTC|1000\.00/);
  });

  it("requires a reason of at least 20 characters before requesting", async () => {
    renderPanel({ defaultReason: "" });

    await userEvent.setup().click(screen.getByTestId("account-metadata-load"));

    expect(await screen.findByText(/at least 20 characters/i)).toBeInTheDocument();
  });

  it("shows access ended with the expiry and a new-grant link on a 403", async () => {
    server.use(
      http.get("*/v1/support/accounts/*/metadata", () =>
        HttpResponse.json(problemDetails(403, "Grant expired"), {
          status: 403,
          headers: { "Content-Type": "application/problem+json" },
        }),
      ),
    );
    const { client, onAccessEnded } = renderPanel({
      expiresAt: "2026-11-08T10:00:00.000Z",
    });
    client.setQueryData(["support", "metadata", "acct-1"], accountMetadata);

    await userEvent.setup().click(screen.getByTestId("account-metadata-load"));

    const alert = await screen.findByTestId("account-metadata-access-ended");
    expect(alert).toHaveTextContent(/access ended/i);
    expect(alert).toHaveTextContent(/2026/);
    expect(screen.getByTestId("account-metadata-request-new")).toHaveAttribute("href", "#grants");
    expect(onAccessEnded).toHaveBeenCalled();
    expect(client.getQueryData(["support", "metadata", "acct-1"])).toBeUndefined();
  });

  it("shows a sign-in message on a 401", async () => {
    server.use(
      http.get("*/v1/support/accounts/*/metadata", () =>
        HttpResponse.json(problemDetails(401, "Unauthorized"), {
          status: 401,
          headers: { "Content-Type": "application/problem+json" },
        }),
      ),
    );
    renderPanel();

    await userEvent.setup().click(screen.getByTestId("account-metadata-load"));

    expect(await screen.findByRole("alert")).toHaveTextContent(/sign in again/i);
  });

  it("renders a missing last-synced time as an em dash, never zero", async () => {
    server.use(
      http.get("*/v1/support/accounts/*/metadata", () =>
        HttpResponse.json({
          id: "acct-2",
          kind: "wallet",
          label: "Cold wallet",
          status: "active",
        }),
      ),
    );
    renderPanel();

    await userEvent.setup().click(screen.getByTestId("account-metadata-load"));

    const synced = await screen.findByTestId("account-metadata-last-synced");
    expect(synced).toHaveTextContent("—");
    expect(synced).not.toHaveTextContent(/^0$/);
  });

  it("shows a network failure", async () => {
    server.use(http.get("*/v1/support/accounts/*/metadata", () => HttpResponse.error()));
    renderPanel();

    await userEvent.setup().click(screen.getByTestId("account-metadata-load"));

    expect(await screen.findByRole("alert")).toHaveTextContent(/gateway/i);
  });

  it("has no accessibility violations", async () => {
    const { container } = renderPanel();
    expect((await axe(container)).violations).toEqual([]);
  });
});
