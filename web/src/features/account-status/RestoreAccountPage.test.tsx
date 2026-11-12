import { Route, Routes } from "react-router-dom";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { NO_RECORD_PROBLEM_TYPE } from "../../api/client";
import { formatDateTime } from "../../i18n";
import { expectNoAxeViolations } from "../../test/axe";
import { problem, stubGateway } from "../../test/gateway";
import { renderWithProviders } from "../../test/render";
import type { DeletionView } from "../../api/types";
import RestoreAccountPage from "./RestoreAccountPage";

const recoveryEndsAt = "2026-12-01T09:00:00Z";

const deletion: DeletionView = {
  deletionId: "deletion-1",
  status: "requested",
  requestedAt: "2026-11-01T09:00:00Z",
  recoveryEndsAt,
  retainedCategories: [],
  cancelledAt: null,
  erasedAt: null,
  attemptCount: 0,
};

function noRecord() {
  return {
    type: NO_RECORD_PROBLEM_TYPE,
    title: "No record",
    status: 404,
    correlationId: "corr-404",
  };
}

function renderRestore(onSignOut = () => {}) {
  return renderWithProviders(
    <Routes>
      <Route path="/account/restore" element={<RestoreAccountPage onSignOut={onSignOut} />} />
      <Route path="/settings/profile" element={<p>the profile page</p>} />
      <Route path="/account/blocked" element={<p>the blocked page</p>} />
    </Routes>,
    { route: "/account/restore" },
  );
}

describe("RestoreAccountPage", () => {
  it("shows the exact recovery date and restores through D2", async () => {
    stubGateway({
      "GET /v1/me/deletion-request": () => ({ body: deletion }),
      "DELETE /v1/me/deletion-request": () => ({
        body: { ...deletion, status: "cancelled", cancelledAt: deletion.requestedAt },
      }),
    });
    const user = userEvent.setup();
    renderRestore();

    expect(await screen.findByTestId("recovery-banner")).toHaveTextContent(
      formatDateTime(recoveryEndsAt, "en", "UTC"),
    );
    await user.click(screen.getByTestId("restore-account"));

    expect(await screen.findByText("the profile page")).toBeInTheDocument();
  });

  it("renders the terminal window-closed state on 410 with no restore control", async () => {
    stubGateway({
      "GET /v1/me/deletion-request": () => ({ body: deletion }),
      "DELETE /v1/me/deletion-request": () => ({
        status: 410,
        body: problem(410, "server detail that must never be rendered"),
      }),
    });
    const user = userEvent.setup();
    renderRestore();

    await user.click(await screen.findByTestId("restore-account"));

    expect(await screen.findByTestId("restore-closed")).toHaveTextContent(
      formatDateTime(recoveryEndsAt, "en", "UTC"),
    );
    expect(screen.queryByTestId("restore-account")).not.toBeInTheDocument();
    expect(screen.getByTestId("restore-sign-out")).toBeInTheDocument();
    expect(screen.queryByText(/server detail/)).not.toBeInTheDocument();
  });

  it("re-resolves the account status when no request remains", async () => {
    stubGateway({
      "GET /v1/me/deletion-request": () => ({ status: 404, body: noRecord() }),
      "GET /v1/me/account-status": () => ({ body: { status: "active" } }),
    });
    renderRestore();

    expect(await screen.findByText("the profile page")).toBeInTheDocument();
  });

  it("routes a re-resolved disabled account to the blocked screen", async () => {
    stubGateway({
      "GET /v1/me/deletion-request": () => ({ status: 404, body: noRecord() }),
      "GET /v1/me/account-status": () => ({ body: { status: "disabled" } }),
    });
    renderRestore();

    expect(await screen.findByText("the blocked page")).toBeInTheDocument();
  });

  it("signs out without touching the request", async () => {
    const onSignOut = vi.fn();
    const fetchMock = stubGateway({
      "GET /v1/me/deletion-request": () => ({ body: deletion }),
    });
    const user = userEvent.setup();
    renderRestore(onSignOut);

    await user.click(await screen.findByTestId("restore-sign-out"));

    expect(onSignOut).toHaveBeenCalledTimes(1);
    expect(
      fetchMock.mock.calls.some(
        ([, init]) =>
          ((init as RequestInit | undefined)?.method ?? "GET").toUpperCase() === "DELETE",
      ),
    ).toBe(false);
  });

  it("re-reads D1 when the restore answer fails", async () => {
    let reads = 0;
    stubGateway({
      "GET /v1/me/deletion-request": () => {
        reads += 1;
        return reads === 1 ? { body: deletion } : { status: 404, body: noRecord() };
      },
      "GET /v1/me/account-status": () => ({ body: { status: "active" } }),
      "DELETE /v1/me/deletion-request": () => ({
        status: 503,
        body: problem(503, "outage"),
      }),
    });
    const user = userEvent.setup();
    renderRestore();

    await user.click(await screen.findByTestId("restore-account"));

    expect(await screen.findByText("the profile page")).toBeInTheDocument();
    expect(screen.queryByTestId("restore-action-error")).not.toBeInTheDocument();
    expect(reads).toBeGreaterThanOrEqual(2);
  });

  it("has no automated accessibility violations", async () => {
    stubGateway({ "GET /v1/me/deletion-request": () => ({ body: deletion }) });
    const { container } = renderRestore();

    await screen.findByTestId("recovery-banner");
    await expectNoAxeViolations(container);
  });
});
