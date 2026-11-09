import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import PrivacyPage from "./PrivacyPage";
import { ApiError } from "../../api/client";
import { renderWithProviders } from "../../test/render";
import type { ConsentRecord, ExportJob } from "../../api/types";

const consent: ConsentRecord[] = [
  {
    purpose: "analytics",
    granted: true,
    legalBasis: "consent",
    recordedAt: "2026-11-01T09:00:00Z",
  },
  {
    purpose: "marketing",
    granted: false,
    legalBasis: "consent",
    recordedAt: "2026-11-01T09:00:00Z",
  },
];

const completeExport: ExportJob = {
  id: "export-1",
  status: "complete",
  progress: 100,
  downloadUrl: "/v1/exports/export-1/download",
  expiresAt: "2026-11-10T09:00:00Z",
};

describe("privacy", () => {
  it("renders consent history and offers a completed export", async () => {
    const startExport = vi.fn().mockResolvedValue(completeExport);
    const user = userEvent.setup();
    renderWithProviders(
      <PrivacyPage consentOverride={consent} startExport={startExport} pollInterval={0} />,
      { route: "/privacy" },
    );

    expect(screen.getByTestId("privacy-consent")).toHaveTextContent("analytics");
    expect(screen.getByTestId("privacy-consent")).toHaveTextContent("consent");

    await user.click(screen.getByTestId("privacy-export"));

    expect(startExport).toHaveBeenCalledOnce();
    expect(await screen.findByTestId("privacy-export-progress")).toHaveTextContent(/complete/i);
    expect(screen.getByTestId("privacy-download")).toHaveAttribute(
      "href",
      "/v1/exports/export-1/download",
    );
  });

  it("requires confirmation before requesting deletion", async () => {
    const requestDeletion = vi.fn().mockResolvedValue({
      id: "del-1",
      status: "requested",
      requestedAt: "2026-11-08T12:00:00Z",
    });
    const user = userEvent.setup();
    renderWithProviders(<PrivacyPage consentOverride={[]} requestDeletion={requestDeletion} />, {
      route: "/privacy",
    });

    await user.click(screen.getByTestId("privacy-delete"));
    expect(requestDeletion).not.toHaveBeenCalled();
    expect(screen.getByTestId("confirm-dialog")).toBeInTheDocument();

    await user.click(screen.getByTestId("confirm-accept"));

    expect(requestDeletion).toHaveBeenCalledOnce();
    expect(await screen.findByTestId("privacy-deletion-status")).toHaveTextContent("requested");
  });

  it("shows the empty consent state and surfaces an export failure", async () => {
    const startExport = vi.fn().mockRejectedValue(
      new ApiError({
        type: "about:blank",
        title: "Service unavailable",
        status: 500,
        correlationId: "corr-500",
      }),
    );
    const user = userEvent.setup();
    renderWithProviders(
      <PrivacyPage consentOverride={[]} startExport={startExport} pollInterval={0} />,
      { route: "/privacy" },
    );

    expect(screen.getByTestId("privacy-no-consent")).toBeInTheDocument();

    await user.click(screen.getByTestId("privacy-export"));
    expect(await screen.findByTestId("privacy-errors")).toBeInTheDocument();
  });

  it("keeps the deletion dialog open and reports a refusal", async () => {
    const requestDeletion = vi.fn().mockRejectedValue(
      new ApiError({
        type: "about:blank",
        title: "Conflict",
        status: 409,
        detail: "a deletion is already in progress",
        correlationId: "corr-409",
      }),
    );
    const user = userEvent.setup();
    renderWithProviders(<PrivacyPage consentOverride={[]} requestDeletion={requestDeletion} />, {
      route: "/privacy",
    });

    await user.click(screen.getByTestId("privacy-delete"));
    await user.click(screen.getByTestId("confirm-accept"));

    expect(await screen.findByTestId("privacy-errors")).toHaveTextContent(/conflict/i);
  });
});
