import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import type { ReactNode } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { axe } from "jest-axe";
import { describe, expect, it, vi } from "vitest";
import { activeGrant, pendingGrant, problemDetails } from "../test/fixtures";
import { renderWithQuery } from "../test/fixtures";
import { server } from "../test/server";
import { GrantApprovalQueue } from "./GrantApprovalQueue";

const ownGrant = pendingGrant({ id: "grant-own", requestedBy: "agent-1" });
const otherGrant = pendingGrant({ id: "grant-other", requestedBy: "agent-2" });

describe("GrantApprovalQueue", () => {
  it("shows an empty state when there are no pending grants", () => {
    renderWithQuery(
      <GrantApprovalQueue
        grants={[]}
        currentUserId="agent-1"
        role="support"
        token="token-1"
        onChanged={vi.fn()}
      />,
    );

    expect(screen.getByTestId("grant-approval-empty")).toHaveTextContent(/no pending grants/i);
  });

  it("lists pending grants with their reason, target, and status", () => {
    renderWithQuery(
      <GrantApprovalQueue
        grants={[otherGrant]}
        currentUserId="agent-1"
        role="support"
        token="token-1"
        onChanged={vi.fn()}
      />,
    );

    const row = screen.getByTestId(`grant-approval-row-${otherGrant.id}`);
    expect(row).toHaveTextContent(otherGrant.reason);
    expect(row).toHaveTextContent(otherGrant.userId);
    expect(row).toHaveTextContent("Pending approval");
  });

  it("approves a grant as the second approver and reports the update", async () => {
    const approved = { ...otherGrant, status: "active" as const, expiresAt: new Date().toISOString() };
    const seen: string[] = [];
    server.use(
      http.post("*/v1/support-grants/*/approve", ({ request }) => {
        seen.push(new URL(request.url).pathname);
        return HttpResponse.json(approved);
      }),
    );
    const onChanged = vi.fn();
    renderWithQuery(
      <GrantApprovalQueue
        grants={[otherGrant]}
        currentUserId="agent-1"
        role="support"
        token="token-1"
        onChanged={onChanged}
      />,
    );

    await userEvent.setup().click(screen.getByTestId(`grant-approval-approve-${otherGrant.id}`));

    await waitFor(() => expect(onChanged).toHaveBeenCalledWith(approved));
    expect(seen[0]).toContain(`/v1/support-grants/${otherGrant.id}/approve`);
  });

  it("disables approval when the current user requested the grant", () => {
    renderWithQuery(
      <GrantApprovalQueue
        grants={[ownGrant]}
        currentUserId="agent-1"
        role="support"
        token="token-1"
        onChanged={vi.fn()}
      />,
    );

    const button = screen.getByTestId(`grant-approval-approve-${ownGrant.id}`);
    expect(button).toBeDisabled();
    expect(screen.getByText(/cannot approve your own request/i)).toBeInTheDocument();
  });

  it("hides revoke from a support user and shows it to an admin", () => {
    const { rerender, client } = renderWithQuery(
      <GrantApprovalQueue
        grants={[otherGrant]}
        currentUserId="agent-1"
        role="support"
        token="token-1"
        onChanged={vi.fn()}
      />,
    );
    expect(screen.queryByTestId(`grant-approval-revoke-${otherGrant.id}`)).not.toBeInTheDocument();

    const withProvider = (children: ReactNode) => (
      <QueryClientProvider client={client}>{children}</QueryClientProvider>
    );
    rerender(
      withProvider(
        <GrantApprovalQueue
          grants={[otherGrant]}
          currentUserId="admin-1"
          role="admin"
          token="token-1"
          onChanged={vi.fn()}
        />,
      ),
    );
    expect(screen.getByTestId(`grant-approval-revoke-${otherGrant.id}`)).toBeInTheDocument();
  });

  it("revokes a grant as an admin and reports the update", async () => {
    const revoked = activeGrant({ id: otherGrant.id, status: "revoked" });
    server.use(
      http.delete("*/v1/support-grants/*", () => HttpResponse.json(revoked)),
    );
    const onChanged = vi.fn();
    renderWithQuery(
      <GrantApprovalQueue
        grants={[otherGrant]}
        currentUserId="admin-1"
        role="admin"
        token="token-1"
        onChanged={onChanged}
      />,
    );

    await userEvent.setup().click(screen.getByTestId(`grant-approval-revoke-${otherGrant.id}`));

    await waitFor(() =>
      expect(onChanged).toHaveBeenCalledWith(expect.objectContaining({ status: "revoked" })),
    );
  });

  it("shows a problem when an action is refused", async () => {
    server.use(
      http.post("*/v1/support-grants/*/approve", () =>
        HttpResponse.json(problemDetails(403, "Second approver required", "corr-403"), {
          status: 403,
          headers: { "Content-Type": "application/problem+json" },
        }),
      ),
    );
    renderWithQuery(
      <GrantApprovalQueue
        grants={[otherGrant]}
        currentUserId="agent-1"
        role="support"
        token="token-1"
        onChanged={vi.fn()}
      />,
    );

    await userEvent.setup().click(screen.getByTestId(`grant-approval-approve-${otherGrant.id}`));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(/second approver required/i);
    expect(alert).toHaveTextContent("corr-403");
  });

  it("has no accessibility violations", async () => {
    const { container } = renderWithQuery(
      <GrantApprovalQueue
        grants={[otherGrant]}
        currentUserId="agent-1"
        role="support"
        token="token-1"
        onChanged={vi.fn()}
      />,
    );
    expect((await axe(container)).violations).toEqual([]);
  });
});
