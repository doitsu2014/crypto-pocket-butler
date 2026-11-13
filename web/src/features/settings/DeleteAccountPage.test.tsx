import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { NO_RECORD_PROBLEM_TYPE } from "../../api/client";
import { formatDateTime } from "../../i18n";
import { expectNoAxeViolations } from "../../test/axe";
import { stubGateway, problem } from "../../test/gateway";
import { renderWithProviders } from "../../test/render";
import type { DeletionView } from "../../api/types";
import DeleteAccountPage from "./DeleteAccountPage";

const requestedAt = "2026-11-01T09:00:00Z";
const recoveryEndsAt = "2026-12-01T09:00:00Z";

const deletion: DeletionView = {
  deletionId: "deletion-1",
  status: "requested",
  requestedAt,
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

function renderPage() {
  return renderWithProviders(<DeleteAccountPage />, { route: "/settings/delete" });
}

describe("DeleteAccountPage", () => {
  it("shows the consequences and waits for the dialog before calling D3", async () => {
    let open = false;
    const fetchMock = stubGateway({
      "GET /v1/me/deletion-request": () =>
        open ? { body: deletion } : { status: 404, body: noRecord() },
      "POST /v1/privacy/deletions": () => {
        open = true;
        return { status: 202, body: deletion };
      },
    });
    const user = userEvent.setup();
    renderPage();

    expect(await screen.findByTestId("delete-consequences")).toHaveTextContent(/cannot be undone/i);
    await user.click(screen.getByTestId("settings-delete-open"));
    expect(screen.getByTestId("confirm-dialog")).toBeInTheDocument();
    expect(
      fetchMock.mock.calls.some(
        ([, init]) => ((init as RequestInit | undefined)?.method ?? "GET").toUpperCase() === "POST",
      ),
    ).toBe(false);

    await user.click(screen.getByTestId("confirm-accept"));

    expect(await screen.findByTestId("deletion-recovery-date")).toHaveTextContent(
      formatDateTime(recoveryEndsAt, "en", "UTC"),
    );
    expect(screen.queryByTestId("settings-delete-open")).not.toBeInTheDocument();
  });

  it("renders an existing request with its exact dates and a Restore action", async () => {
    const fetchMock = stubGateway({
      "GET /v1/me/deletion-request": () => ({
        body: {
          ...deletion,
          retainedCategories: [
            { category: "tax records", basis: "legal obligation" },
            { category: "audit trail", basis: "legitimate interest" },
          ],
        },
      }),
      "DELETE /v1/me/deletion-request": () => ({
        body: { ...deletion, status: "cancelled", cancelledAt: requestedAt },
      }),
    });
    const user = userEvent.setup();
    renderPage();

    const notice = await screen.findByTestId("deletion-open-request");
    expect(notice).toHaveTextContent(formatDateTime(requestedAt, "en", "UTC"));
    expect(screen.getByTestId("deletion-recovery-date")).toHaveTextContent(
      formatDateTime(recoveryEndsAt, "en", "UTC"),
    );
    expect(screen.getByTestId("deletion-retained")).toHaveTextContent(/tax records/);
    expect(screen.getByTestId("deletion-retained")).toHaveTextContent(/audit trail/);

    await user.click(screen.getByTestId("deletion-restore"));

    await waitFor(() =>
      expect(
        fetchMock.mock.calls.some(
          ([input, init]) =>
            String(input).endsWith("/v1/me/deletion-request") &&
            ((init as RequestInit | undefined)?.method ?? "GET").toUpperCase() === "DELETE",
        ),
      ).toBe(true),
    );
  });

  it("renders the policy sentence when nothing is retained", async () => {
    stubGateway({
      "GET /v1/me/deletion-request": () => ({ body: deletion }),
    });
    renderPage();

    expect(await screen.findByTestId("deletion-retained-policy")).toHaveTextContent(
      /exact list appears here/i,
    );
  });

  it("re-reads D1 on a 409 and renders the open request, not an error", async () => {
    let reads = 0;
    stubGateway({
      "GET /v1/me/deletion-request": () => {
        reads += 1;
        return reads === 1 ? { status: 404, body: noRecord() } : { body: deletion };
      },
      "POST /v1/privacy/deletions": () => ({
        status: 409,
        body: problem(409, "a request is already open"),
      }),
    });
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByTestId("settings-delete-open"));
    await user.click(screen.getByTestId("confirm-accept"));

    expect(await screen.findByTestId("deletion-open-request")).toBeInTheDocument();
    expect(screen.queryByTestId("delete-action-error")).not.toBeInTheDocument();
    expect(reads).toBeGreaterThanOrEqual(2);
  });

  it("re-reads D1 when the deletion answer is lost", async () => {
    let reads = 0;
    stubGateway({
      "GET /v1/me/deletion-request": () => {
        reads += 1;
        return reads === 1 ? { status: 404, body: noRecord() } : { body: deletion };
      },
      "POST /v1/privacy/deletions": () => {
        throw new Error("connection lost before the answer arrived");
      },
    });
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByTestId("settings-delete-open"));
    await user.click(screen.getByTestId("confirm-accept"));

    expect(await screen.findByTestId("deletion-open-request")).toBeInTheDocument();
    expect(screen.queryByTestId("delete-action-error")).not.toBeInTheDocument();
    expect(reads).toBeGreaterThanOrEqual(2);
  });

  it("has no automated accessibility violations", async () => {
    stubGateway({
      "GET /v1/me/deletion-request": () => ({ status: 404, body: noRecord() }),
    });
    const { container } = renderPage();

    await screen.findByTestId("delete-consequences");
    await expectNoAxeViolations(container);
  });
});
