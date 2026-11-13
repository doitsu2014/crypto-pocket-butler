import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { axe } from "jest-axe";
import { describe, expect, it, vi } from "vitest";
import type { AdminUserRecord } from "../../api/types";
import { problemDetails, renderWithQuery } from "../../test/fixtures";
import { server } from "../../test/server";
import { StatusChangePanel } from "./StatusChangePanel";

function record(overrides: Partial<AdminUserRecord> = {}): AdminUserRecord {
  return {
    id: "user-1",
    displayName: "Test User",
    email: "user@example.com",
    emailVerified: true,
    status: "active",
    role: "user" as unknown as AdminUserRecord["role"],
    createdAt: "2026-11-01T10:00:00.000Z",
    updatedAt: "2026-11-02T10:00:00.000Z",
    ...overrides,
  };
}

function renderPanel(overrides: Partial<AdminUserRecord> = {}) {
  return renderWithQuery(
    <StatusChangePanel record={record(overrides)} token="admin-token" onChanged={vi.fn()} />,
  );
}

function problem(status: number, title: string, correlationId = "corr-status") {
  return HttpResponse.json(problemDetails(status, title, correlationId), {
    status,
    headers: { "Content-Type": "application/problem+json" },
  });
}

describe("StatusChangePanel", () => {
  it("confirms before disabling and sends nothing until Confirm", async () => {
    const user = userEvent.setup();
    let calls = 0;
    let seenBody: unknown;
    const onChanged = vi.fn();
    server.use(
      http.post("*/v1/admin/users/user-1/disable", async ({ request }) => {
        calls += 1;
        seenBody = await request.json();
        return HttpResponse.json({
          userId: "user-1",
          status: "disabled",
          transitionId: "transition-1",
          occurredAt: "2026-11-02T10:05:00.000Z",
        });
      }),
    );
    renderWithQuery(
      <StatusChangePanel record={record()} token="admin-token" onChanged={onChanged} />,
    );

    await user.type(screen.getByTestId("admin-status-reason"), "Account abuse");
    await user.click(screen.getByTestId("admin-status-action"));

    expect(screen.getByTestId("admin-status-confirm")).toHaveTextContent(
      "The user cannot sign in while the account is disabled. History is not deleted.",
    );
    expect(calls).toBe(0);

    await user.click(screen.getByTestId("admin-status-confirm-button"));

    await waitFor(() =>
      expect(screen.getByTestId("admin-status-result")).toHaveTextContent("disabled"),
    );
    expect(screen.getByTestId("admin-status-result")).toHaveTextContent("This change is audited.");
    expect(seenBody).toEqual({ reason: "Account abuse" });
    await waitFor(() => expect(onChanged).toHaveBeenCalledTimes(1));
  });

  it("cancels the confirmation without sending anything and returns focus", async () => {
    const user = userEvent.setup();
    let calls = 0;
    server.use(
      http.post("*/v1/admin/users/user-1/disable", () => {
        calls += 1;
        return HttpResponse.json({});
      }),
    );
    renderPanel();

    await user.type(screen.getByTestId("admin-status-reason"), "Account abuse");
    await user.click(screen.getByTestId("admin-status-action"));
    await user.click(screen.getByTestId("admin-status-cancel"));

    expect(screen.queryByTestId("admin-status-confirm")).not.toBeInTheDocument();
    expect(calls).toBe(0);
    expect(screen.getByTestId("admin-status-action")).toHaveFocus();
  });

  it("cancels on Escape without sending anything", async () => {
    const user = userEvent.setup();
    let calls = 0;
    server.use(
      http.post("*/v1/admin/users/user-1/disable", () => {
        calls += 1;
        return HttpResponse.json({});
      }),
    );
    renderPanel();

    await user.type(screen.getByTestId("admin-status-reason"), "Account abuse");
    await user.click(screen.getByTestId("admin-status-action"));
    await user.keyboard("{Escape}");

    expect(screen.queryByTestId("admin-status-confirm")).not.toBeInTheDocument();
    expect(calls).toBe(0);
    expect(screen.getByTestId("admin-status-action")).toHaveFocus();
  });

  it("reports a 409 as the state having moved and never claims success", async () => {
    const user = userEvent.setup();
    server.use(
      http.post("*/v1/admin/users/user-1/disable", () => problem(409, "Conflict", "corr-409")),
    );
    renderPanel();

    await user.type(screen.getByTestId("admin-status-reason"), "Account abuse");
    await user.click(screen.getByTestId("admin-status-action"));
    await user.click(screen.getByTestId("admin-status-confirm-button"));

    const alert = await screen.findByTestId("admin-status-error");
    expect(alert).toHaveTextContent("corr-409");
    expect(screen.queryByTestId("admin-status-result")).not.toBeInTheDocument();
  });

  it("offers restore only for a disabled account", () => {
    const { unmount } = renderPanel({ status: "active" });
    expect(screen.getByTestId("admin-status-action")).toHaveTextContent("Disable account");
    unmount();

    renderPanel({ status: "disabled" });
    expect(screen.getByTestId("admin-status-action")).toHaveTextContent("Restore account");
  });

  it("explains a pending-deletion or erased account and offers no action", () => {
    const { unmount } = renderPanel({ status: "pending_deletion" });
    expect(screen.getByTestId("admin-status-explanatory")).toBeInTheDocument();
    expect(screen.queryByTestId("admin-status-action")).not.toBeInTheDocument();
    unmount();

    renderPanel({ status: "deleted" });
    expect(screen.getByTestId("admin-status-explanatory")).toBeInTheDocument();
    expect(screen.queryByTestId("admin-status-action")).not.toBeInTheDocument();
  });

  it("has no accessibility violations", async () => {
    const { container } = renderPanel();
    expect((await axe(container)).violations).toEqual([]);
  });
});
