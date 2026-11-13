import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { axe } from "jest-axe";
import { describe, expect, it, vi } from "vitest";
import { problemDetails, pendingGrant, renderWithQuery } from "../../test/fixtures";
import { server } from "../../test/server";
import { AdminApprovals } from "./AdminApprovals";

function renderApprovals(onChanged = vi.fn()) {
  return renderWithQuery(
    <AdminApprovals
      currentUserId="admin-1"
      role="admin"
      token="admin-token"
      onChanged={onChanged}
    />,
  );
}

describe("AdminApprovals", () => {
  it("lists the pending grants it reads, not an empty queue", async () => {
    server.use(
      http.get("*/v1/support-grants", () =>
        HttpResponse.json([pendingGrant({ id: "grant-pending", requestedBy: "agent-1" })]),
      ),
    );
    renderApprovals();

    expect(await screen.findByTestId("grant-approval-row-grant-pending")).toBeInTheDocument();
    expect(screen.queryByTestId("grant-approval-empty")).not.toBeInTheDocument();
  });

  it("never renders an empty list on a failure", async () => {
    server.use(
      http.get("*/v1/support-grants", () =>
        HttpResponse.json(problemDetails(503, "Service unavailable", "corr-grants"), {
          status: 503,
          headers: { "Content-Type": "application/problem+json" },
        }),
      ),
    );
    renderApprovals();

    const alert = await screen.findByTestId("admin-approvals-error");
    expect(alert).toHaveTextContent("corr-grants");
    expect(screen.queryByTestId("grant-approval-empty")).not.toBeInTheDocument();
    expect(screen.queryByTestId("grant-approval-queue")).not.toBeInTheDocument();
  });

  it("retries the pending-grant read after a failure", async () => {
    const user = userEvent.setup();
    let reads = 0;
    server.use(
      http.get("*/v1/support-grants", () => {
        reads += 1;
        if (reads === 1) {
          return HttpResponse.json(problemDetails(503, "Service unavailable", "corr-retry"), {
            status: 503,
            headers: { "Content-Type": "application/problem+json" },
          });
        }
        return HttpResponse.json([pendingGrant({ id: "grant-retried" })]);
      }),
    );
    renderApprovals();

    await user.click(await screen.findByTestId("admin-approvals-retry"));

    expect(await screen.findByTestId("grant-approval-row-grant-retried")).toBeInTheDocument();
    expect(screen.queryByTestId("admin-approvals-error")).not.toBeInTheDocument();
  });

  it("approves, re-reads the queue and reports the change", async () => {
    const user = userEvent.setup();
    const onChanged = vi.fn();
    let reads = 0;
    server.use(
      http.get("*/v1/support-grants", () => {
        reads += 1;
        return HttpResponse.json(
          reads === 1 ? [pendingGrant({ id: "grant-1", requestedBy: "agent-1" })] : [],
        );
      }),
      http.post("*/v1/support-grants/grant-1/approve", () =>
        HttpResponse.json({
          ...pendingGrant({ id: "grant-1", requestedBy: "agent-1" }),
          status: "active",
          approvedBy: "admin-1",
        }),
      ),
    );
    renderApprovals(onChanged);

    await user.click(await screen.findByTestId("grant-approval-approve-grant-1"));

    await waitFor(() => expect(onChanged).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(reads).toBeGreaterThanOrEqual(2));
    await waitFor(() =>
      expect(screen.queryByTestId("grant-approval-row-grant-1")).not.toBeInTheDocument(),
    );
  });

  it("revokes, re-reads the queue and reports the change", async () => {
    const user = userEvent.setup();
    const onChanged = vi.fn();
    let reads = 0;
    server.use(
      http.get("*/v1/support-grants", () => {
        reads += 1;
        return HttpResponse.json(
          reads === 1 ? [pendingGrant({ id: "grant-2", requestedBy: "agent-9" })] : [],
        );
      }),
      http.delete("*/v1/support-grants/grant-2", () => new HttpResponse(null, { status: 204 })),
    );
    renderApprovals(onChanged);

    await user.click(await screen.findByTestId("grant-approval-revoke-grant-2"));

    await waitFor(() => expect(onChanged).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(reads).toBeGreaterThanOrEqual(2));
  });

  it("has no accessibility violations in its failure state", async () => {
    server.use(
      http.get("*/v1/support-grants", () =>
        HttpResponse.json(problemDetails(503, "Service unavailable", "corr-axe"), {
          status: 503,
          headers: { "Content-Type": "application/problem+json" },
        }),
      ),
    );
    const { container } = renderApprovals();

    await screen.findByTestId("admin-approvals-error");
    expect((await axe(container)).violations).toEqual([]);
  });
});
