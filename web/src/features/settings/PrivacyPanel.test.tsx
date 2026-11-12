import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { formatDateTime } from "../../i18n";
import { expectNoAxeViolations } from "../../test/axe";
import { problem, stubGateway } from "../../test/gateway";
import { renderWithProviders } from "../../test/render";
import type { ConsentHistoryView } from "../../api/types";
import PrivacyPanel from "./PrivacyPanel";
import { SettingsNav } from "./SettingsNav";

const occurredAt = "2026-10-01T09:00:00Z";

const history: ConsentHistoryView = {
  events: [
    {
      id: "event-2",
      seq: 2,
      kind: "analytics",
      action: "withdrawn",
      occurredAt,
      policyVersion: "2026-11-01.1",
    },
    {
      id: "event-1",
      seq: 1,
      kind: "terms",
      action: "granted",
      occurredAt: "2026-08-01T09:00:00Z",
      policyVersion: "2026-08-01.1",
    },
  ],
};

function renderPanel(ui = <PrivacyPanel />) {
  return renderWithProviders(ui, { route: "/settings/privacy" });
}

describe("PrivacyPanel", () => {
  it("lists the consent history and starts the unchanged export flow", async () => {
    const fetchMock = stubGateway({
      "GET /v1/me/consent": () => ({ body: history }),
      "POST /v1/exports": () => ({
        status: 202,
        body: {
          id: "export-1",
          status: "complete",
          progress: 100,
          downloadUrl: "https://files.example/export-1.zip",
          expiresAt: null,
        },
      }),
    });
    const user = userEvent.setup();
    renderPanel(<PrivacyPanel pollInterval={0} />);

    const table = await screen.findByTestId("privacy-consent");
    expect(table).toHaveTextContent(/analytics/i);
    expect(table).toHaveTextContent(/withdrawn/i);
    expect(table).toHaveTextContent(formatDateTime(occurredAt, "en", "UTC"));

    await user.click(screen.getByTestId("privacy-export"));

    expect(await screen.findByTestId("privacy-download")).toHaveAttribute(
      "href",
      "https://files.example/export-1.zip",
    );
    const deletionPosts = fetchMock.mock.calls.filter(([input, init]) => {
      const method = ((init as RequestInit | undefined)?.method ?? "GET").toUpperCase();
      return String(input).endsWith("/v1/privacy/deletions") && method === "POST";
    });
    expect(deletionPosts).toHaveLength(0);
  });

  it("links the danger zone to the delete page instead of starting a deletion here", async () => {
    stubGateway({ "GET /v1/me/consent": () => ({ body: history }) });
    renderPanel();

    const link = await screen.findByTestId("privacy-delete-link");
    expect(link).toHaveAttribute("href", "/settings/delete");
    expect(screen.queryByTestId("privacy-delete")).not.toBeInTheDocument();
  });

  it("reports a history failure without blanking the navigation", async () => {
    stubGateway({
      "GET /v1/me/consent": () => ({
        status: 503,
        body: problem(503, "server detail that must never be rendered"),
      }),
    });
    renderPanel(
      <>
        <SettingsNav />
        <PrivacyPanel />
      </>,
    );

    expect(await screen.findByTestId("privacy-consent-error")).toBeInTheDocument();
    expect(screen.getByTestId("settings-nav-privacy")).toHaveAttribute("href", "/settings/privacy");
    expect(screen.queryByText(/server detail/)).not.toBeInTheDocument();
  });

  it("renders the empty history as a sentence, not an empty table", async () => {
    stubGateway({ "GET /v1/me/consent": () => ({ body: { events: [] } }) });
    renderPanel();

    expect(await screen.findByTestId("privacy-no-consent")).toBeInTheDocument();
    expect(screen.queryByTestId("privacy-consent")).not.toBeInTheDocument();
  });

  it("has no automated accessibility violations", async () => {
    stubGateway({ "GET /v1/me/consent": () => ({ body: history }) });
    const { container } = renderPanel();

    await screen.findByTestId("privacy-consent");
    await expectNoAxeViolations(container);
  });
});
