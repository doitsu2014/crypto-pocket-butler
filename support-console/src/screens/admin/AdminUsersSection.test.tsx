import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { axe } from "jest-axe";
import { describe, expect, it, vi } from "vitest";
import type { AdminUserRecord } from "../../api/types";
import { adminSession, problemDetails, renderWithQuery, supportSession } from "../../test/fixtures";
import { server } from "../../test/server";
import { AdminUsersSection } from "./AdminUsersSection";

function record(overrides: Partial<AdminUserRecord> = {}): AdminUserRecord {
  return {
    id: "user-1",
    displayName: "Test User",
    email: "user@example.com",
    emailVerified: true,
    status: "active",
    role: "admin",
    createdAt: "2026-11-01T10:00:00.000Z",
    updatedAt: "2026-11-02T10:00:00.000Z",
    ...overrides,
  };
}

function serveEmptyGrantQueue() {
  server.use(http.get("*/v1/support-grants", () => HttpResponse.json([])));
}

function renderSection(overrides: { role?: "admin" | "support" } = {}) {
  return renderWithQuery(
    <AdminUsersSection
      token={adminSession.accessToken}
      currentUserId={adminSession.user.id}
      signedInEmail={adminSession.user.email}
      role={overrides.role ?? "admin"}
      onGrantChanged={vi.fn()}
    />,
  );
}

describe("AdminUsersSection", () => {
  it("looks a user up by exact email and renders the record", async () => {
    const user = userEvent.setup();
    serveEmptyGrantQueue();
    let seenBody: unknown;
    server.use(
      http.post("*/v1/admin/users/lookup", async ({ request }) => {
        seenBody = await request.json();
        return HttpResponse.json(record());
      }),
    );
    renderSection();

    await user.type(screen.getByTestId("admin-lookup-email"), "user@example.com");
    await user.click(screen.getByTestId("admin-lookup-submit"));

    expect(await screen.findByTestId("admin-user-record")).toHaveTextContent("Test User");
    expect(seenBody).toEqual({ email: "user@example.com" });
  });

  it("keeps the address out of the request line and shows the no-such-address sentence", async () => {
    const user = userEvent.setup();
    serveEmptyGrantQueue();
    const seenUrls: string[] = [];
    server.use(
      http.post("*/v1/admin/users/lookup", ({ request }) => {
        seenUrls.push(request.url);
        return HttpResponse.json(problemDetails(404, "Not found", "corr-404"), {
          status: 404,
          headers: { "Content-Type": "application/problem+json" },
        });
      }),
    );
    renderSection();

    await user.type(screen.getByTestId("admin-lookup-email"), "missing@example.com");
    await user.click(screen.getByTestId("admin-lookup-submit"));

    expect(await screen.findByTestId("admin-lookup-not-found")).toHaveTextContent(
      "No user has that address.",
    );
    expect(screen.queryByTestId("admin-user-record")).not.toBeInTheDocument();
    expect(seenUrls).toHaveLength(1);
    expect(seenUrls[0]).not.toContain("missing@example.com");
  });

  it("re-reads the record after a role change", async () => {
    const user = userEvent.setup();
    serveEmptyGrantQueue();
    let reads = 0;
    let current = record({ role: "admin" });
    server.use(
      http.put("*/v1/admin/users/user-1/role", async ({ request }) => {
        const body = (await request.json()) as { newRole: string };
        current = record({ role: body.newRole as AdminUserRecord["role"] });
        return HttpResponse.json({
          userId: "user-1",
          previousRole: "admin",
          newRole: body.newRole,
        });
      }),
      http.post("*/v1/admin/users/lookup", () => {
        reads += 1;
        return HttpResponse.json(current);
      }),
    );
    renderSection();

    await user.type(screen.getByTestId("admin-lookup-email"), "user@example.com");
    await user.click(screen.getByTestId("admin-lookup-submit"));
    await user.selectOptions(await screen.findByTestId("admin-role-select"), "support");
    await user.type(screen.getByTestId("admin-role-reason"), "Moved to the support team");
    await user.click(screen.getByTestId("admin-role-submit"));

    await waitFor(() => expect(screen.getByTestId("role-badge")).toHaveTextContent(/support/i));
    expect(screen.getByTestId("admin-role-result")).toHaveTextContent("This change is audited.");
    await waitFor(() => expect(reads).toBeGreaterThanOrEqual(2));
  });

  it("renders nothing at all for a support session", () => {
    const { container } = renderWithQuery(
      <AdminUsersSection
        token={supportSession.accessToken}
        currentUserId={supportSession.user.id}
        role="support"
        onGrantChanged={vi.fn()}
      />,
    );

    expect(container).toBeEmptyDOMElement();
  });

  it("has no accessibility violations with a record open", async () => {
    const user = userEvent.setup();
    serveEmptyGrantQueue();
    server.use(http.post("*/v1/admin/users/lookup", () => HttpResponse.json(record())));
    const { container } = renderSection();

    await user.type(screen.getByTestId("admin-lookup-email"), "user@example.com");
    await user.click(screen.getByTestId("admin-lookup-submit"));
    await screen.findByTestId("admin-user-record");

    expect((await axe(container)).violations).toEqual([]);
  });
});
