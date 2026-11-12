import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { axe } from "jest-axe";
import { describe, expect, it, vi } from "vitest";
import {
  activeGrant,
  pendingGrant,
  problemDetails,
  renderWithQuery,
  supportMetadata,
} from "../test/fixtures";
import { server } from "../test/server";
import { SupportMetadataPanel } from "./SupportMetadataPanel";

const METADATA = "*/v1/support-grants/*/metadata";

function problemResponse(status: number, title: string, correlationId = "corr-1") {
  return HttpResponse.json(problemDetails(status, title, correlationId), {
    status,
    headers: { "Content-Type": "application/problem+json" },
  });
}

function renderPanel(
  overrides: { status?: "active" | "pending_approval"; onAccessEnded?: () => void } = {},
) {
  const onAccessEnded = overrides.onAccessEnded ?? vi.fn();
  const result = renderWithQuery(
    <SupportMetadataPanel
      grant={activeGrant()}
      token="token-1"
      status={overrides.status ?? "active"}
      onAccessEnded={onAccessEnded}
    />,
  );
  return { ...result, onAccessEnded };
}

describe("SupportMetadataPanel", () => {
  it("renders the user, the accounts table, and the portfolios from one read", async () => {
    let requests = 0;
    server.use(
      http.get(METADATA, () => {
        requests += 1;
        return HttpResponse.json(supportMetadata());
      }),
    );
    renderPanel();

    expect(await screen.findByText("Customer One")).toBeInTheDocument();
    expect(screen.getByTestId("metadata-user")).toHaveTextContent("user-1");
    expect(screen.getByTestId("metadata-user")).toHaveTextContent(/verified/i);

    const accounts = screen.getByTestId("metadata-accounts");
    expect(within(accounts).getByText("Main Binance")).toBeInTheDocument();
    expect(within(accounts).getByText("Cold wallet")).toBeInTheDocument();
    expect(within(accounts).getByText("exchange")).toBeInTheDocument();
    expect(within(accounts).getAllByText("—").length).toBeGreaterThan(0);

    expect(screen.getByTestId("metadata-portfolios")).toHaveTextContent("Long term");
    expect(requests).toBe(1);
  });

  it("states that every metadata read is audited", async () => {
    server.use(http.get(METADATA, () => HttpResponse.json(supportMetadata())));
    renderPanel();

    expect(await screen.findByText(/every metadata read is recorded/i)).toBeInTheDocument();
  });

  it("shows both empty states when the projection is legitimate but empty", async () => {
    server.use(
      http.get(METADATA, () =>
        HttpResponse.json(supportMetadata({ accounts: [], portfolios: [], displayName: null })),
      ),
    );
    renderPanel();

    expect(await screen.findByTestId("metadata-accounts-empty")).toHaveTextContent(
      /no accounts to show/i,
    );
    expect(screen.getByTestId("metadata-portfolios-empty")).toHaveTextContent(
      /no portfolios to show/i,
    );
    expect(screen.queryByText("Customer One")).not.toBeInTheDocument();
  });

  it("narrows the filter without issuing another request", async () => {
    let requests = 0;
    server.use(
      http.get(METADATA, () => {
        requests += 1;
        return HttpResponse.json(supportMetadata());
      }),
    );
    renderPanel();
    await screen.findByText("Main Binance");

    const user = userEvent.setup();
    await user.type(screen.getByTestId("metadata-filter"), "cold");

    const accounts = screen.getByTestId("metadata-accounts");
    expect(within(accounts).getByText("Cold wallet")).toBeInTheDocument();
    expect(within(accounts).queryByText("Main Binance")).not.toBeInTheDocument();
    expect(requests).toBe(1);
  });

  it("shows access ended and reports it on a 403", async () => {
    server.use(http.get(METADATA, () => problemResponse(403, "Access ended")));
    const { onAccessEnded } = renderPanel();

    expect(await screen.findByText(/access ended/i)).toBeInTheDocument();
    await waitFor(() => expect(onAccessEnded).toHaveBeenCalledTimes(1));
  });

  it("asks for a sign-in on a 401", async () => {
    server.use(http.get(METADATA, () => problemResponse(401, "Sign in again")));
    renderPanel();

    expect(await screen.findByText(/session has expired/i)).toBeInTheDocument();
    expect(screen.getByText(/sign in again/i)).toBeInTheDocument();
  });

  it("offers Refresh on a 503 and recovers on the retry", async () => {
    let requests = 0;
    server.use(
      http.get(METADATA, () => {
        requests += 1;
        return requests === 1
          ? problemResponse(503, "Service unavailable")
          : HttpResponse.json(supportMetadata());
      }),
    );
    renderPanel();

    expect(await screen.findByRole("alert")).toHaveTextContent(/could not load/i);
    const user = userEvent.setup();
    await user.click(screen.getByTestId("metadata-refresh"));

    expect(await screen.findByText("Customer One")).toBeInTheDocument();
    expect(requests).toBe(2);
  });

  it("makes no read while the clock-derived status is not active", async () => {
    let requests = 0;
    server.use(
      http.get(METADATA, () => {
        requests += 1;
        return HttpResponse.json(supportMetadata());
      }),
    );
    renderWithQuery(
      <SupportMetadataPanel
        grant={pendingGrant()}
        token="token-1"
        status="pending_approval"
        onAccessEnded={vi.fn()}
      />,
    );

    expect(await screen.findByText(/available while a grant is active/i)).toBeInTheDocument();
    expect(requests).toBe(0);
  });

  it("has no accessibility violations in the populated, empty, and error states", async () => {
    server.use(http.get(METADATA, () => HttpResponse.json(supportMetadata())));
    const populated = renderPanel();
    await screen.findByText("Customer One");
    expect((await axe(populated.container)).violations).toEqual([]);
    populated.unmount();

    server.use(
      http.get(METADATA, () =>
        HttpResponse.json(supportMetadata({ accounts: [], portfolios: [] })),
      ),
    );
    const empty = renderPanel();
    await screen.findByTestId("metadata-accounts-empty");
    expect((await axe(empty.container)).violations).toEqual([]);
    empty.unmount();

    server.use(http.get(METADATA, () => problemResponse(503, "Service unavailable")));
    const failed = renderPanel();
    await screen.findByRole("alert");
    expect((await axe(failed.container)).violations).toEqual([]);
  });
});
